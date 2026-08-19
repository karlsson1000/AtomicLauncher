use crate::commands::validation::{sanitize_instance_name, sanitize_mod_filename, sanitize_filename, sanitize_resourcepack_filename, sanitize_shaderpack_filename, validate_download_url};
use crate::utils::{get_instance_dir, open_folder};
use crate::utils::curseforge::{CurseforgeClient, CurseforgeGetModFilesResult, CurseforgeSearchResult, CurseforgeModDetail};
use crate::utils::modrinth::{ModrinthClient, ModrinthProjectDetails, ModrinthSearchResult, ModrinthVersion};
use tauri::Manager;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::time::UNIX_EPOCH;

fn cache_path(instance_dir: &std::path::Path) -> std::path::PathBuf {
    instance_dir.join(".mod_cache.json")
}

fn invalidate_mod_cache(instance_name: &str) {
    let instance_dir = get_instance_dir(instance_name);
    let path = cache_path(&instance_dir);
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
}

#[derive(Serialize, Deserialize)]
pub struct CacheEntry {
    pub mtime: u128,
    pub size: u64,
    pub sha1_hash: String,
    pub metadata: Option<ModFileWithMetadata>,
}

#[derive(Serialize, Deserialize)]
pub struct ModFile {
    pub filename: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModHash {
    pub filename: String,
    pub size: u64,
    pub disabled: bool,
    pub sha1_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModFileWithMetadata {
    pub filename: String,
    pub size: u64,
    pub project_id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub author: Option<String>,
    pub downloads: Option<u64>,
    pub disabled: bool,
    pub current_version_id: Option<String>,
}

#[tauri::command]
pub async fn get_installed_mods(instance_name: String) -> Result<Vec<ModFile>, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    let mods_dir = instance_dir.join("mods");
    
    if !mods_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut mods = Vec::new();
    
    match std::fs::read_dir(&mods_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if !path.starts_with(&mods_dir) {
                    continue;
                }
                
                if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.ends_with(".jar") || filename.ends_with(".jar.disabled") {
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                mods.push(ModFile {
                                    filename: filename.to_string(),
                                    size: metadata.len(),
                                });
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            return Err(e.to_string());
        }
    }
    
    mods.sort_by(|a, b| a.filename.to_lowercase().cmp(&b.filename.to_lowercase()));
    
    Ok(mods)
}

#[tauri::command]
pub async fn delete_mod(instance_name: String, filename: String) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let safe_filename = if filename.ends_with(".disabled") {
        let base = sanitize_mod_filename(filename.trim_end_matches(".disabled"))?;
        format!("{}.disabled", base)
    } else {
        sanitize_mod_filename(&filename)?
    };
    
    let instance_dir = get_instance_dir(&safe_name);
    let mods_dir = instance_dir.join("mods");
    let mod_path = mods_dir.join(&safe_filename);
    
    let canonical_mod_path = mod_path.canonicalize()
        .map_err(|_| format!("Mod file '{}' not found", safe_filename))?;
    
    let canonical_mods_dir = mods_dir.canonicalize()
        .map_err(|_| "Mods directory not found".to_string())?;
    
    if !canonical_mod_path.starts_with(&canonical_mods_dir) {
        return Err("Invalid mod path".to_string());
    }
    
    if !canonical_mod_path.is_file() {
        return Err(format!("Mod file '{}' not found", safe_filename));
    }
    
    std::fs::remove_file(&canonical_mod_path)
        .map_err(|e| e.to_string())?;

    invalidate_mod_cache(&safe_name);

    Ok(())
}

#[tauri::command]
pub fn open_mods_folder(instance_name: String) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    let mods_dir = instance_dir.join("mods");
    
    if !mods_dir.exists() {
        std::fs::create_dir_all(&mods_dir)
            .map_err(|e| e.to_string())?;
    }
    
    open_folder(mods_dir)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_installed_mod_hashes(instance_name: String) -> Result<Vec<ModHash>, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let instance_dir = get_instance_dir(&safe_name);
    let mods_dir = instance_dir.join("mods");

    if !mods_dir.exists() {
        return Ok(Vec::new());
    }

    let mut disk_cache = load_cache(&instance_dir);
    let mut mods = Vec::new();
    let mut changed = false;

    for entry in std::fs::read_dir(&mods_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_file() { continue; }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(f) if f.ends_with(".jar") || f.ends_with(".jar.disabled") => f.to_string(),
            _ => continue,
        };

        let meta = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let size = meta.len();
        let mtime = meta.modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let disabled = filename.ends_with(".disabled");

        let sha1_hash = match disk_cache.get(&filename) {
            Some(entry) if entry.mtime == mtime && entry.size == size => {
                entry.sha1_hash.clone()
            }
            _ => {
                changed = true;
                match std::fs::read(&path) {
                    Ok(bytes) => format!("{:x}", Sha1::digest(&bytes)),
                    Err(_) => continue,
                }
            }
        };

        let existing_metadata = disk_cache.get(&filename).and_then(|e| e.metadata.clone());
        disk_cache.insert(filename.clone(), CacheEntry {
            mtime,
            size,
            sha1_hash: sha1_hash.clone(),
            metadata: existing_metadata,
        });

        mods.push(ModHash {
            filename,
            size,
            disabled,
            sha1_hash,
        });
    }

    let current_filenames: std::collections::HashSet<String> = mods.iter().map(|m| m.filename.clone()).collect();
    disk_cache.retain(|k, _| current_filenames.contains(k));

    if changed {
        save_cache(&instance_dir, &disk_cache);
    }

    mods.sort_by(|a, b| a.filename.to_lowercase().cmp(&b.filename.to_lowercase()));
    Ok(mods)
}

fn load_cache(instance_dir: &std::path::Path) -> HashMap<String, CacheEntry> {
    let cache_file = cache_path(instance_dir);
    if cache_file.exists() {
        std::fs::read_to_string(&cache_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    }
}

fn save_cache(instance_dir: &std::path::Path, cache: &HashMap<String, CacheEntry>) {
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = std::fs::write(cache_path(instance_dir), json);
    }
}

#[tauri::command]
pub async fn get_installed_mods_with_metadata(instance_name: String) -> Result<Vec<ModFileWithMetadata>, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let instance_dir = get_instance_dir(&safe_name);
    let mods_dir = instance_dir.join("mods");

    if !mods_dir.exists() {
        return Ok(Vec::new());
    }

    let cache_file = cache_path(&instance_dir);
    let mut disk_cache: HashMap<String, CacheEntry> = if cache_file.exists() {
        std::fs::read_to_string(&cache_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    let mut mods = Vec::new();
    let mut hashes_needing_metadata: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(&mods_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_file() { continue; }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(f) if f.ends_with(".jar") || f.ends_with(".jar.disabled") => f.to_string(),
            _ => continue,
        };

        let meta = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let size = meta.len();
        let mtime = meta.modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let disabled = filename.ends_with(".disabled");

        let (_, metadata) = match disk_cache.get(&filename) {
            Some(entry) if entry.mtime == mtime && entry.size == size => {
                let hash = entry.sha1_hash.clone();
                let meta = entry.metadata.clone();
                (hash, meta)
            }
            _ => {
                let hash = match std::fs::read(&path) {
                    Ok(bytes) => format!("{:x}", Sha1::digest(&bytes)),
                    Err(_) => continue,
                };
                hashes_needing_metadata.push(hash.clone());
                let existing_meta = disk_cache.get(&filename).and_then(|e| e.metadata.clone());
                disk_cache.insert(filename.clone(), CacheEntry {
                    mtime,
                    size,
                    sha1_hash: hash.clone(),
                    metadata: existing_meta.clone(),
                });
                (hash, existing_meta)
            }
        };

        mods.push(ModFileWithMetadata {
            filename: filename.clone(),
            size,
            project_id: metadata.as_ref().and_then(|m| m.project_id.clone()),
            name: metadata.as_ref().and_then(|m| m.name.clone()),
            description: metadata.as_ref().and_then(|m| m.description.clone()),
            icon_url: metadata.as_ref().and_then(|m| m.icon_url.clone()),
            author: metadata.as_ref().and_then(|m| m.author.clone()),
            downloads: metadata.as_ref().and_then(|m| m.downloads),
            disabled,
            current_version_id: metadata.as_ref().and_then(|m| m.current_version_id.clone()),
        });

    }


    if !hashes_needing_metadata.is_empty() {
        let client = ModrinthClient::new().map_err(|e| e.to_string())?;
        let mut project_ids: Vec<String> = Vec::new();
        let mut hash_to_version_and_project: HashMap<String, (String, String)> = HashMap::new();

        for chunk in hashes_needing_metadata.chunks(100) {
            if let Ok(version_files) = client.get_version_files_by_hashes(chunk).await {
                for (hash, vf) in &version_files {
                    hash_to_version_and_project.insert(hash.clone(), (vf.project_id.clone(), vf.id.clone()));
                    if !project_ids.contains(&vf.project_id) {
                        project_ids.push(vf.project_id.clone());
                    }
                }
            }
        }

        if !project_ids.is_empty() {
            if let Ok(projects) = client.get_projects_batch(&project_ids).await {
                let project_map: HashMap<String, ModrinthProjectDetails> =
                    projects.into_iter().map(|p| (p.id.clone(), p)).collect();

                for (hash, (proj_id, version_id)) in &hash_to_version_and_project {
                    if let Some(project) = project_map.get(proj_id) {
                        let metadata = ModFileWithMetadata {
                            filename: String::new(),
                            size: 0,
                            project_id: Some(proj_id.clone()),
                            name: Some(project.title.clone()),
                            description: Some(project.description.clone()),
                            icon_url: project.icon_url.clone(),
                            author: None,
                            downloads: Some(project.downloads),
                            disabled: false,
                            current_version_id: Some(version_id.clone()),
                        };

                        for entry in disk_cache.values_mut() {
                            if entry.sha1_hash == *hash {
                                entry.metadata = Some(ModFileWithMetadata {
                                    filename: String::new(),
                                    size: entry.size,
                                    ..metadata.clone()
                                });
                            }
                        }

                        for mod_entry in mods.iter_mut() {
                            if disk_cache.get(&mod_entry.filename)
                                .map(|e| e.sha1_hash == *hash)
                                .unwrap_or(false)
                            {
                                mod_entry.project_id = Some(proj_id.clone());
                                mod_entry.name = Some(project.title.clone());
                                mod_entry.description = Some(project.description.clone());
                                mod_entry.icon_url = project.icon_url.clone();
                                mod_entry.author = None;
                                mod_entry.downloads = Some(project.downloads);
                                mod_entry.current_version_id = Some(version_id.clone());
                            }
                        }
                    }
                }
            }
        }

        if let Ok(json) = serde_json::to_string(&disk_cache) {
            let _ = std::fs::write(&cache_file, json);
        }
    }

    mods.sort_by(|a, b| a.filename.to_lowercase().cmp(&b.filename.to_lowercase()));
    Ok(mods)
}

#[tauri::command]
pub async fn toggle_mod(instance_name: String, filename: String, disable: bool) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let safe_filename = if filename.ends_with(".disabled") {
        let base = filename.trim_end_matches(".disabled");
        sanitize_mod_filename(base)?
    } else {
        sanitize_mod_filename(&filename)?
    };
    
    let instance_dir = get_instance_dir(&safe_name);
    let mods_dir = instance_dir.join("mods");
    
    let (old_path, new_filename) = if disable {
        let old = mods_dir.join(&safe_filename);
        let new_name = format!("{}.disabled", safe_filename);
        (old, new_name)
    } else {
        let old = mods_dir.join(format!("{}.disabled", safe_filename));
        (old, safe_filename.clone())
    };
    
    let new_path = mods_dir.join(&new_filename);
    
    let canonical_old = old_path.canonicalize()
        .map_err(|_| "Mod file not found".to_string())?;
    
    let canonical_mods_dir = mods_dir.canonicalize()
        .map_err(|_| "Mods directory not found".to_string())?;
    
    if !canonical_old.starts_with(&canonical_mods_dir) {
        return Err("Invalid mod path".to_string());
    }
    
    std::fs::rename(&old_path, &new_path)
        .map_err(|e| e.to_string())?;

    invalidate_mod_cache(&safe_name);

    Ok(())
}

#[tauri::command]
pub async fn search_mods(
    query: String,
    facets: Option<String>,
    index: Option<String>,
    offset: Option<u32>,
    limit: Option<u32>,
) -> Result<ModrinthSearchResult, String> {
    if query.len() > 200 {
        return Err("Search query too long (max 200 characters)".to_string());
    }
    
    let safe_limit = limit.unwrap_or(20).min(100);
    
    let client = ModrinthClient::new().map_err(|e| e.to_string())?;
    client
        .search_projects(
            &query,
            facets.as_deref(),
            index.as_deref(),
            offset,
            Some(safe_limit),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_mod_details(id_or_slug: String) -> Result<ModrinthProjectDetails, String> {
    if !id_or_slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid mod ID or slug format".to_string());
    }
    
    if id_or_slug.len() > 100 {
        return Err("Mod ID or slug too long".to_string());
    }
    
    let client = ModrinthClient::new().map_err(|e| e.to_string())?;
    client
        .get_project(&id_or_slug)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_project_details(id_or_slug: String) -> Result<ModrinthProjectDetails, String> {
    if !id_or_slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid project ID or slug format".to_string());
    }
    
    if id_or_slug.len() > 100 {
        return Err("Project ID or slug too long".to_string());
    }
    
    let client = ModrinthClient::new().map_err(|e| e.to_string())?;
    client
        .get_project(&id_or_slug)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_mod_versions(
    id_or_slug: String,
    loaders: Option<Vec<String>>,
    game_versions: Option<Vec<String>>,
) -> Result<Vec<ModrinthVersion>, String> {
    if !id_or_slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid mod ID or slug format".to_string());
    }
    
    if id_or_slug.len() > 100 {
        return Err("Mod ID or slug too long".to_string());
    }
    
    if let Some(ref loader_list) = loaders {
        for loader in loader_list {
            if !loader.chars().all(|c| c.is_alphanumeric() || c == '-') {
                return Err("Invalid loader name".to_string());
            }
        }
    }
    
    if let Some(ref version_list) = game_versions {
        for version in version_list {
            if !version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
                return Err("Invalid game version".to_string());
            }
        }
    }
    
    let client = ModrinthClient::new().map_err(|e| e.to_string())?;
    client
        .get_project_versions(&id_or_slug, loaders, game_versions)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn download_mod(
    instance_name: String,
    download_url: String,
    filename: String,
) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let safe_filename = sanitize_mod_filename(&filename)?;
    let _ = validate_download_url(&download_url)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    let mods_dir = instance_dir.join("mods");

    if !mods_dir.exists() {
        std::fs::create_dir_all(&mods_dir)
            .map_err(|e| e.to_string())?;
    }

    let destination = mods_dir.join(&safe_filename);
    
    if !destination.starts_with(&mods_dir) {
        return Err("Invalid destination path".to_string());
    }

    let client = ModrinthClient::new().map_err(|e| e.to_string())?;
    client
        .download_mod_file(&download_url, &destination)
        .await
        .map_err(|e| e.to_string())
}

// CurseForge

pub fn curseforge_api_key(app: &tauri::AppHandle) -> Result<String, String> {
    let config = app.state::<crate::CurseforgeConfig>();
    if config.api_key.is_empty() {
        Err("CURSEFORGE_API_KEY not set in .env".to_string())
    } else {
        Ok(config.api_key.to_string())
    }
}

#[tauri::command]
pub async fn search_curseforge_mods(
    app_handle: tauri::AppHandle,
    query: String,
    class_id: u32,
    category_ids: Option<String>,
    game_version: Option<String>,
    mod_loader_types: Option<String>,
    sort_field: Option<u32>,
    sort_order: Option<String>,
    index: Option<u32>,
    page_size: Option<u32>,
) -> Result<CurseforgeSearchResult, String> {
    if query.len() > 200 {
        return Err("Search query too long (max 200 characters)".to_string());
    }

    let api_key = curseforge_api_key(&app_handle)?;
    let client = CurseforgeClient::new(api_key).map_err(|e| e.to_string())?;
    client
        .search_mods(
            &query,
            class_id,
            category_ids.as_deref(),
            game_version.as_deref(),
            mod_loader_types.as_deref(),
            sort_field.unwrap_or(2),
            sort_order.as_deref(),
            index.unwrap_or(0),
            page_size.unwrap_or(20).min(100),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_curseforge_mod_files(
    app_handle: tauri::AppHandle,
    mod_id: u32,
    game_version: Option<String>,
    mod_loader_type: Option<u32>,
    page_size: Option<u32>,
) -> Result<CurseforgeGetModFilesResult, String> {
    let api_key = curseforge_api_key(&app_handle)?;
    let client = CurseforgeClient::new(api_key).map_err(|e| e.to_string())?;
    client
        .get_mod_files(mod_id, game_version.as_deref(), mod_loader_type, page_size)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn download_curseforge_file_temp(
    app_handle: tauri::AppHandle,
    download_url: String,
    filename: String,
) -> Result<String, String> {
    let safe_filename = sanitize_filename(&filename)?;

    let temp_dir = std::env::temp_dir().join("octane_curseforge");
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let destination = temp_dir.join(&safe_filename);
    let api_key = curseforge_api_key(&app_handle)?;
    let client = CurseforgeClient::new(api_key).map_err(|e| e.to_string())?;
    client
        .download_file(&download_url, &destination)
        .await
        .map_err(|e| e.to_string())?;

    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn download_curseforge_file(
    app_handle: tauri::AppHandle,
    instance_name: String,
    download_url: String,
    filename: String,
    target_folder: String,
) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let safe_filename = match target_folder.as_str() {
        "mods" => sanitize_mod_filename(&filename)?,
        "resourcepacks" => sanitize_resourcepack_filename(&filename)?,
        "shaderpacks" => sanitize_shaderpack_filename(&filename)?,
        _ => sanitize_filename(&filename)?,
    };
    let _ = validate_download_url(&download_url)?;

    let instance_dir = get_instance_dir(&safe_name);
    let target_dir = instance_dir.join(&target_folder);

    if !target_dir.exists() {
        std::fs::create_dir_all(&target_dir)
            .map_err(|e| e.to_string())?;
    }

    let destination = target_dir.join(&safe_filename);

    if !destination.starts_with(&target_dir) {
        return Err("Invalid destination path".to_string());
    }

    let api_key = curseforge_api_key(&app_handle)?;
    let client = CurseforgeClient::new(api_key).map_err(|e| e.to_string())?;
    client
        .download_file(&download_url, &destination)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_curseforge_mod_details(
    app_handle: tauri::AppHandle,
    mod_id: u32,
) -> Result<CurseforgeModDetail, String> {
    let api_key = curseforge_api_key(&app_handle)?;
    let client = CurseforgeClient::new(api_key).map_err(|e| e.to_string())?;
    client.get_mod(mod_id).await
}