use crate::services::instance::InstanceManager;
use crate::services::installer::MinecraftInstaller;
use crate::services::fabric::FabricInstaller;
use crate::services::accounts::AccountManager;
use crate::models::{AppConfig, Instance};
use crate::utils::*;
use std::sync::Mutex;
use crate::commands::validation::sanitize_instance_name;
use tauri::{Emitter, Manager};
use base64::{Engine as _, engine::general_purpose};
use std::path::PathBuf;

fn resolve_world_dir(instance_name: &str, folder_name: &str) -> Result<PathBuf, String> {
    let safe_name = sanitize_instance_name(instance_name)?;

    if folder_name.is_empty()
        || folder_name.starts_with('.')
        || folder_name.contains("..")
        || folder_name.contains('/')
        || folder_name.contains('\\')
    {
        return Err("Invalid world folder name".to_string());
    }

    let saves_dir = get_instance_dir(&safe_name).join("saves");
    let world_dir = saves_dir.join(folder_name);

    let canonical_world = world_dir
        .canonicalize()
        .map_err(|_| format!("World folder '{}' does not exist", folder_name))?;
    let canonical_saves = saves_dir
        .canonicalize()
        .map_err(|e| e.to_string())?;

    if canonical_world.parent() != Some(canonical_saves.as_path()) {
        return Err("Invalid world folder name".to_string());
    }

    Ok(world_dir)
}

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
        if loader_type != "fabric" && loader_type != "vanilla" && loader_type != "neoforge" && loader_type != "forge" {
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

    let _ = app_handle.emit("creation-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 10,
        "stage": format!("Checking Minecraft {}...", version)
    }));

    let meta_dir = get_meta_dir();
    let installer = MinecraftInstaller::new(meta_dir.clone())
        .map_err(|e| e.to_string())?;
    
    let _ = app_handle.emit("creation-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 20,
        "stage": format!("Installing Minecraft {}...", version)
    }));

    installer
        .install_version(&version)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit("creation-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 60,
        "stage": "Minecraft version ready"
    }));

    let final_version = if let Some(loader_type) = &loader {
        if loader_type == "fabric" {
            if let Some(fabric_version) = &loader_version {
                let _ = app_handle.emit("creation-progress", serde_json::json!({
                    "instance": safe_name,
                    "progress": 70,
                    "stage": format!("Installing Fabric {}...", fabric_version)
                }));

                let fabric_installer = FabricInstaller::new(meta_dir.clone())
                    .map_err(|e| e.to_string())?;
                
                fabric_installer
                    .install_fabric(&version, fabric_version)
                    .await
                    .map_err(|e| e.to_string())?
            } else {
                return Err("Fabric loader version not specified".to_string());
            }
        } else if loader_type == "forge" {
            if let Some(forge_version) = &loader_version {
                let _ = app_handle.emit("creation-progress", serde_json::json!({
                    "instance": safe_name,
                    "progress": 70,
                    "stage": format!("Downloading Forge installer {}...", forge_version)
                }));

                let forge_installer = crate::services::forge::ForgeInstaller::new(meta_dir.clone())
                    .map_err(|e| e.to_string())?;

                let app_handle_clone = app_handle.clone();
                let safe_name_clone = safe_name.clone();
                let progress_task = tauri::async_runtime::spawn(async move {
                    for i in 0..20 {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        let progress = 75 + (i * 1).min(10);
                        let _ = app_handle_clone.emit("creation-progress", serde_json::json!({
                            "instance": safe_name_clone,
                            "progress": progress,
                            "stage": "Running Forge installer (this may take a minute)..."
                        }));
                    }
                });
                
                let version_id = forge_installer
                    .install_forge(forge_version)
                    .await
                    .map_err(|e| e.to_string())?;
                
                progress_task.abort();
                    
                let _ = app_handle.emit("creation-progress", serde_json::json!({
                    "instance": safe_name,
                    "progress": 85,
                    "stage": "Forge installation complete"
                }));
                
                version_id
            } else {
                return Err("Forge loader version not specified".to_string());
            }
        } else if loader_type == "neoforge" {
            if let Some(neoforge_version) = &loader_version {
                let _ = app_handle.emit("creation-progress", serde_json::json!({
                    "instance": safe_name,
                    "progress": 70,
                    "stage": format!("Downloading NeoForge installer {}...", neoforge_version)
                }));

                let neoforge_installer = crate::services::neoforge::NeoForgeInstaller::new(meta_dir.clone())
                    .map_err(|e| e.to_string())?;

                let app_handle_clone = app_handle.clone();
                let safe_name_clone = safe_name.clone();
                let progress_task = tauri::async_runtime::spawn(async move {
                    for i in 0..20 {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        let progress = 75 + (i * 1).min(10);
                        let _ = app_handle_clone.emit("creation-progress", serde_json::json!({
                            "instance": safe_name_clone,
                            "progress": progress,
                            "stage": "Running NeoForge installer (this may take a minute)..."
                        }));
                    }
                });
                
                let version_id = neoforge_installer
                    .install_neoforge(neoforge_version)
                    .await
                    .map_err(|e| e.to_string())?;
                
                progress_task.abort();
                    
                let _ = app_handle.emit("creation-progress", serde_json::json!({
                    "instance": safe_name,
                    "progress": 85,
                    "stage": "NeoForge installation complete"
                }));
                
                version_id
            } else {
                return Err("NeoForge loader version not specified".to_string());
            }
        } else {
            version.clone()
        }
    } else {
        version.clone()
    };

    let _ = app_handle.emit("creation-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 90,
        "stage": "Creating instance structure..."
    }));

    InstanceManager::create(&safe_name, &final_version, loader.clone(), loader_version.clone())
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit("creation-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 100,
        "stage": "Instance created successfully!"
    }));

    Ok(format!("Successfully created instance '{}'", safe_name))
}

lazy_static::lazy_static! {
    pub static ref RUNNING_PROCESSES: Mutex<std::collections::HashMap<String, u32>> = Mutex::new(std::collections::HashMap::new());
}

#[tauri::command]
pub async fn kill_instance(instance_name: String) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let pid = {
        let processes = RUNNING_PROCESSES.lock().map_err(|e| e.to_string())?;
        processes.get(&safe_name).copied()
    };
    
    if let Some(pid) = pid {
        if pid == 0 {
            return Err("Invalid process PID".to_string());
        }

        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            let _ = Command::new("taskkill")
                .args(&["/F", "/PID", &pid.to_string()])
                .output();
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            if pid > 0 {
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
            }
        }
        
        let mut processes = RUNNING_PROCESSES.lock().map_err(|e| e.to_string())?;
        processes.remove(&safe_name);
        
        Ok(())
    } else {
        Err("Instance is not running".to_string())
    }
}

#[tauri::command]
pub async fn get_instances() -> Result<Vec<Instance>, String> {
    InstanceManager::get_all().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_instance(instance_name: String, permanent: bool) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    InstanceManager::delete(&safe_name, permanent)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_instance(old_name: String, new_name: String) -> Result<(), String> {
    let safe_old_name = sanitize_instance_name(&old_name)?;
    let safe_new_name = sanitize_instance_name(&new_name)?;
    
    if safe_old_name == safe_new_name {
        return Ok(());
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
        .map_err(|e| e.to_string())?;
    
    let instance_json_path = new_path.join("instance.json");
    if instance_json_path.exists() {
        let content = std::fs::read_to_string(&instance_json_path)
            .map_err(|e| e.to_string())?;
        
        let mut instance: Instance = serde_json::from_str(&content)
            .map_err(|e| e.to_string())?;
        
        instance.name = safe_new_name.clone();
        
        let updated_json = serde_json::to_string_pretty(&instance)
            .map_err(|e| e.to_string())?;
        
        std::fs::write(&instance_json_path, updated_json)
            .map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn launch_instance_with_active_account(
    instance_name: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let config = app_handle.state::<AppConfig>();

    let active_account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account")?;

    let access_token = AccountManager::get_valid_token(&active_account.uuid, &config.microsoft_client_id)
        .await
        .map_err(|e| e.to_string())?;

    crate::services::instance::InstanceManager::launch(
        &safe_name,
        &active_account.username,
        &active_account.uuid,
        &access_token,
        app_handle,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn launch_world(
    instance_name: String,
    world_name: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let config = app_handle.state::<AppConfig>();

    let active_account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account")?;

    let access_token = AccountManager::get_valid_token(&active_account.uuid, &config.microsoft_client_id)
        .await
        .map_err(|e| e.to_string())?;

    crate::services::instance::InstanceManager::launch_with_world(
        &safe_name,
        &active_account.username,
        &active_account.uuid,
        &access_token,
        &world_name,
        app_handle,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn launch_instance(
    instance_name: String,
    username: String,
    uuid: String,
    access_token: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Invalid username format".to_string());
    }
    
    if !uuid.chars().all(|c| c.is_alphanumeric() || c == '-') || uuid.len() > 36 {
        return Err("Invalid UUID format".to_string());
    }
    
    InstanceManager::launch(&safe_name, &username, &uuid, &access_token, app_handle)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_instance_icon(
    instance_name: String,
    image_data: String,
) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    
    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }
    
    let image_bytes = general_purpose::STANDARD
        .decode(&image_data)
        .map_err(|e| e.to_string())?;
    
    if image_bytes.len() > 2 * 1024 * 1024 {
        return Err("Image too large (max 2MB)".to_string());
    }
    
    let format = image::guess_format(&image_bytes)
        .map_err(|e| e.to_string())?;
    
    match format {
        image::ImageFormat::Png | 
        image::ImageFormat::Jpeg | 
        image::ImageFormat::WebP => {},
        _ => return Err("Unsupported image format. Use PNG, JPEG, or WebP".to_string()),
    }
    
    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| e.to_string())?;
    
    let resized = img.resize_exact(256, 256, image::imageops::FilterType::Lanczos3);
    
    let icon_path = instance_dir.join("icon.png");
    resized.save(&icon_path)
        .map_err(|e| e.to_string())?;
    
    let instance_json = instance_dir.join("instance.json");
    let content = std::fs::read_to_string(&instance_json)
        .map_err(|e| e.to_string())?;
    
    let mut instance: Instance = serde_json::from_str(&content)
        .map_err(|e| e.to_string())?;
    
    instance.icon_path = Some("icon.png".to_string());
    
    let updated_json = serde_json::to_string_pretty(&instance)
        .map_err(|e| e.to_string())?;
    
    std::fs::write(&instance_json, updated_json)
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn remove_instance_icon(instance_name: String) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    
    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }
    
    let icon_path = instance_dir.join("icon.png");
    if icon_path.exists() {
        std::fs::remove_file(&icon_path)
            .map_err(|e| e.to_string())?;
    }
    
    let instance_json = instance_dir.join("instance.json");
    let content = std::fs::read_to_string(&instance_json)
        .map_err(|e| e.to_string())?;
    
    let mut instance: Instance = serde_json::from_str(&content)
        .map_err(|e| e.to_string())?;
    
    instance.icon_path = None;
    
    let updated_json = serde_json::to_string_pretty(&instance)
        .map_err(|e| e.to_string())?;
    
    std::fs::write(&instance_json, updated_json)
        .map_err(|e| e.to_string())?;
    
    Ok(())
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
        .map_err(|e| e.to_string())?;
    
    let base64_data = general_purpose::STANDARD.encode(&image_bytes);
    
    Ok(Some(format!("data:image/png;base64,{}", base64_data)))
}

#[tauri::command]
pub async fn duplicate_instance(
    instance_name: String,
    new_name: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
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
    
    let _ = app_handle.emit("duplication-progress", serde_json::json!({
        "instance": safe_new_name,
        "progress": 0,
        "stage": "Calculating size..."
    }));
    
    let total_files = count_files(&source_path)
        .map_err(|e| e.to_string())?;
    
    let copied_files = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    copy_dir_recursive_with_progress(
        &source_path,
        &dest_path,
        total_files,
        copied_files.clone(),
        &safe_new_name,
        &app_handle,
    )
    .map_err(|e| e.to_string())?;
    
    let _ = app_handle.emit("duplication-progress", serde_json::json!({
        "instance": safe_new_name,
        "progress": 90,
        "stage": "Updating metadata..."
    }));
    
    let instance_json_path = dest_path.join("instance.json");
    if instance_json_path.exists() {
        let content = std::fs::read_to_string(&instance_json_path)
            .map_err(|e| e.to_string())?;
        
        let mut instance: Instance = serde_json::from_str(&content)
            .map_err(|e| e.to_string())?;
        
        instance.name = safe_new_name.clone();
        instance.created_at = chrono::Utc::now().to_rfc3339();
        instance.last_played = None;
        
        let updated_json = serde_json::to_string_pretty(&instance)
            .map_err(|e| e.to_string())?;
        
        std::fs::write(&instance_json_path, updated_json)
            .map_err(|e| e.to_string())?;
    }
    
    let _ = app_handle.emit("duplication-progress", serde_json::json!({
        "instance": safe_new_name,
        "progress": 100,
        "stage": "Complete!"
    }));
    
    Ok(())
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
pub fn open_instance_folder(instance_name: String) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);

    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }

    open_folder(instance_dir).map_err(|e| e.to_string())
}

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
pub fn open_worlds_folder(instance_name: String) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let saves_dir = get_instance_dir(&safe_name).join("saves");

    if !saves_dir.exists() {
        std::fs::create_dir_all(&saves_dir)
            .map_err(|e| e.to_string())?;
    }

    open_folder(saves_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_world_folder(instance_name: String, folder_name: String) -> Result<(), String> {
    let world_dir = resolve_world_dir(&instance_name, &folder_name)?;

    open_folder(world_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_world(instance_name: String, folder_name: String) -> Result<(), String> {
    let world_dir = resolve_world_dir(&instance_name, &folder_name)?;

    std::fs::remove_dir_all(&world_dir)
        .map_err(|e| e.to_string())
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

                let size = calculate_dir_size(&path).unwrap_or(0);

                let created = path.metadata()
                    .ok()
                    .and_then(|m| m.created().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64);

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
) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    if !fabric_version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid fabric version format".to_string());
    }
    
    let instance_dir = get_instance_dir(&safe_name);
    
    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }
    
    let instance_json_path = instance_dir.join("instance.json");
    let content = std::fs::read_to_string(&instance_json_path)
        .map_err(|e| e.to_string())?;
    
    let mut instance: Instance = serde_json::from_str(&content)
        .map_err(|e| e.to_string())?;
    
    if instance.loader != Some("fabric".to_string()) {
        return Err("This instance is not using Fabric loader".to_string());
    }
    
    let minecraft_version = if instance.version.contains("fabric-loader") {
        let parts: Vec<&str> = instance.version.split('-').collect();
        if let Some(mc_version) = parts.last() {
            mc_version.to_string()
        } else {
            return Err("Could not determine Minecraft version".to_string());
        }
    } else {
        instance.version.clone()
    };
    
    let meta_dir = get_meta_dir();
    let fabric_installer = FabricInstaller::new(meta_dir)
        .map_err(|e| e.to_string())?;
    
    let new_fabric_version_id = fabric_installer
        .install_fabric(&minecraft_version, &fabric_version)
        .await
        .map_err(|e| e.to_string())?;
    
    instance.version = new_fabric_version_id;
    instance.loader_version = Some(fabric_version);
    
    let updated_json = serde_json::to_string_pretty(&instance)
        .map_err(|e| e.to_string())?;
    
    std::fs::write(&instance_json_path, updated_json)
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn update_instance_neoforge_loader(
    instance_name: String,
    neoforge_version: String,
) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;

    if !neoforge_version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid NeoForge version format".to_string());
    }

    let instance_dir = get_instance_dir(&safe_name);

    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }

    let instance_json_path = instance_dir.join("instance.json");
    let content = std::fs::read_to_string(&instance_json_path)
        .map_err(|e| e.to_string())?;

    let mut instance: Instance = serde_json::from_str(&content)
        .map_err(|e| e.to_string())?;

    if instance.loader != Some("neoforge".to_string()) {
        return Err("This instance is not using NeoForge loader".to_string());
    }

    let meta_dir = get_meta_dir();
    let neoforge_installer = crate::services::neoforge::NeoForgeInstaller::new(meta_dir)
        .map_err(|e| e.to_string())?;

    let new_version_id = neoforge_installer
        .install_neoforge(&neoforge_version)
        .await
        .map_err(|e| e.to_string())?;

    instance.version = new_version_id;
    instance.loader_version = Some(neoforge_version);

    let updated_json = serde_json::to_string_pretty(&instance)
        .map_err(|e| e.to_string())?;

    std::fs::write(&instance_json_path, updated_json)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_instance_forge_loader(
    instance_name: String,
    forge_full_version: String,
) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;

    if !forge_full_version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid Forge version format".to_string());
    }

    let instance_dir = get_instance_dir(&safe_name);

    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }

    let instance_json_path = instance_dir.join("instance.json");
    let content = std::fs::read_to_string(&instance_json_path)
        .map_err(|e| e.to_string())?;

    let mut instance: Instance = serde_json::from_str(&content)
        .map_err(|e| e.to_string())?;

    if instance.loader != Some("forge".to_string()) {
        return Err("This instance is not using Forge loader".to_string());
    }

    let meta_dir = get_meta_dir();
    let forge_installer = crate::services::forge::ForgeInstaller::new(meta_dir)
        .map_err(|e| e.to_string())?;

    let new_version_id = forge_installer
        .install_forge(&forge_full_version)
        .await
        .map_err(|e| e.to_string())?;

    instance.version = new_version_id;
    instance.loader_version = Some(forge_full_version);

    let updated_json = serde_json::to_string_pretty(&instance)
        .map_err(|e| e.to_string())?;

    std::fs::write(&instance_json_path, updated_json)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_instance_minecraft_version(
    instance_name: String,
    new_minecraft_version: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    if !new_minecraft_version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid Minecraft version format".to_string());
    }
    
    let instance_dir = get_instance_dir(&safe_name);
    
    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }
    
    let instance_json_path = instance_dir.join("instance.json");
    let content = std::fs::read_to_string(&instance_json_path)
        .map_err(|e| e.to_string())?;
    
    let mut instance: Instance = serde_json::from_str(&content)
        .map_err(|e| e.to_string())?;
    
    let is_fabric = instance.loader == Some("fabric".to_string());
    let is_neoforge = instance.loader == Some("neoforge".to_string());
    let is_forge = instance.loader == Some("forge".to_string());

    if is_fabric {
        let _ = app_handle.emit("version-update-progress", serde_json::json!({
            "instance": safe_name,
            "stage": format!("Installing Minecraft {}...", new_minecraft_version)
        }));
        
        let meta_dir = get_meta_dir();
        let installer = MinecraftInstaller::new(meta_dir.clone())
            .map_err(|e| e.to_string())?;
        
        installer
            .install_version(&new_minecraft_version)
            .await
            .map_err(|e| e.to_string())?;
        
        let _ = app_handle.emit("version-update-progress", serde_json::json!({
            "instance": safe_name,
            "stage": "Finding compatible Fabric loader..."
        }));
        
        let fabric_installer = FabricInstaller::new(meta_dir.clone())
            .map_err(|e| e.to_string())?;
        let compatible_loader = fabric_installer
            .get_compatible_loader_for_minecraft(&new_minecraft_version)
            .await
            .map_err(|e| e.to_string())?;
        
        let _ = app_handle.emit("version-update-progress", serde_json::json!({
            "instance": safe_name,
            "stage": format!("Installing Fabric loader {}...", compatible_loader)
        }));
        
        let new_fabric_version_id = fabric_installer
            .install_fabric(&new_minecraft_version, &compatible_loader)
            .await
            .map_err(|e| e.to_string())?;
        
        instance.version = new_fabric_version_id;
        instance.loader_version = Some(compatible_loader);
    } else if is_neoforge {
        let _ = app_handle.emit("version-update-progress", serde_json::json!({
            "instance": safe_name,
            "stage": format!("Installing Minecraft {}...", new_minecraft_version)
        }));
        
        let meta_dir = get_meta_dir();
        let installer = MinecraftInstaller::new(meta_dir.clone())
            .map_err(|e| e.to_string())?;
        
        installer
            .install_version(&new_minecraft_version)
            .await
            .map_err(|e| e.to_string())?;
        
        let _ = app_handle.emit("version-update-progress", serde_json::json!({
            "instance": safe_name,
            "stage": "Finding compatible NeoForge loader..."
        }));
        
        let neoforge_installer = crate::services::neoforge::NeoForgeInstaller::new(meta_dir.clone())
            .map_err(|e| e.to_string())?;
        let compatible_loader = neoforge_installer
            .get_compatible_loader_for_minecraft(&new_minecraft_version)
            .await
            .map_err(|e| e.to_string())?;
        
        let _ = app_handle.emit("version-update-progress", serde_json::json!({
            "instance": safe_name,
            "stage": format!("Installing NeoForge loader {}...", compatible_loader)
        }));
        
        let new_version_id = neoforge_installer
            .install_neoforge(&compatible_loader)
            .await
            .map_err(|e| e.to_string())?;
        
        instance.version = new_version_id;
        instance.loader_version = Some(compatible_loader);
    } else if is_forge {
        let _ = app_handle.emit("version-update-progress", serde_json::json!({
            "instance": safe_name,
            "stage": format!("Installing Minecraft {}...", new_minecraft_version)
        }));
        
        let meta_dir = get_meta_dir();
        let installer = MinecraftInstaller::new(meta_dir.clone())
            .map_err(|e| e.to_string())?;
        
        installer
            .install_version(&new_minecraft_version)
            .await
            .map_err(|e| e.to_string())?;
        
        let _ = app_handle.emit("version-update-progress", serde_json::json!({
            "instance": safe_name,
            "stage": "Finding compatible Forge loader..."
        }));
        
        let forge_installer = crate::services::forge::ForgeInstaller::new(meta_dir.clone())
            .map_err(|e| e.to_string())?;
        let compatible_forge_ver = forge_installer
            .get_compatible_loader_for_minecraft(&new_minecraft_version)
            .await
            .map_err(|e| e.to_string())?;
        
        let full_version = format!("{}-{}", new_minecraft_version, compatible_forge_ver);
        
        let _ = app_handle.emit("version-update-progress", serde_json::json!({
            "instance": safe_name,
            "stage": format!("Installing Forge loader {}...", compatible_forge_ver)
        }));
        
        let new_version_id = forge_installer
            .install_forge(&full_version)
            .await
            .map_err(|e| e.to_string())?;
        
        instance.version = new_version_id;
        instance.loader_version = Some(full_version);
    } else {
        let _ = app_handle.emit("version-update-progress", serde_json::json!({
            "instance": safe_name,
            "stage": format!("Installing Minecraft {}...", new_minecraft_version)
        }));
        
        let meta_dir = get_meta_dir();
        let installer = MinecraftInstaller::new(meta_dir)
            .map_err(|e| e.to_string())?;
        
        installer
            .install_version(&new_minecraft_version)
            .await
            .map_err(|e| e.to_string())?;
        
        instance.version = new_minecraft_version.clone();
    }
    
    let _ = app_handle.emit("version-update-progress", serde_json::json!({
        "instance": safe_name,
        "stage": "Updating instance metadata..."
    }));
    
    let natives_dir = instance_dir.join("natives");
    if natives_dir.exists() {
        std::fs::remove_dir_all(&natives_dir)
            .map_err(|e| e.to_string())?;
    }
    
    let updated_json = serde_json::to_string_pretty(&instance)
        .map_err(|e| e.to_string())?;
    
    std::fs::write(&instance_json_path, updated_json)
        .map_err(|e| e.to_string())?;
    
    let _ = app_handle.emit("version-update-progress", serde_json::json!({
        "instance": safe_name,
        "stage": "Complete!"
    }));
    
    Ok(())
}

