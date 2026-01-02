use crate::services::instance::InstanceManager;
use crate::services::installer::MinecraftInstaller;
use crate::services::fabric::FabricInstaller;
use crate::services::accounts::AccountManager;
use crate::models::Instance;
use crate::utils::*;
use crate::commands::validation::sanitize_instance_name;
use tauri::Emitter;
use base64::{Engine as _, engine::general_purpose};

#[tauri::command]
pub async fn create_instance(
    instance_name: String,
    version: String,
    loader: Option<String>,
    loader_version: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    if !version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid version format".to_string());
    }
    
    if let Some(ref loader_type) = loader {
        if loader_type != "fabric" && loader_type != "vanilla" {
            return Err("Invalid loader type".to_string());
        }
    }
    
    if let Some(ref lv) = loader_version {
        if !lv.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
            return Err("Invalid loader version format".to_string());
        }
    }

    let _ = app_handle.emit("creation-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 0,
        "stage": "Starting instance creation..."
    }));

    println!("Creating instance: {}", safe_name);
    println!("Minecraft version: {}", version);
    println!("Loader type: {:?}", loader);

    let _ = app_handle.emit("creation-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 10,
        "stage": format!("Checking Minecraft {}...", version)
    }));

    let meta_dir = get_meta_dir();
    let installer = MinecraftInstaller::new(meta_dir.clone());
    
    // Check if already installed
    let needs_installation = !installer.check_version_installed(&version);
    
    if needs_installation {
        let _ = app_handle.emit("creation-progress", serde_json::json!({
            "instance": safe_name,
            "progress": 20,
            "stage": format!("Installing Minecraft {}...", version)
        }));

        installer
            .install_version(&version)
            .await
            .map_err(|e| {
                let err_msg = format!("Failed to install Minecraft: {}", e);
                println!("ERROR: {}", err_msg);
                err_msg
            })?;
    }

    let _ = app_handle.emit("creation-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 60,
        "stage": "Minecraft version ready"
    }));

    println!("✓ Minecraft {} is ready", version);

    let final_version = if let Some(loader_type) = &loader {
        if loader_type == "fabric" {
            if let Some(fabric_version) = &loader_version {
                let _ = app_handle.emit("creation-progress", serde_json::json!({
                    "instance": safe_name,
                    "progress": 70,
                    "stage": format!("Installing Fabric {}...", fabric_version)
                }));

                println!("Installing Fabric loader {}...", fabric_version);
                let fabric_installer = FabricInstaller::new(meta_dir);
                
                match fabric_installer
                    .install_fabric(&version, fabric_version)
                    .await
                {
                    Ok(fabric_id) => {
                        println!("✓ Fabric {} is ready", fabric_version);
                        fabric_id
                    }
                    Err(e) => {
                        let err_msg = format!("Failed to install Fabric: {}", e);
                        println!("ERROR: {}", err_msg);
                        return Err(err_msg);
                    }
                }
            } else {
                let err_msg = "Fabric loader version not specified".to_string();
                println!("ERROR: {}", err_msg);
                return Err(err_msg);
            }
        } else {
            println!("Using vanilla version (no mod loader)");
            version.clone()
        }
    } else {
        println!("Using vanilla version (no mod loader)");
        version.clone()
    };

    let _ = app_handle.emit("creation-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 90,
        "stage": "Creating instance structure..."
    }));

    println!("Creating instance with version: {}", final_version);
    InstanceManager::create(&safe_name, &final_version, loader.clone(), loader_version.clone())
        .map_err(|e| {
            let err_msg = format!("Failed to create instance: {}", e);
            println!("ERROR: {}", err_msg);
            err_msg
        })?;

    let _ = app_handle.emit("creation-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 100,
        "stage": "Instance created successfully!"
    }));

    let success_msg = format!("Successfully created instance '{}'", safe_name);
    println!("✓ {}", success_msg);
    Ok(success_msg)
}

#[tauri::command]
pub async fn get_instances() -> Result<Vec<Instance>, String> {
    InstanceManager::get_all().map_err(|e| format!("Failed to get instances: {}", e))
}

#[tauri::command]
pub async fn delete_instance(instance_name: String) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    InstanceManager::delete(&safe_name)
        .map_err(|e| format!("Failed to delete instance: {}", e))?;

    Ok(format!("Successfully deleted instance '{}'", safe_name))
}

#[tauri::command]
pub async fn rename_instance(old_name: String, new_name: String) -> Result<String, String> {
    let safe_old_name = sanitize_instance_name(&old_name)?;
    let safe_new_name = sanitize_instance_name(&new_name)?;
    
    if safe_old_name == safe_new_name {
        return Ok("Instance name unchanged".to_string());
    }
    
    let instances_dir = get_instances_dir();
    let old_path = instances_dir.join(&safe_old_name);
    let new_path = instances_dir.join(&safe_new_name);
    
    if !old_path.exists() {
        return Err(format!("Instance '{}' does not exist", safe_old_name));
    }
    
    if new_path.exists() {
        return Err(format!("Instance '{}' already exists", safe_new_name));
    }
    
    std::fs::rename(&old_path, &new_path)
        .map_err(|e| format!("Failed to rename instance directory: {}", e))?;
    
    let instance_json_path = new_path.join("instance.json");
    if instance_json_path.exists() {
        let content = std::fs::read_to_string(&instance_json_path)
            .map_err(|e| format!("Failed to read instance.json: {}", e))?;
        
        let mut instance: Instance = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse instance.json: {}", e))?;
        
        instance.name = safe_new_name.clone();
        
        let updated_json = serde_json::to_string_pretty(&instance)
            .map_err(|e| format!("Failed to serialize instance.json: {}", e))?;
        
        std::fs::write(&instance_json_path, updated_json)
            .map_err(|e| format!("Failed to write instance.json: {}", e))?;
    }
    
    Ok(format!("Successfully renamed instance to '{}'", safe_new_name))
}

#[tauri::command]
pub async fn launch_instance_with_active_account(
    instance_name: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let active_account = AccountManager::get_active_account()
        .map_err(|e| format!("Failed to get active account: {}", e))?
        .ok_or_else(|| "No active account. Please sign in first.".to_string())?;
    
    crate::services::instance::InstanceManager::launch(
        &safe_name,
        &active_account.username,
        &active_account.uuid,
        &active_account.access_token,
        app_handle,
    )
    .map_err(|e| format!("Failed to launch instance: {}", e))?;

    Ok(format!("Launched instance '{}' with account {}", safe_name, active_account.username))
}

#[tauri::command]
pub async fn launch_instance(
    instance_name: String,
    username: String,
    uuid: String,
    access_token: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Invalid username format".to_string());
    }
    
    if !uuid.chars().all(|c| c.is_alphanumeric() || c == '-') || uuid.len() > 36 {
        return Err("Invalid UUID format".to_string());
    }
    
    InstanceManager::launch(&safe_name, &username, &uuid, &access_token, app_handle)
        .map_err(|e| format!("Failed to launch instance: {}", e))?;

    Ok(format!("Launched instance '{}'", safe_name))
}

#[tauri::command]
pub async fn set_instance_icon(
    instance_name: String,
    image_data: String,
) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    
    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }
    
    let image_bytes = general_purpose::STANDARD
        .decode(&image_data)
        .map_err(|e| format!("Invalid base64 image data: {}", e))?;
    
    if image_bytes.len() > 2 * 1024 * 1024 {
        return Err("Image too large (max 2MB)".to_string());
    }
    
    let format = image::guess_format(&image_bytes)
        .map_err(|e| format!("Invalid image format: {}", e))?;
    
    match format {
        image::ImageFormat::Png | 
        image::ImageFormat::Jpeg | 
        image::ImageFormat::WebP => {},
        _ => return Err("Unsupported image format. Use PNG, JPEG, or WebP".to_string()),
    }
    
    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| format!("Failed to load image: {}", e))?;
    
    let resized = img.resize_exact(256, 256, image::imageops::FilterType::Lanczos3);
    
    let icon_path = instance_dir.join("icon.png");
    resized.save(&icon_path)
        .map_err(|e| format!("Failed to save icon: {}", e))?;
    
    let instance_json = instance_dir.join("instance.json");
    let content = std::fs::read_to_string(&instance_json)
        .map_err(|e| format!("Failed to read instance data: {}", e))?;
    
    let mut instance: Instance = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse instance data: {}", e))?;
    
    instance.icon_path = Some("icon.png".to_string());
    
    let updated_json = serde_json::to_string_pretty(&instance)
        .map_err(|e| format!("Failed to serialize instance data: {}", e))?;
    
    std::fs::write(&instance_json, updated_json)
        .map_err(|e| format!("Failed to write instance data: {}", e))?;
    
    Ok("Icon set successfully".to_string())
}

#[tauri::command]
pub async fn remove_instance_icon(instance_name: String) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    
    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }
    
    let icon_path = instance_dir.join("icon.png");
    if icon_path.exists() {
        std::fs::remove_file(&icon_path)
            .map_err(|e| format!("Failed to remove icon file: {}", e))?;
    }
    
    let instance_json = instance_dir.join("instance.json");
    let content = std::fs::read_to_string(&instance_json)
        .map_err(|e| format!("Failed to read instance data: {}", e))?;
    
    let mut instance: Instance = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse instance data: {}", e))?;
    
    instance.icon_path = None;
    
    let updated_json = serde_json::to_string_pretty(&instance)
        .map_err(|e| format!("Failed to serialize instance data: {}", e))?;
    
    std::fs::write(&instance_json, updated_json)
        .map_err(|e| format!("Failed to write instance data: {}", e))?;
    
    Ok("Icon removed successfully".to_string())
}

#[tauri::command]
pub async fn get_instance_icon(instance_name: String) -> Result<Option<String>, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    
    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }
    
    let icon_path = instance_dir.join("icon.png");
    
    if !icon_path.exists() {
        return Ok(None);
    }
    
    let canonical_icon = icon_path.canonicalize()
        .map_err(|_| "Icon file not found".to_string())?;
    
    let canonical_instance = instance_dir.canonicalize()
        .map_err(|_| "Instance directory not found".to_string())?;
    
    if !canonical_icon.starts_with(&canonical_instance) {
        return Err("Invalid icon path".to_string());
    }
    
    let image_bytes = std::fs::read(&icon_path)
        .map_err(|e| format!("Failed to read icon: {}", e))?;
    
    let base64_data = general_purpose::STANDARD.encode(&image_bytes);
    
    Ok(Some(format!("data:image/png;base64,{}", base64_data)))
}

#[tauri::command]
pub async fn duplicate_instance(
    instance_name: String,
    new_name: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let safe_old_name = sanitize_instance_name(&instance_name)?;
    let safe_new_name = sanitize_instance_name(&new_name)?;
    
    if safe_old_name == safe_new_name {
        return Err("Source and destination names cannot be the same".to_string());
    }
    
    let instances_dir = get_instances_dir();
    let source_path = instances_dir.join(&safe_old_name);
    let dest_path = instances_dir.join(&safe_new_name);
    
    if !source_path.exists() {
        return Err(format!("Instance '{}' does not exist", safe_old_name));
    }
    
    if dest_path.exists() {
        return Err(format!("Instance '{}' already exists", safe_new_name));
    }
    
    println!("Duplicating instance '{}' to '{}'", safe_old_name, safe_new_name);
    
    let _ = app_handle.emit("duplication-progress", serde_json::json!({
        "instance": safe_new_name,
        "progress": 0,
        "stage": "Calculating size..."
    }));
    
    let total_files = count_files(&source_path)
        .map_err(|e| format!("Failed to count files: {}", e))?;
    
    println!("Total files to copy: {}", total_files);
    
    let copied_files = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    copy_dir_recursive_with_progress(
        &source_path,
        &dest_path,
        total_files,
        copied_files.clone(),
        &safe_new_name,
        &app_handle,
    )
    .map_err(|e| format!("Failed to copy instance directory: {}", e))?;
    
    let _ = app_handle.emit("duplication-progress", serde_json::json!({
        "instance": safe_new_name,
        "progress": 90,
        "stage": "Updating metadata..."
    }));
    
    let instance_json_path = dest_path.join("instance.json");
    if instance_json_path.exists() {
        let content = std::fs::read_to_string(&instance_json_path)
            .map_err(|e| format!("Failed to read instance.json: {}", e))?;
        
        let mut instance: Instance = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse instance.json: {}", e))?;
        
        instance.name = safe_new_name.clone();
        instance.created_at = chrono::Utc::now().to_rfc3339();
        instance.last_played = None;
        
        let updated_json = serde_json::to_string_pretty(&instance)
            .map_err(|e| format!("Failed to serialize instance.json: {}", e))?;
        
        std::fs::write(&instance_json_path, updated_json)
            .map_err(|e| format!("Failed to write instance.json: {}", e))?;
    }
    
    let _ = app_handle.emit("duplication-progress", serde_json::json!({
        "instance": safe_new_name,
        "progress": 100,
        "stage": "Complete!"
    }));
    
    println!("✓ Successfully duplicated instance");
    Ok(format!("Successfully duplicated instance to '{}'", safe_new_name))
}

fn count_files(path: &std::path::Path) -> std::io::Result<usize> {
    use std::fs;
    
    let mut count = 0;
    
    if path.is_file() {
        return Ok(1);
    }
    
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        
        if entry.file_name() == "natives" {
            continue;
        }
        
        if entry_path.is_dir() {
            count += count_files(&entry_path)?;
        } else {
            count += 1;
        }
    }
    
    Ok(count)
}

fn copy_dir_recursive_with_progress(
    src: &std::path::Path,
    dst: &std::path::Path,
    total_files: usize,
    copied_files: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    instance_name: &str,
    app_handle: &tauri::AppHandle,
) -> std::io::Result<()> {
    use std::fs;
    use std::sync::atomic::Ordering;
    
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if file_type.is_dir() {
            if entry.file_name() == "natives" {
                continue;
            }
            copy_dir_recursive_with_progress(
                &src_path,
                &dst_path,
                total_files,
                copied_files.clone(),
                instance_name,
                app_handle,
            )?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path)?;
            
            let current = copied_files.fetch_add(1, Ordering::Relaxed) + 1;
            let progress = ((current as f64 / total_files as f64) * 85.0) as u32;
            
            if current % 10 == 0 || progress >= 85 {
                let file_name = src_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file");
                
                let _ = app_handle.emit("duplication-progress", serde_json::json!({
                    "instance": instance_name,
                    "progress": progress,
                    "stage": format!("Copying files... ({}/{})", current, total_files),
                    "current_file": file_name
                }));
            }
        }
    }
    
    Ok(())
}

#[tauri::command]
pub fn get_launcher_directory() -> String {
    get_launcher_dir().to_string_lossy().to_string()
}

#[tauri::command]
pub fn open_instance_folder(instance_name: String) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);

    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }

    open_folder(instance_dir).map_err(|e| format!("Failed to open folder: {}", e))?;

    Ok(format!("Opened folder for instance '{}'", safe_name))
}

// SYSTEM UTILITIES

use sysinfo::System;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SystemInfo {
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
    pub recommended_max_memory_mb: u64,
}

#[tauri::command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    let mut sys = System::new_all();
    sys.refresh_memory();
    
    let total_mb = sys.total_memory() / 1024 / 1024;
    let available_mb = sys.available_memory() / 1024 / 1024;
    let recommended_max_mb = total_mb * 80 / 100;
    
    Ok(SystemInfo {
        total_memory_mb: total_mb,
        available_memory_mb: available_mb,
        recommended_max_memory_mb: recommended_max_mb,
    })
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    open::that(url).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_debug_report(version: String) -> Result<String, String> {
    if !version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid version format".to_string());
    }
    
    Ok(crate::utils::generate_debug_report(&version))
}

#[tauri::command]
pub async fn save_debug_report(version: String) -> Result<String, String> {
    if !version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid version format".to_string());
    }
    
    let report = crate::utils::generate_debug_report(&version);
    let logs_dir = get_logs_dir();
    
    std::fs::create_dir_all(&logs_dir)
        .map_err(|e| format!("Failed to create logs directory: {}", e))?;
    
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("debug_report_{}.txt", timestamp);
    let filepath = logs_dir.join(&filename);
    
    std::fs::write(&filepath, report)
        .map_err(|e| format!("Failed to write debug report: {}", e))?;
    
    Ok(filepath.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_worlds_folder(instance_name: String) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let saves_dir = get_instance_dir(&safe_name).join("saves");

    if !saves_dir.exists() {
        std::fs::create_dir_all(&saves_dir)
            .map_err(|e| format!("Failed to create saves folder: {}", e))?;
    }

    open_folder(saves_dir).map_err(|e| format!("Failed to open saves folder: {}", e))?;

    Ok(format!("Opened saves folder for instance '{}'", safe_name))
}

#[tauri::command]
pub fn open_world_folder(instance_name: String, folder_name: String) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    // Sanitize folder_name to prevent path traversal
    if folder_name.contains("..") || folder_name.contains("/") || folder_name.contains("\\") {
        return Err("Invalid folder name".to_string());
    }
    
    let world_dir = get_instance_dir(&safe_name).join("saves").join(&folder_name);

    if !world_dir.exists() {
        return Err(format!("World folder '{}' does not exist", folder_name));
    }

    open_folder(world_dir).map_err(|e| format!("Failed to open world folder: {}", e))?;

    Ok(format!("Opened world folder '{}'", folder_name))
}

#[tauri::command]
pub fn delete_world(instance_name: String, folder_name: String) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    // Sanitize folder_name to prevent path traversal
    if folder_name.contains("..") || folder_name.contains("/") || folder_name.contains("\\") {
        return Err("Invalid folder name".to_string());
    }
    
    let world_dir = get_instance_dir(&safe_name).join("saves").join(&folder_name);

    if !world_dir.exists() {
        return Err(format!("World folder '{}' does not exist", folder_name));
    }

    std::fs::remove_dir_all(&world_dir)
        .map_err(|e| format!("Failed to delete world folder: {}", e))?;

    Ok(format!("Successfully deleted world '{}'", folder_name))
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct World {
    pub name: String,
    pub folder_name: String,
    pub size: u64,
    pub last_played: Option<i64>,
    pub game_mode: Option<String>,
    pub version: Option<String>,
    pub icon: Option<String>,
    pub created: Option<i64>,
}

#[tauri::command]
pub fn get_instance_worlds(instance_name: String) -> Result<Vec<World>, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let saves_dir = get_instance_dir(&safe_name).join("saves");

    if !saves_dir.exists() {
        return Ok(Vec::new());
    }

    let mut worlds = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&saves_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            
            if path.is_dir() {
                let folder_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                // Calculate folder size
                let size = calculate_dir_size(&path).unwrap_or(0);

                // Get creation timestamp
                let created = path.metadata()
                    .ok()
                    .and_then(|m| m.created().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64);

                // Try to read world icon
                let icon = read_world_icon(&path);

                worlds.push(World {
                    name: folder_name.clone(),
                    folder_name,
                    size,
                    last_played: None,
                    game_mode: None,
                    version: None,
                    icon,
                    created,
                });
            }
        }
    }

    worlds.sort_by(|a, b| {
        match (a.created, b.created) {
            (Some(a_time), Some(b_time)) => b_time.cmp(&a_time),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.folder_name.cmp(&b.folder_name),
        }
    });

    Ok(worlds)
}

fn read_world_icon(world_path: &std::path::Path) -> Option<String> {
    let icon_path = world_path.join("icon.png");
    
    if !icon_path.exists() {
        return None;
    }
    
    // Read the icon file
    if let Ok(image_bytes) = std::fs::read(&icon_path) {
        let base64_data = base64::engine::general_purpose::STANDARD.encode(&image_bytes);
        Some(format!("data:image/png;base64,{}", base64_data))
    } else {
        None
    }
}

fn calculate_dir_size(path: &std::path::Path) -> std::io::Result<u64> {
    let mut size = 0u64;
    
    if path.is_file() {
        return Ok(path.metadata()?.len());
    }
    
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        
        if entry_path.is_dir() {
            size += calculate_dir_size(&entry_path)?;
        } else {
            size += entry.metadata()?.len();
        }
    }
    
    Ok(size)
}

#[tauri::command]
pub async fn update_instance_fabric_loader(
    instance_name: String,
    fabric_version: String,
) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    if !fabric_version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid fabric version format".to_string());
    }
    
    let instance_dir = get_instance_dir(&safe_name);
    
    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }
    
    // Load instance metadata
    let instance_json_path = instance_dir.join("instance.json");
    let content = std::fs::read_to_string(&instance_json_path)
        .map_err(|e| format!("Failed to read instance.json: {}", e))?;
    
    let mut instance: Instance = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse instance.json: {}", e))?;
    
    // Verify this is a Fabric instance
    if instance.loader != Some("fabric".to_string()) {
        return Err("This instance is not using Fabric loader".to_string());
    }
    
    // Get the Minecraft version (need to extract from the fabric version string)
    let minecraft_version = if instance.version.contains("fabric-loader") {
        // Extract Minecraft version from fabric version string
        // Format: fabric-loader-X.X.X-1.XX.X -> 1.XX.X
        let parts: Vec<&str> = instance.version.split('-').collect();
        if let Some(mc_version) = parts.last() {
            mc_version.to_string()
        } else {
            return Err("Could not determine Minecraft version from instance".to_string());
        }
    } else {
        instance.version.clone()
    };
    
    println!("Updating Fabric loader for instance '{}'", safe_name);
    println!("Minecraft version: {}", minecraft_version);
    println!("New Fabric version: {}", fabric_version);
    
    // Install the new Fabric version
    let meta_dir = get_meta_dir();
    let fabric_installer = FabricInstaller::new(meta_dir);
    
    let new_fabric_version_id = fabric_installer
        .install_fabric(&minecraft_version, &fabric_version)
        .await
        .map_err(|e| format!("Failed to install Fabric: {}", e))?;
    
    println!("✓ Installed Fabric version: {}", new_fabric_version_id);
    
    // Update instance metadata
    instance.version = new_fabric_version_id;
    instance.loader_version = Some(fabric_version);
    
    let updated_json = serde_json::to_string_pretty(&instance)
        .map_err(|e| format!("Failed to serialize instance.json: {}", e))?;
    
    std::fs::write(&instance_json_path, updated_json)
        .map_err(|e| format!("Failed to write instance.json: {}", e))?;
    
    Ok(format!("Successfully updated Fabric loader to version {}", instance.loader_version.as_deref().unwrap_or("unknown")))
}