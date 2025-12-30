use crate::services::instance::InstanceManager;
use crate::services::installer::MinecraftInstaller;
use crate::services::fabric::FabricInstaller;
use crate::utils::modrinth::{ModrinthClient, ModrinthVersion};
use crate::utils::*;
use crate::commands::validation::{sanitize_instance_name, validate_download_url};
use tauri::Emitter;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ModpackVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub files: Vec<ModpackFile>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ModpackFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
}

#[tauri::command]
pub async fn get_modpack_versions(
    id_or_slug: String,
    game_version: Option<String>,
) -> Result<Vec<ModrinthVersion>, String> {
    if !id_or_slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid modpack ID or slug format".to_string());
    }
    
    if id_or_slug.len() > 100 {
        return Err("Modpack ID or slug too long".to_string());
    }
    
    if let Some(ref version) = game_version {
        if !version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
            return Err("Invalid game version format".to_string());
        }
    }
    
    let client = ModrinthClient::new();
    client
        .get_project_versions(
            &id_or_slug,
            None,
            game_version.map(|v| vec![v]),
        )
        .await
        .map_err(|e| format!("Failed to get modpack versions: {}", e))
}

#[tauri::command]
pub async fn install_modpack(
    modpack_slug: String,
    instance_name: String,
    version_id: String,
    preferred_game_version: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    if !modpack_slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid modpack slug format".to_string());
    }
    
    if !version_id.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err("Invalid version ID format".to_string());
    }
    
    if let Some(ref version) = preferred_game_version {
        if !version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
            return Err("Invalid preferred game version format".to_string());
        }
    }

    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 0,
        "stage": "Starting modpack installation..."
    }));
    
    println!("Installing modpack: {}", modpack_slug);
    
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 5,
        "stage": "Fetching modpack information..."
    }));
    
    let client = ModrinthClient::new();
    let versions = client
        .get_project_versions(&modpack_slug, None, None)
        .await
        .map_err(|e| format!("Failed to fetch modpack versions: {}", e))?;
    
    let version = versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| "Version not found".to_string())?;
    
    let game_version = if let Some(ref preferred) = preferred_game_version {
        if version.game_versions.contains(preferred) {
            preferred.clone()
        } else {
            version.game_versions.first()
                .ok_or_else(|| "No game version found".to_string())?
                .clone()
        }
    } else {
        version.game_versions.first()
            .ok_or_else(|| "No game version found".to_string())?
            .clone()
    };
    
    let loader = version.loaders.first()
        .map(|l| l.to_lowercase())
        .unwrap_or_else(|| "vanilla".to_string());
    
    println!("Game version: {}, Loader: {}", game_version, loader);
    
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 10,
        "stage": format!("Installing Minecraft {}...", game_version)
    }));
    
    let meta_dir = get_meta_dir();
    let installer = MinecraftInstaller::new(meta_dir.clone());
    installer
        .install_version(&game_version)
        .await
        .map_err(|e| format!("Failed to install Minecraft: {}", e))?;
    
    let final_version = if loader == "fabric" {
        let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
            "instance": safe_name,
            "progress": 20,
            "stage": "Installing Fabric loader..."
        }));
        
        let fabric_installer = FabricInstaller::new(meta_dir);
        
        let fabric_versions = fabric_installer
            .get_loader_versions()
            .await
            .map_err(|e| format!("Failed to get Fabric versions: {}", e))?;
        
        let fabric_version = fabric_versions
            .iter()
            .find(|v| v.stable)
            .or_else(|| fabric_versions.first())
            .ok_or_else(|| "No Fabric versions found".to_string())?;
        
        fabric_installer
            .install_fabric(&game_version, &fabric_version.version)
            .await
            .map_err(|e| format!("Failed to install Fabric: {}", e))?
    } else {
        game_version.clone()
    };
    
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 30,
        "stage": "Creating instance..."
    }));
    
    InstanceManager::create(
        &safe_name,
        &final_version,
        if loader == "vanilla" { None } else { Some(loader.clone()) },
        None,
    )
    .map_err(|e| format!("Failed to create instance: {}", e))?;
    
    let instance_dir = get_instance_dir(&safe_name);
    let mods_dir = instance_dir.join("mods");
    
    std::fs::create_dir_all(&mods_dir)
        .map_err(|e| format!("Failed to create mods directory: {}", e))?;
    
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 40,
        "stage": "Downloading modpack..."
    }));
    
    let primary_file = version.files.iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| "No modpack file found".to_string())?;
    
    let temp_dir = std::env::temp_dir();
    let modpack_file = temp_dir.join(&primary_file.filename);
    
    validate_download_url(&primary_file.url)?;
    
    client
        .download_mod_file(&primary_file.url, &modpack_file)
        .await
        .map_err(|e| format!("Failed to download modpack: {}", e))?;
    
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 50,
        "stage": "Extracting modpack..."
    }));
    
    let extract_dir = temp_dir.join(format!("modpack_extract_{}", safe_name));
    if extract_dir.exists() {
        let _ = std::fs::remove_dir_all(&extract_dir);
    }
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Failed to create extraction directory: {}", e))?;
    
    extract_modpack(&modpack_file, &extract_dir)
        .map_err(|e| format!("Failed to extract modpack: {}", e))?;
    
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 60,
        "stage": "Reading modpack manifest..."
    }));
    
    let manifest_path = extract_dir.join("modrinth.index.json");
    if !manifest_path.exists() {
        return Err("Invalid modpack: modrinth.index.json not found".to_string());
    }
    
    let manifest_content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content)
        .map_err(|e| format!("Failed to parse manifest: {}", e))?;
    
    let overrides_dir = extract_dir.join("overrides");
    if overrides_dir.exists() {
        let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
            "instance": safe_name,
            "progress": 65,
            "stage": "Copying overrides..."
        }));
        
        copy_dir_recursive(&overrides_dir, &instance_dir)
            .map_err(|e| format!("Failed to copy overrides: {}", e))?;
    }
    
    if let Some(files) = manifest.get("files").and_then(|f| f.as_array()) {
        let total_files = files.len();
        let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
            "instance": safe_name,
            "progress": 70,
            "stage": format!("Downloading {} mods...", total_files)
        }));
        
        for (idx, file) in files.iter().enumerate() {
            let downloads = file.get("downloads")
                .and_then(|d| d.as_array())
                .ok_or_else(|| "Invalid file entry in manifest".to_string())?;
            
            let download_url = downloads.first()
                .and_then(|u| u.as_str())
                .ok_or_else(|| "No download URL found".to_string())?;
            
            let path = file.get("path")
                .and_then(|p| p.as_str())
                .ok_or_else(|| "No path found in file entry".to_string())?;
            
            let dest_path = instance_dir.join(path);
            
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            }
            
            validate_download_url(download_url)?;
            client.download_mod_file(download_url, &dest_path)
                .await
                .map_err(|e| format!("Failed to download mod: {}", e))?;
            
            let progress = 70 + ((idx + 1) * 25 / total_files) as u32;
            let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
                "instance": safe_name,
                "progress": progress,
                "stage": format!("Downloading mods... ({}/{})", idx + 1, total_files)
            }));
        }
    }
    
    let _ = std::fs::remove_file(&modpack_file);
    let _ = std::fs::remove_dir_all(&extract_dir);

    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 100,
        "stage": "Installation complete!"
    }));
    
    Ok(format!("Successfully installed modpack '{}'", safe_name))
}

fn copy_dir_recursive(
    src: &std::path::Path,
    dst: &std::path::Path,
) -> std::io::Result<()> {
    use std::fs;
    
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}

fn extract_modpack(
    archive_path: &std::path::Path,
    dest_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use zip::ZipArchive;
    
    let file = std::fs::File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };
        
        if !outpath.starts_with(dest_dir) {
            continue;
        }
        
        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)?;
                }
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }
    
    Ok(())
}

#[tauri::command]
pub async fn get_modpack_manifest(
    modpack_slug: String,
    version_id: String,
) -> Result<serde_json::Value, String> {
    if !modpack_slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid modpack slug format".to_string());
    }
    
    if !version_id.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err("Invalid version ID format".to_string());
    }
    
    let client = ModrinthClient::new();
    
    let versions = client
        .get_project_versions(&modpack_slug, None, None)
        .await
        .map_err(|e| format!("Failed to fetch versions: {}", e))?;
    
    let version = versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| "Version not found".to_string())?;
    
    Ok(serde_json::json!({
        "name": version.name,
        "version_number": version.version_number,
        "game_versions": version.game_versions,
        "loaders": version.loaders,
        "files": version.files.iter().map(|f| serde_json::json!({
            "filename": f.filename,
            "size": f.size,
            "primary": f.primary
        })).collect::<Vec<_>>()
    }))
}

#[tauri::command]
pub async fn get_modpack_game_versions() -> Result<Vec<String>, String> {
    let version_list = vec![
        "1.21.11", "1.21.10", "1.21.9", "1.21.8", "1.21.7", "1.21.6", "1.21.5", "1.21.4", "1.21.3", "1.21.2", "1.21.1", "1.21",
        "1.20.6", "1.20.5", "1.20.4", "1.20.3", "1.20.2", "1.20.1", "1.20",
        "1.19.4", "1.19.3", "1.19.2", "1.19.1", "1.19",
        "1.18.2", "1.18.1", "1.18",
        "1.17.1", "1.17",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    
    Ok(version_list)
}

#[tauri::command]
pub async fn get_modpack_name_from_file(
    file_path: String,
) -> Result<String, String> {
    use std::path::Path;
    
    let file_path_obj = Path::new(&file_path);
    if !file_path_obj.exists() {
        return Err("Modpack file does not exist".to_string());
    }
    
    let extension = file_path_obj
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| "Invalid file extension".to_string())?;
    
    if extension != "mrpack" && extension != "zip" {
        return Err("Invalid modpack file format. Expected .mrpack or .zip".to_string());
    }
    
    let temp_dir = std::env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let extract_dir = temp_dir.join(format!("modpack_preview_{}", timestamp));
    
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Failed to create extraction directory: {}", e))?;
    
    let extract_result = extract_modpack(file_path_obj, &extract_dir);
    if let Err(e) = extract_result {
        let _ = std::fs::remove_dir_all(&extract_dir);
        return Err(format!("Failed to extract modpack: {}", e));
    }
    
    let manifest_path = extract_dir.join("modrinth.index.json");
    if !manifest_path.exists() {
        let _ = std::fs::remove_dir_all(&extract_dir);
        return Err("Invalid modpack: modrinth.index.json not found".to_string());
    }
    
    let manifest_content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| {
            let _ = std::fs::remove_dir_all(&extract_dir);
            format!("Failed to read manifest: {}", e)
        })?;
    
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content)
        .map_err(|e| {
            let _ = std::fs::remove_dir_all(&extract_dir);
            format!("Failed to parse manifest: {}", e)
        })?;
    
    let modpack_name = manifest.get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("Imported Modpack")
        .to_string();
    
    let _ = std::fs::remove_dir_all(&extract_dir);
    
    Ok(modpack_name)
}

#[tauri::command]
pub async fn install_modpack_from_file(
    file_path: String,
    instance_name: String,
    preferred_game_version: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    use std::path::Path;
    
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let file_path_obj = Path::new(&file_path);
    if !file_path_obj.exists() {
        return Err("Modpack file does not exist".to_string());
    }
    
    let extension = file_path_obj
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| "Invalid file extension".to_string())?;
    
    if extension != "mrpack" && extension != "zip" {
        return Err("Invalid modpack file format. Expected .mrpack or .zip".to_string());
    }
    
    if let Some(ref version) = preferred_game_version {
        if !version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
            return Err("Invalid preferred game version format".to_string());
        }
    }

    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 0,
        "stage": "Starting modpack installation..."
    }));
    
    println!("Installing modpack from file: {}", file_path);
    
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 5,
        "stage": "Reading modpack file..."
    }));
    
    let temp_dir = std::env::temp_dir();
    let extract_dir = temp_dir.join(format!("modpack_extract_{}", safe_name));
    if extract_dir.exists() {
        let _ = std::fs::remove_dir_all(&extract_dir);
    }
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Failed to create extraction directory: {}", e))?;
    
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 10,
        "stage": "Extracting modpack..."
    }));
    
    extract_modpack(file_path_obj, &extract_dir)
        .map_err(|e| format!("Failed to extract modpack: {}", e))?;
    
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 20,
        "stage": "Reading modpack manifest..."
    }));
    
    let manifest_path = extract_dir.join("modrinth.index.json");
    if !manifest_path.exists() {
        return Err("Invalid modpack: modrinth.index.json not found".to_string());
    }
    
    let manifest_content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    
    let manifest: serde_json::Value = serde_json::from_str(&manifest_content)
        .map_err(|e| format!("Failed to parse manifest: {}", e))?;
    
    let dependencies = manifest.get("dependencies")
        .and_then(|d| d.as_object())
        .ok_or_else(|| "Invalid manifest: missing dependencies".to_string())?;
    
    let game_version = if let Some(ref preferred) = preferred_game_version {
        preferred.clone()
    } else {
        dependencies.get("minecraft")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "No Minecraft version found in manifest".to_string())?
            .to_string()
    };
    
    let loader = if dependencies.contains_key("fabric-loader") {
        "fabric"
    } else if dependencies.contains_key("forge") {
        "forge"
    } else if dependencies.contains_key("quilt-loader") {
        "quilt"
    } else {
        "vanilla"
    };
    
    println!("Game version: {}, Loader: {}", game_version, loader);
    
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 30,
        "stage": format!("Installing Minecraft {}...", game_version)
    }));
    
    let meta_dir = get_meta_dir();
    let installer = MinecraftInstaller::new(meta_dir.clone());
    installer
        .install_version(&game_version)
        .await
        .map_err(|e| format!("Failed to install Minecraft: {}", e))?;
    
    let final_version = if loader == "fabric" {
        let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
            "instance": safe_name,
            "progress": 40,
            "stage": "Installing Fabric loader..."
        }));
        
        let fabric_installer = FabricInstaller::new(meta_dir);
        
        let fabric_versions = fabric_installer
            .get_loader_versions()
            .await
            .map_err(|e| format!("Failed to get Fabric versions: {}", e))?;
        
        let fabric_version = fabric_versions
            .iter()
            .find(|v| v.stable)
            .or_else(|| fabric_versions.first())
            .ok_or_else(|| "No Fabric versions found".to_string())?;
        
        fabric_installer
            .install_fabric(&game_version, &fabric_version.version)
            .await
            .map_err(|e| format!("Failed to install Fabric: {}", e))?
    } else {
        game_version.clone()
    };
    
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 50,
        "stage": "Creating instance..."
    }));
    
    InstanceManager::create(
        &safe_name,
        &final_version,
        if loader == "vanilla" { None } else { Some(loader.to_string()) },
        None,
    )
    .map_err(|e| format!("Failed to create instance: {}", e))?;
    
    let instance_dir = get_instance_dir(&safe_name);
    
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 60,
        "stage": "Copying overrides..."
    }));
    
    let overrides_dir = extract_dir.join("overrides");
    if overrides_dir.exists() {
        copy_dir_recursive(&overrides_dir, &instance_dir)
            .map_err(|e| format!("Failed to copy overrides: {}", e))?;
    }
    
    if let Some(files) = manifest.get("files").and_then(|f| f.as_array()) {
        let total_files = files.len();
        let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
            "instance": safe_name,
            "progress": 70,
            "stage": format!("Downloading {} mods...", total_files)
        }));
        
        let client = crate::utils::modrinth::ModrinthClient::new();
        
        for (idx, file) in files.iter().enumerate() {
            let downloads = file.get("downloads")
                .and_then(|d| d.as_array())
                .ok_or_else(|| "Invalid file entry in manifest".to_string())?;
            
            let download_url = downloads.first()
                .and_then(|u| u.as_str())
                .ok_or_else(|| "No download URL found".to_string())?;
            
            let path = file.get("path")
                .and_then(|p| p.as_str())
                .ok_or_else(|| "No path found in file entry".to_string())?;
            
            let dest_path = instance_dir.join(path);
            
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            }
            
            validate_download_url(download_url)?;
            client.download_mod_file(download_url, &dest_path)
                .await
                .map_err(|e| format!("Failed to download mod: {}", e))?;
            
            let progress = 70 + ((idx + 1) * 25 / total_files) as u32;
            let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
                "instance": safe_name,
                "progress": progress,
                "stage": format!("Downloading mods... ({}/{})", idx + 1, total_files)
            }));
        }
    }
    
    let _ = std::fs::remove_dir_all(&extract_dir);

    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 100,
        "stage": "Installation complete!"
    }));
    
    Ok(format!("Successfully installed modpack '{}'", safe_name))
}