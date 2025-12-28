use crate::commands::validation::{sanitize_instance_name, sanitize_filename, validate_download_url};
use crate::utils::{get_instance_dir, open_folder};
use crate::utils::modrinth::{ModrinthClient, ModrinthProjectDetails, ModrinthSearchResult, ModrinthVersion};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ModFile {
    pub filename: String,
    pub size: u64,
}

// ===== MOD FILE MANAGEMENT =====

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
            return Err(format!("Failed to read mods directory: {}", e));
        }
    }
    
    mods.sort_by(|a, b| a.filename.to_lowercase().cmp(&b.filename.to_lowercase()));
    
    Ok(mods)
}

#[tauri::command]
pub async fn delete_mod(instance_name: String, filename: String) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let safe_filename = sanitize_filename(&filename)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    let mods_dir = instance_dir.join("mods");
    let mod_path = mods_dir.join(&safe_filename);
    
    let canonical_mod_path = mod_path.canonicalize()
        .map_err(|_| format!("Mod file '{}' not found", safe_filename))?;
    
    let canonical_mods_dir = mods_dir.canonicalize()
        .map_err(|_| "Mods directory not found".to_string())?;
    
    if !canonical_mod_path.starts_with(&canonical_mods_dir) {
        return Err("Invalid mod path (path traversal detected)".to_string());
    }
    
    if !canonical_mod_path.is_file() {
        return Err(format!("Mod file '{}' not found or is not a file", safe_filename));
    }
    
    std::fs::remove_file(&canonical_mod_path)
        .map_err(|e| format!("Failed to delete mod: {}", e))?;
    
    Ok(format!("Successfully deleted {}", safe_filename))
}

#[tauri::command]
pub fn open_mods_folder(instance_name: String) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    let mods_dir = instance_dir.join("mods");
    
    if !mods_dir.exists() {
        std::fs::create_dir_all(&mods_dir)
            .map_err(|e| format!("Failed to create mods directory: {}", e))?;
    }
    
    open_folder(mods_dir)
        .map_err(|e| format!("Failed to open mods folder: {}", e))?;
    
    Ok(format!("Opened mods folder for instance '{}'", safe_name))
}

#[tauri::command]
pub async fn toggle_mod(instance_name: String, filename: String, disable: bool) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let safe_filename = if filename.ends_with(".disabled") {
        let base = filename.trim_end_matches(".disabled");
        sanitize_filename(base)?
    } else {
        sanitize_filename(&filename)?
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
        return Err("Invalid mod path (path traversal detected)".to_string());
    }
    
    std::fs::rename(&old_path, &new_path)
        .map_err(|e| format!("Failed to toggle mod: {}", e))?;
    
    let status = if disable { "disabled" } else { "enabled" };
    Ok(format!("Successfully {} mod", status))
}

// ===== MODRINTH API =====

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
    
    let client = ModrinthClient::new();
    client
        .search_projects(
            &query,
            facets.as_deref(),
            index.as_deref(),
            offset,
            Some(safe_limit),
        )
        .await
        .map_err(|e| format!("Failed to search mods: {}", e))
}

#[tauri::command]
pub async fn get_mod_details(id_or_slug: String) -> Result<ModrinthProjectDetails, String> {
    if !id_or_slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid mod ID or slug format".to_string());
    }
    
    if id_or_slug.len() > 100 {
        return Err("Mod ID or slug too long".to_string());
    }
    
    let client = ModrinthClient::new();
    client
        .get_project(&id_or_slug)
        .await
        .map_err(|e| format!("Failed to get mod details: {}", e))
}

#[tauri::command]
pub async fn get_project_details(id_or_slug: String) -> Result<ModrinthProjectDetails, String> {
    if !id_or_slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid project ID or slug format".to_string());
    }
    
    if id_or_slug.len() > 100 {
        return Err("Project ID or slug too long".to_string());
    }
    
    let client = ModrinthClient::new();
    client
        .get_project(&id_or_slug)
        .await
        .map_err(|e| format!("Failed to get project details: {}", e))
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
    
    let client = ModrinthClient::new();
    client
        .get_project_versions(&id_or_slug, loaders, game_versions)
        .await
        .map_err(|e| format!("Failed to get mod versions: {}", e))
}

#[tauri::command]
pub async fn download_mod(
    instance_name: String,
    download_url: String,
    filename: String,
) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let safe_filename = sanitize_filename(&filename)?;
    
    validate_download_url(&download_url)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    let mods_dir = instance_dir.join("mods");

    if !mods_dir.exists() {
        std::fs::create_dir_all(&mods_dir)
            .map_err(|e| format!("Failed to create mods directory: {}", e))?;
    }

    let destination = mods_dir.join(&safe_filename);
    
    if !destination.starts_with(&mods_dir) {
        return Err("Invalid destination path".to_string());
    }

    let client = ModrinthClient::new();
    client
        .download_mod_file(&download_url, &destination)
        .await
        .map_err(|e| format!("Failed to download mod: {}", e))?;

    Ok(format!("Successfully downloaded {}", safe_filename))
}