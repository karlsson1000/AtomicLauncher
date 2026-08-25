use crate::commands::mods::ModFileWithMetadata;
use crate::commands::validation::{sanitize_instance_name, sanitize_resourcepack_filename, sanitize_shaderpack_filename};
use crate::utils::{get_instance_dir, open_folder};

// Resource Packs

#[tauri::command]
pub async fn get_installed_resourcepacks(instance_name: String) -> Result<Vec<String>, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    let resourcepacks_dir = instance_dir.join("resourcepacks");
    
    if !resourcepacks_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut packs = Vec::new();
    
    match std::fs::read_dir(&resourcepacks_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if !path.starts_with(&resourcepacks_dir) {
                    continue;
                }
                
                if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        // Accept both .zip and .jar files
                        if filename.ends_with(".zip") || filename.ends_with(".jar") {
                            packs.push(filename.to_string());
                        }
                    }
                }
            }
        }
        Err(e) => {
            return Err(e.to_string());
        }
    }
    
    packs.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    
    Ok(packs)
}

#[tauri::command]
pub async fn download_resourcepack(
    instance_name: String,
    download_url: String,
    filename: String,
    expected_sha1: Option<String>,
) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let safe_filename = sanitize_resourcepack_filename(&filename)?;

    crate::commands::mods::download_into_content_dir(
        &safe_name,
        "resourcepacks",
        &download_url,
        &safe_filename,
        expected_sha1.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn delete_resourcepack(instance_name: String, filename: String) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let safe_filename = sanitize_resourcepack_filename(&filename)?;

    crate::commands::mods::delete_content_file(&safe_name, "resourcepacks", "resourcepack", &safe_filename, "Resource pack")
}

#[tauri::command]
pub fn open_resourcepacks_folder(instance_name: String) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    let resourcepacks_dir = instance_dir.join("resourcepacks");
    
    if !resourcepacks_dir.exists() {
        std::fs::create_dir_all(&resourcepacks_dir)
            .map_err(|e| e.to_string())?;
    }
    
    open_folder(resourcepacks_dir)
        .map_err(|e| e.to_string())
}

// Shader Packs

#[tauri::command]
pub async fn get_installed_shaderpacks(instance_name: String) -> Result<Vec<String>, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    let shaderpacks_dir = instance_dir.join("shaderpacks");
    
    if !shaderpacks_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut packs = Vec::new();
    
    match std::fs::read_dir(&shaderpacks_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if !path.starts_with(&shaderpacks_dir) {
                    continue;
                }
                
                if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        // Accept both .zip and .jar files
                        if filename.ends_with(".zip") || filename.ends_with(".jar") {
                            packs.push(filename.to_string());
                        }
                    }
                }
            }
        }
        Err(e) => {
            return Err(e.to_string());
        }
    }
    
    packs.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    
    Ok(packs)
}

#[tauri::command]
pub async fn download_shaderpack(
    instance_name: String,
    download_url: String,
    filename: String,
    expected_sha1: Option<String>,
) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let safe_filename = sanitize_shaderpack_filename(&filename)?;

    crate::commands::mods::download_into_content_dir(
        &safe_name,
        "shaderpacks",
        &download_url,
        &safe_filename,
        expected_sha1.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn delete_shaderpack(instance_name: String, filename: String) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let safe_filename = sanitize_shaderpack_filename(&filename)?;

    crate::commands::mods::delete_content_file(&safe_name, "shaderpacks", "shaderpack", &safe_filename, "Shader pack")
}

#[tauri::command]
pub fn open_shaderpacks_folder(instance_name: String) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    let shaderpacks_dir = instance_dir.join("shaderpacks");
    
    if !shaderpacks_dir.exists() {
        std::fs::create_dir_all(&shaderpacks_dir)
            .map_err(|e| e.to_string())?;
    }
    
    open_folder(shaderpacks_dir)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_installed_resourcepacks_with_metadata(
    instance_name: String,
) -> Result<Vec<ModFileWithMetadata>, String> {
    crate::commands::mods::get_installed_content_with_metadata(
        instance_name,
        "resourcepacks",
        "resourcepack",
        &[".zip", ".jar"],
        false,
    )
    .await
}

#[tauri::command]
pub async fn get_installed_shaderpacks_with_metadata(
    instance_name: String,
) -> Result<Vec<ModFileWithMetadata>, String> {
    crate::commands::mods::get_installed_content_with_metadata(
        instance_name,
        "shaderpacks",
        "shaderpack",
        &[".zip", ".jar"],
        false,
    )
    .await
}