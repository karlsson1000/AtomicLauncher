use crate::commands::validation::{sanitize_instance_name, sanitize_mod_filename, sanitize_filename, sanitize_resourcepack_filename, sanitize_shaderpack_filename, validate_download_url};
use crate::utils::get_instance_dir;
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

pub(crate) fn invalidate_content_cache(instance_name: &str, cache_kind: &str) {
    let instance_dir = get_instance_dir(instance_name);
    let path = instance_dir.join(format!(".{}_cache.json", cache_kind));
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
}

fn invalidate_mod_cache(instance_name: &str) {
    invalidate_content_cache(instance_name, "mod");
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
pub(crate) fn delete_content_file(
    safe_name: &str,
    folder: &str,
    cache_kind: &str,
    safe_filename: &str,
    label: &str,
) -> Result<(), String> {
    let instance_dir = get_instance_dir(safe_name);
    let content_dir = instance_dir.join(folder);
    let file_path = content_dir.join(safe_filename);

    let canonical_file_path = file_path
        .canonicalize()
        .map_err(|_| format!("{} '{}' not found", label, safe_filename))?;

    let canonical_content_dir = content_dir
        .canonicalize()
        .map_err(|_| format!("{} directory not found", label))?;

    if !canonical_file_path.starts_with(&canonical_content_dir) {
        return Err(format!("Invalid {} path", label.to_lowercase()));
    }

    if !canonical_file_path.is_file() {
        return Err(format!("{} '{}' not found", label, safe_filename));
    }

    std::fs::remove_file(&canonical_file_path)
        .map_err(|e| e.to_string())?;

    invalidate_content_cache(safe_name, cache_kind);

    Ok(())
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

    delete_content_file(&safe_name, "mods", "mod", &safe_filename, "Mod file")
}

fn collect_mod_hashes(instance_dir: &std::path::Path) -> Result<Vec<ModHash>, String> {
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
    get_installed_content_with_metadata(instance_name, "mods", "mod", &[".jar", ".jar.disabled"], true).await
}

pub(crate) async fn get_installed_content_with_metadata(
    instance_name: String,
    folder: &str,
    cache_kind: &str,
    allowed_extensions: &[&str],
    supports_disabled: bool,
) -> Result<Vec<ModFileWithMetadata>, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let instance_dir = get_instance_dir(&safe_name);
    let content_dir = instance_dir.join(folder);

    if !content_dir.exists() {
        return Ok(Vec::new());
    }

    let cache_file = instance_dir.join(format!(".{}_cache.json", cache_kind));
    let mut disk_cache: HashMap<String, CacheEntry> = if cache_file.exists() {
        std::fs::read_to_string(&cache_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    let mut items = Vec::new();
    let mut hashes_needing_metadata: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(&content_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_file() { continue; }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(f) if allowed_extensions.iter().any(|ext| f.ends_with(ext)) => f.to_string(),
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
        let disabled = supports_disabled && filename.ends_with(".disabled");

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

        items.push(ModFileWithMetadata {
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
        match enrich_with_modrinth_metadata(&mut disk_cache, &mut items, &mut hashes_needing_metadata).await {
            Ok(()) => {
                if let Ok(json) = serde_json::to_string(&disk_cache) {
                    let _ = std::fs::write(&cache_file, json);
                }
            }
            Err(e) => eprintln!("Failed to fetch Modrinth metadata for {} items: {}", hashes_needing_metadata.len(), e),
        }
    }

    items.sort_by(|a, b| a.filename.to_lowercase().cmp(&b.filename.to_lowercase()));
    Ok(items)
}

async fn enrich_with_modrinth_metadata(
    disk_cache: &mut HashMap<String, CacheEntry>,
    items: &mut [ModFileWithMetadata],
    hashes_needing_metadata: &mut [String],
) -> Result<(), String> {
    let client = ModrinthClient::new().map_err(|e| e.to_string())?;
    let mut project_ids: Vec<String> = Vec::new();
    let mut hash_to_version_and_project: HashMap<String, (String, String)> = HashMap::new();

    for chunk in hashes_needing_metadata.chunks(100) {
        let version_files = client
            .get_version_files_by_hashes(chunk)
            .await
            .map_err(|e| e.to_string())?;
        for (hash, vf) in &version_files {
            hash_to_version_and_project.insert(hash.clone(), (vf.project_id.clone(), vf.id.clone()));
            if !project_ids.contains(&vf.project_id) {
                project_ids.push(vf.project_id.clone());
            }
        }
    }

    if project_ids.is_empty() {
        return Ok(());
    }

    let projects = client
        .get_projects_batch(&project_ids)
        .await
        .map_err(|e| e.to_string())?;
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

            for item in items.iter_mut() {
                if disk_cache.get(&item.filename)
                    .map(|e| e.sha1_hash == *hash)
                    .unwrap_or(false)
                {
                    item.project_id = Some(proj_id.clone());
                    item.name = Some(project.title.clone());
                    item.description = Some(project.description.clone());
                    item.icon_url = project.icon_url.clone();
                    item.author = None;
                    item.downloads = Some(project.downloads);
                    item.current_version_id = Some(version_id.clone());
                }
            }
        }
    }

    Ok(())
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

#[derive(Debug, Serialize)]
pub struct ModUpdate {
    pub filename: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "latestVersion")]
    pub latest_version: LatestVersionInfo,
}

#[derive(Debug, Serialize)]
pub struct LatestVersionInfo {
    pub id: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: String,
    pub filename: String,
    #[serde(rename = "sha1")]
    pub sha1: String,
}

#[tauri::command]
pub async fn check_mod_updates(
    instance_name: String,
    loader: String,
    game_version: String,
) -> Result<Vec<ModUpdate>, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;

    if !loader.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err("Invalid loader name".to_string());
    }
    if !game_version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid game version".to_string());
    }

    let instance_dir = get_instance_dir(&safe_name);
    let installed = tauri::async_runtime::spawn_blocking(move || collect_mod_hashes(&instance_dir))
        .await
        .map_err(|e| e.to_string())??;

    let eligible: Vec<ModHash> = installed.into_iter().filter(|m| !m.disabled).collect();
    if eligible.is_empty() {
        return Ok(Vec::new());
    }

    let hashes: Vec<String> = eligible.iter().map(|m| m.sha1_hash.clone()).collect();
    let client = ModrinthClient::new().map_err(|e| e.to_string())?;
    let latest = client
        .get_latest_version_files_by_hashes(&hashes, Some(vec![game_version]), Some(vec![loader]))
        .await
        .map_err(|e| e.to_string())?;

    let mut updates = Vec::new();
    for mh in &eligible {
        let Some(resp) = latest.get(&mh.sha1_hash) else { continue };
        if resp.files.iter().any(|f| f.hashes.sha1 == mh.sha1_hash) {
            continue;
        }
        let Some(file) = resp.files.iter().find(|f| f.primary).or_else(|| resp.files.first()) else { continue };
        updates.push(ModUpdate {
            filename: mh.filename.clone(),
            project_id: resp.project_id.clone(),
            latest_version: LatestVersionInfo {
                id: resp.id.clone(),
                download_url: file.url.clone(),
                filename: file.filename.clone(),
                sha1: file.hashes.sha1.clone(),
            },
        });
    }

    Ok(updates)
}

pub(crate) async fn download_into_content_dir(
    safe_name: &str,
    folder: &str,
    download_url: &str,
    safe_filename: &str,
    expected_sha1: Option<&str>,
) -> Result<(), String> {
    let _ = validate_download_url(download_url)?;

    let instance_dir = get_instance_dir(safe_name);
    let content_dir = instance_dir.join(folder);

    if !content_dir.exists() {
        std::fs::create_dir_all(&content_dir)
            .map_err(|e| e.to_string())?;
    }

    let destination = content_dir.join(safe_filename);

    if !destination.starts_with(&content_dir) {
        return Err("Invalid destination path".to_string());
    }

    let client = ModrinthClient::new().map_err(|e| e.to_string())?;
    client
        .download_mod_file(download_url, &destination, expected_sha1)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn download_mod(
    instance_name: String,
    download_url: String,
    filename: String,
    expected_sha1: Option<String>,
) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let safe_filename = sanitize_mod_filename(&filename)?;

    download_into_content_dir(&safe_name, "mods", &download_url, &safe_filename, expected_sha1.as_deref()).await
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
    download_url: String,
    filename: String,
    expected_sha1: Option<String>,
) -> Result<String, String> {
    let safe_filename = sanitize_filename(&filename)?;
    let _ = validate_download_url(&download_url)?;

    let temp_dir = std::env::temp_dir().join("octane_curseforge");
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let destination = temp_dir.join(&safe_filename);
    crate::utils::download::download_file_verified(
        &download_url,
        &destination,
        expected_sha1.as_deref(),
    )
    .await?;

    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn download_curseforge_file(
    instance_name: String,
    download_url: String,
    filename: String,
    target_folder: String,
    expected_sha1: Option<String>,
) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let safe_filename = match target_folder.as_str() {
        "mods" => sanitize_mod_filename(&filename)?,
        "resourcepacks" => sanitize_resourcepack_filename(&filename)?,
        "shaderpacks" => sanitize_shaderpack_filename(&filename)?,
        _ => return Err("Invalid target folder".to_string()),
    };

    crate::commands::mods::download_into_content_dir(
        &safe_name,
        &target_folder,
        &download_url,
        &safe_filename,
        expected_sha1.as_deref(),
    )
    .await?;

    Ok(())
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