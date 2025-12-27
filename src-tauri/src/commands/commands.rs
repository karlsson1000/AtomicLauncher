use crate::auth::Authenticator;
use crate::services::instance::InstanceManager;
use crate::services::installer::MinecraftInstaller;
use crate::services::fabric::FabricInstaller;
use crate::models::{AuthResponse, Instance, FabricLoaderVersion, LauncherSettings};
use crate::utils::modrinth::{ModrinthClient, ModrinthProjectDetails, ModrinthSearchResult, ModrinthVersion};
use crate::services::settings::SettingsManager;
use crate::services::template::TemplateManager;
use crate::models::{InstanceTemplate, MinecraftOptions};
use sysinfo::System;
use crate::utils::*;
use tauri::Emitter;
use std::path::PathBuf;
use url::Url;
use base64::{Engine as _, engine::general_purpose};
use crate::services::accounts::AccountManager;
use crate::models::AccountInfo;

// ===== SECURITY HELPERS =====

/// Sanitize instance names to prevent path traversal
fn sanitize_instance_name(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Instance name cannot be empty".to_string());
    }
    
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err("Instance name contains invalid characters".to_string());
    }
    
    if name.starts_with('.') {
        return Err("Instance name cannot start with a dot".to_string());
    }
    
    // Additional check for null bytes
    if name.contains('\0') {
        return Err("Instance name contains null bytes".to_string());
    }
    
    Ok(name.to_string())
}

/// Sanitize filenames to prevent path traversal
fn sanitize_filename(filename: &str) -> Result<String, String> {
    if filename.is_empty() {
        return Err("Filename cannot be empty".to_string());
    }
    
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err("Filename contains invalid characters".to_string());
    }
    
    if filename.starts_with('.') {
        return Err("Filename cannot start with a dot".to_string());
    }
    
    if filename.contains('\0') {
        return Err("Filename contains null bytes".to_string());
    }
    
    // Only allow .jar files for mods
    if !filename.ends_with(".jar") {
        return Err("Only .jar files are allowed".to_string());
    }
    
    Ok(filename.to_string())
}

/// Sanitize server names
fn sanitize_server_name(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Server name cannot be empty".to_string());
    }
    
    if name.len() > 100 {
        return Err("Server name too long (max 100 characters)".to_string());
    }
    
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err("Server name contains invalid characters".to_string());
    }
    
    if name.contains('\0') {
        return Err("Server name contains null bytes".to_string());
    }
    
    Ok(name.to_string())
}

/// Validate server address
fn validate_server_address(address: &str) -> Result<(), String> {
    if address.is_empty() {
        return Err("Server address cannot be empty".to_string());
    }
    
    if address.len() > 255 {
        return Err("Server address too long".to_string());
    }
    
    // Basic validation: alphanumeric, dots, hyphens
    if !address.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Server address contains invalid characters".to_string());
    }
    
    Ok(())
}

/// Validate Java executable path
fn validate_java_path(path: &str) -> Result<(), String> {
    let path_buf = PathBuf::from(path);
    
    // Check it exists
    if !path_buf.exists() {
        return Err("Java path does not exist".to_string());
    }
    
    // Check it's a file (not a directory)
    if !path_buf.is_file() {
        return Err("Java path must be a file".to_string());
    }
    
    // Check filename is java or javaw
    let filename = path_buf.file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?;
    
    let valid_names = if cfg!(windows) {
        vec!["java.exe", "javaw.exe"]
    } else {
        vec!["java"]
    };
    
    if !valid_names.contains(&filename) {
        return Err(format!("Java executable must be named: {}", valid_names.join(" or ")));
    }
    
    // Verify it's actually Java by checking version
    let output = std::process::Command::new(path)
        .arg("-version")
        .output()
        .map_err(|e| format!("Failed to execute Java: {}", e))?;
    
    let version_output = String::from_utf8_lossy(&output.stderr);
    if !version_output.to_lowercase().contains("java") && 
       !version_output.to_lowercase().contains("openjdk") {
        return Err("Not a valid Java executable".to_string());
    }
    
    Ok(())
}

/// Validate download URL is from trusted sources
fn validate_download_url(url: &str) -> Result<Url, String> {
    let parsed_url = Url::parse(url)
        .map_err(|_| "Invalid URL format".to_string())?;
    
    // Only allow HTTPS
    if parsed_url.scheme() != "https" {
        return Err("Only HTTPS URLs are allowed".to_string());
    }
    
    // Whitelist allowed domains
    let allowed_hosts = vec![
        "cdn.modrinth.com",
        "github.com",
        "raw.githubusercontent.com",
    ];
    
    let host = parsed_url.host_str()
        .ok_or("URL has no host")?;
    
    if !allowed_hosts.contains(&host) {
        return Err(format!("Downloads only allowed from: {}", allowed_hosts.join(", ")));
    }
    
    Ok(parsed_url)
}

// ===== SYSTEM INFO HELPERS =====

/// Get total system memory in MB
fn get_system_memory_mb() -> u64 {
    let mut sys = System::new_all();
    sys.refresh_memory();
    sys.total_memory() / 1024 / 1024 // Convert bytes to MB
}

/// Validate memory allocation against system memory
fn validate_memory_allocation(memory_mb: u64) -> Result<(), String> {
    if memory_mb < 512 {
        return Err("Memory allocation must be at least 512MB".to_string());
    }
    
    let system_memory = get_system_memory_mb();
    
    if memory_mb > system_memory {
        return Err(format!(
            "Memory allocation ({} MB) exceeds system memory ({} MB)",
            memory_mb, system_memory
        ));
    }
    
    // Warn if allocation is more than 80% of system memory
    if memory_mb > (system_memory * 80 / 100) {
        return Err(format!(
            "Memory allocation ({} MB) is too high. Recommended maximum: {} MB (80% of system memory)",
            memory_mb, system_memory * 80 / 100
        ));
    }
    
    Ok(())
}

// ===== MODEL STRUCTURES =====

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ModFile {
    filename: String,
    size: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ServerInfo {
    name: String,
    address: String,
    port: u16,
    status: String,
    players_online: Option<u32>,
    players_max: Option<u32>,
    version: Option<String>,
    motd: Option<String>,
    favicon: Option<String>,
    last_checked: Option<i64>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SystemInfo {
    total_memory_mb: u64,
    available_memory_mb: u64,
    recommended_max_memory_mb: u64,
}

// ===== AUTHENTICATION COMMANDS =====

#[tauri::command]
pub async fn microsoft_login() -> Result<AuthResponse, String> {
    let authenticator = Authenticator::new()
        .map_err(|e| format!("Failed to initialize authenticator: {}", e))?;
    
    authenticator
        .authenticate()
        .await
        .map_err(|e| format!("Authentication failed: {}", e))
}

// ===== MULTI-ACCOUNT COMMANDS =====

/// Get all stored accounts
#[tauri::command]
pub async fn get_accounts() -> Result<Vec<AccountInfo>, String> {
    AccountManager::get_all_accounts()
        .map_err(|e| format!("Failed to get accounts: {}", e))
}

/// Get the currently active account
#[tauri::command]
pub async fn get_active_account() -> Result<Option<AccountInfo>, String> {
    let active = AccountManager::get_active_account()
        .map_err(|e| format!("Failed to get active account: {}", e))?;
    
    if let Some(account) = active {
        Ok(Some(AccountInfo {
            uuid: account.uuid,
            username: account.username,
            is_active: true,
            added_at: account.added_at,
            last_used: account.last_used,
        }))
    } else {
        Ok(None)
    }
}

/// Switch to a different account
#[tauri::command]
pub async fn switch_account(uuid: String) -> Result<String, String> {
    // Validate UUID format
    if !uuid.chars().all(|c| c.is_alphanumeric() || c == '-') || uuid.len() > 36 {
        return Err("Invalid UUID format".to_string());
    }
    
    AccountManager::set_active_account(&uuid)
        .map_err(|e| format!("Failed to switch account: {}", e))?;
    
    Ok(format!("Switched to account {}", uuid))
}

/// Remove an account
#[tauri::command]
pub async fn remove_account(uuid: String) -> Result<String, String> {
    // Validate UUID format
    if !uuid.chars().all(|c| c.is_alphanumeric() || c == '-') || uuid.len() > 36 {
        return Err("Invalid UUID format".to_string());
    }
    
    AccountManager::remove_account(&uuid)
        .map_err(|e| format!("Failed to remove account: {}", e))?;
    
    Ok(format!("Account {} removed", uuid))
}

/// Login with Microsoft
#[tauri::command]
pub async fn microsoft_login_and_store() -> Result<AccountInfo, String> {
    let authenticator = crate::auth::Authenticator::new()
        .map_err(|e| format!("Failed to initialize authenticator: {}", e))?;
    
    let auth_response = authenticator
        .authenticate()
        .await
        .map_err(|e| format!("Authentication failed: {}", e))?;
    
    // Check if account already exists
    let account_exists = AccountManager::account_exists(&auth_response.uuid)
        .map_err(|e| format!("Failed to check account: {}", e))?;
    
    if account_exists {
        // Just switch to this account and update last used
        AccountManager::set_active_account(&auth_response.uuid)
            .map_err(|e| format!("Failed to switch account: {}", e))?;
    } else {
        // Add new account
        AccountManager::add_account(
            auth_response.uuid.clone(),
            auth_response.username.clone(),
            auth_response.access_token.clone(),
        )
        .map_err(|e| format!("Failed to store account: {}", e))?;
    }
    
    // Get the account info to return
    let accounts = AccountManager::get_all_accounts()
        .map_err(|e| format!("Failed to get accounts: {}", e))?;
    
    accounts
        .into_iter()
        .find(|acc| acc.uuid == auth_response.uuid)
        .ok_or_else(|| "Failed to retrieve account info".to_string())
}

/// Launch instance with the active account
#[tauri::command]
pub async fn launch_instance_with_active_account(
    instance_name: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // Sanitize instance name
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    // Get active account
    let active_account = AccountManager::get_active_account()
        .map_err(|e| format!("Failed to get active account: {}", e))?
        .ok_or_else(|| "No active account. Please sign in first.".to_string())?;
    
    // Launch with the active account credentials
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

// ===== MINECRAFT VERSION COMMANDS =====

#[tauri::command]
pub async fn get_minecraft_versions() -> Result<Vec<String>, String> {
    let installer = MinecraftInstaller::new(get_meta_dir());
    installer
        .get_versions()
        .await
        .map_err(|e| format!("Failed to fetch versions: {}", e))
}

#[tauri::command]
pub async fn install_minecraft(version: String) -> Result<String, String> {
    // Validate version string (basic alphanumeric + dots check)
    if !version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid version format".to_string());
    }
    
    let meta_dir = get_meta_dir();
    let installer = MinecraftInstaller::new(meta_dir);

    installer
        .install_version(&version)
        .await
        .map_err(|e| format!("Installation failed: {}", e))?;

    Ok(format!("Successfully installed Minecraft {}", version))
}

#[tauri::command]
pub async fn check_version_installed(version: String) -> Result<bool, String> {
    // Validate version string
    if !version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid version format".to_string());
    }
    
    let meta_dir = get_meta_dir();
    let installer = MinecraftInstaller::new(meta_dir);
    Ok(installer.check_version_installed(&version))
}

// ===== FABRIC LOADER COMMANDS =====

#[tauri::command]
pub async fn get_fabric_versions() -> Result<Vec<FabricLoaderVersion>, String> {
    let installer = FabricInstaller::new(get_meta_dir());
    installer
        .get_loader_versions()
        .await
        .map_err(|e| format!("Failed to fetch Fabric versions: {}", e))
}

#[tauri::command]
pub async fn install_fabric(minecraft_version: String, loader_version: String) -> Result<String, String> {
    // Validate version strings
    if !minecraft_version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid Minecraft version format".to_string());
    }
    if !loader_version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid loader version format".to_string());
    }
    
    let meta_dir = get_meta_dir();
    let installer = FabricInstaller::new(meta_dir);

    installer
        .install_fabric(&minecraft_version, &loader_version)
        .await
        .map_err(|e| format!("Fabric installation failed: {}", e))
}

// ===== INSTANCE MANAGEMENT COMMANDS =====

#[tauri::command]
pub async fn create_instance(
    instance_name: String,
    version: String,
    loader: Option<String>,
    loader_version: Option<String>,
) -> Result<String, String> {
    // Sanitize instance name
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    // Validate version
    if !version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid version format".to_string());
    }
    
    // Validate loader if present
    if let Some(ref loader_type) = loader {
        if loader_type != "fabric" && loader_type != "vanilla" {
            return Err("Invalid loader type".to_string());
        }
    }
    
    // Validate loader version if present
    if let Some(ref lv) = loader_version {
        if !lv.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
            return Err("Invalid loader version format".to_string());
        }
    }

    println!("Creating instance: {}", safe_name);
    println!("Minecraft version: {}", version);
    println!("Loader type: {:?}", loader);
    println!("Loader version: {:?}", loader_version);

    let meta_dir = get_meta_dir();
    let installer = MinecraftInstaller::new(meta_dir.clone());

    println!("Step 1: Verifying/installing Minecraft {}...", version);
    installer
        .install_version(&version)
        .await
        .map_err(|e| {
            let err_msg = format!("Failed to install Minecraft: {}", e);
            println!("ERROR: {}", err_msg);
            err_msg
        })?;
    println!("✓ Minecraft {} is ready", version);

    let final_version = if let Some(loader_type) = &loader {
        if loader_type == "fabric" {
            if let Some(fabric_version) = &loader_version {
                println!("Step 2: Verifying/installing Fabric loader {}...", fabric_version);
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
            println!("Step 2: Using vanilla version (no mod loader)");
            version.clone()
        }
    } else {
        println!("Step 2: Using vanilla version (no mod loader)");
        version.clone()
    };

    println!("Step 3: Creating instance with version: {}", final_version);
    InstanceManager::create(&safe_name, &final_version, loader.clone(), loader_version.clone())
        .map_err(|e| {
            let err_msg = format!("Failed to create instance: {}", e);
            println!("ERROR: {}", err_msg);
            err_msg
        })?;

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
    // Sanitize instance name
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    InstanceManager::delete(&safe_name)
        .map_err(|e| format!("Failed to delete instance: {}", e))?;

    Ok(format!("Successfully deleted instance '{}'", safe_name))
}

#[tauri::command]
pub async fn rename_instance(old_name: String, new_name: String) -> Result<String, String> {
    // Sanitize both names
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
    
    // Rename the directory
    std::fs::rename(&old_path, &new_path)
        .map_err(|e| format!("Failed to rename instance directory: {}", e))?;
    
    // Update instance.json
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

// ===== INSTANCE ICON COMMANDS =====

/// Set a custom icon for an instance from base64 image data
#[tauri::command]
pub async fn set_instance_icon(
    instance_name: String,
    image_data: String, // Base64 encoded image
) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose};
    
    // Sanitize instance name
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    
    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }
    
    // Decode base64 image
    let image_bytes = general_purpose::STANDARD
        .decode(&image_data)
        .map_err(|e| format!("Invalid base64 image data: {}", e))?;
    
    // Validate image size (max 2MB)
    if image_bytes.len() > 2 * 1024 * 1024 {
        return Err("Image too large (max 2MB)".to_string());
    }
    
    // Detect image format
    let format = image::guess_format(&image_bytes)
        .map_err(|e| format!("Invalid image format: {}", e))?;
    
    // Only allow common image formats
    match format {
        image::ImageFormat::Png | 
        image::ImageFormat::Jpeg | 
        image::ImageFormat::WebP => {},
        _ => return Err("Unsupported image format. Use PNG, JPEG, or WebP".to_string()),
    }
    
    // Load and resize image to 256x256
    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| format!("Failed to load image: {}", e))?;
    
    let resized = img.resize_exact(256, 256, image::imageops::FilterType::Lanczos3);
    
    // Save as PNG in instance directory
    let icon_path = instance_dir.join("icon.png");
    resized.save(&icon_path)
        .map_err(|e| format!("Failed to save icon: {}", e))?;
    
    // Update instance.json with icon path
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

/// Remove the custom icon from an instance
#[tauri::command]
pub async fn remove_instance_icon(instance_name: String) -> Result<String, String> {
    // Sanitize instance name
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    
    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }
    
    // Remove icon file if it exists
    let icon_path = instance_dir.join("icon.png");
    if icon_path.exists() {
        std::fs::remove_file(&icon_path)
            .map_err(|e| format!("Failed to remove icon file: {}", e))?;
    }
    
    // Update instance.json
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

/// Get the instance icon as base64 data
#[tauri::command]
pub async fn get_instance_icon(instance_name: String) -> Result<Option<String>, String> {
    use base64::{Engine as _, engine::general_purpose};
    
    // Sanitize instance name
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    
    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }
    
    let icon_path = instance_dir.join("icon.png");
    
    if !icon_path.exists() {
        return Ok(None);
    }
    
    // Security: ensure the icon path is within the instance directory
    let canonical_icon = icon_path.canonicalize()
        .map_err(|_| "Icon file not found".to_string())?;
    
    let canonical_instance = instance_dir.canonicalize()
        .map_err(|_| "Instance directory not found".to_string())?;
    
    if !canonical_icon.starts_with(&canonical_instance) {
        return Err("Invalid icon path".to_string());
    }
    
    // Read and encode icon as base64
    let image_bytes = std::fs::read(&icon_path)
        .map_err(|e| format!("Failed to read icon: {}", e))?;
    
    let base64_data = general_purpose::STANDARD.encode(&image_bytes);
    
    Ok(Some(format!("data:image/png;base64,{}", base64_data)))
}

// ==================== RUST: Update commands.rs ====================

#[tauri::command]
pub async fn duplicate_instance(
    instance_name: String,
    new_name: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // Sanitize both names
    let safe_old_name = sanitize_instance_name(&instance_name)?;
    let safe_new_name = sanitize_instance_name(&new_name)?;
    
    if safe_old_name == safe_new_name {
        return Err("Source and destination names cannot be the same".to_string());
    }
    
    let instances_dir = get_instances_dir();
    let source_path = instances_dir.join(&safe_old_name);
    let dest_path = instances_dir.join(&safe_new_name);
    
    // Check source exists
    if !source_path.exists() {
        return Err(format!("Instance '{}' does not exist", safe_old_name));
    }
    
    // Check destination doesn't exist
    if dest_path.exists() {
        return Err(format!("Instance '{}' already exists", safe_new_name));
    }
    
    println!("Duplicating instance '{}' to '{}'", safe_old_name, safe_new_name);
    
    // Emit initial progress
    let _ = app_handle.emit("duplication-progress", serde_json::json!({
        "instance": safe_new_name,
        "progress": 0,
        "stage": "Calculating size..."
    }));
    
    // Count total files first
    let total_files = count_files(&source_path)
        .map_err(|e| format!("Failed to count files: {}", e))?;
    
    println!("Total files to copy: {}", total_files);
    
    // Copy with progress tracking
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
    
    // Emit 90% progress before updating metadata
    let _ = app_handle.emit("duplication-progress", serde_json::json!({
        "instance": safe_new_name,
        "progress": 90,
        "stage": "Updating metadata..."
    }));
    
    // Update instance.json in the new copy
    let instance_json_path = dest_path.join("instance.json");
    if instance_json_path.exists() {
        let content = std::fs::read_to_string(&instance_json_path)
            .map_err(|e| format!("Failed to read instance.json: {}", e))?;
        
        let mut instance: Instance = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse instance.json: {}", e))?;
        
        // Update the name and reset timestamps
        instance.name = safe_new_name.clone();
        instance.created_at = chrono::Utc::now().to_rfc3339();
        instance.last_played = None;
        
        let updated_json = serde_json::to_string_pretty(&instance)
            .map_err(|e| format!("Failed to serialize instance.json: {}", e))?;
        
        std::fs::write(&instance_json_path, updated_json)
            .map_err(|e| format!("Failed to write instance.json: {}", e))?;
    }
    
    // Emit completion
    let _ = app_handle.emit("duplication-progress", serde_json::json!({
        "instance": safe_new_name,
        "progress": 100,
        "stage": "Complete!"
    }));
    
    println!("✓ Successfully duplicated instance");
    Ok(format!("Successfully duplicated instance to '{}'", safe_new_name))
}

// Helper function to count total files
fn count_files(path: &std::path::Path) -> std::io::Result<usize> {
    use std::fs;
    
    let mut count = 0;
    
    if path.is_file() {
        return Ok(1);
    }
    
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        
        // Skip natives directory
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

// Helper function to recursively copy directories with progress
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
            // Skip natives directory as it will be regenerated on launch
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
            
            // Update progress
            let current = copied_files.fetch_add(1, Ordering::Relaxed) + 1;
            let progress = ((current as f64 / total_files as f64) * 85.0) as u32; // 0-85% for copying
            
            // Emit progress every 10 files or if it's a significant percentage change
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
pub async fn toggle_mod(instance_name: String, filename: String, disable: bool) -> Result<String, String> {
    // Sanitize instance name
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    // Sanitize filename - need to handle both .jar and .jar.disabled
    let safe_filename = if filename.ends_with(".disabled") {
        let base = filename.trim_end_matches(".disabled");
        sanitize_filename(base)?
    } else {
        sanitize_filename(&filename)?
    };
    
    let instance_dir = get_instance_dir(&safe_name);
    let mods_dir = instance_dir.join("mods");
    
    let (old_path, new_filename) = if disable {
        // Disabling: rename from .jar to .jar.disabled
        let old = mods_dir.join(&safe_filename);
        let new_name = format!("{}.disabled", safe_filename);
        (old, new_name)
    } else {
        // Enabling: rename from .jar.disabled to .jar
        let old = mods_dir.join(format!("{}.disabled", safe_filename));
        (old, safe_filename.clone())
    };
    
    let new_path = mods_dir.join(&new_filename);
    
    // Security: ensure paths are within mods_dir
    let canonical_old = old_path.canonicalize()
        .map_err(|_| format!("Mod file not found"))?;
    
    let canonical_mods_dir = mods_dir.canonicalize()
        .map_err(|_| "Mods directory not found".to_string())?;
    
    if !canonical_old.starts_with(&canonical_mods_dir) {
        return Err("Invalid mod path (path traversal detected)".to_string());
    }
    
    // Rename the file
    std::fs::rename(&old_path, &new_path)
        .map_err(|e| format!("Failed to toggle mod: {}", e))?;
    
    let status = if disable { "disabled" } else { "enabled" };
    Ok(format!("Successfully {} mod", status))
}

#[tauri::command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    let mut sys = System::new_all();
    sys.refresh_memory();
    
    let total_mb = sys.total_memory() / 1024 / 1024;
    let available_mb = sys.available_memory() / 1024 / 1024;
    let recommended_max_mb = total_mb * 80 / 100; // 80% of total
    
    Ok(SystemInfo {
        total_memory_mb: total_mb,
        available_memory_mb: available_mb,
        recommended_max_memory_mb: recommended_max_mb,
    })
}

// ===== LAUNCH COMMANDS =====

#[tauri::command]
pub async fn launch_instance(
    instance_name: String,
    username: String,
    uuid: String,
    access_token: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // Sanitize instance name
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    // Validate username
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Invalid username format".to_string());
    }
    
    // Validate UUID
    if !uuid.chars().all(|c| c.is_alphanumeric() || c == '-') || uuid.len() > 36 {
        return Err("Invalid UUID format".to_string());
    }
    
    InstanceManager::launch(&safe_name, &username, &uuid, &access_token, app_handle)
        .map_err(|e| format!("Failed to launch instance: {}", e))?;

    Ok(format!("Launched instance '{}'", safe_name))
}

// ===== LAUNCHER DIRECTORY COMMANDS =====

#[tauri::command]
pub fn get_launcher_directory() -> String {
    get_launcher_dir().to_string_lossy().to_string()
}

#[tauri::command]
pub fn open_instance_folder(instance_name: String) -> Result<String, String> {
    // Sanitize instance name
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);

    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }

    open_folder(instance_dir).map_err(|e| format!("Failed to open folder: {}", e))?;

    Ok(format!("Opened folder for instance '{}'", safe_name))
}

// ===== MOD MANAGEMENT COMMANDS =====

#[tauri::command]
pub async fn get_installed_mods(instance_name: String) -> Result<Vec<ModFile>, String> {
    // Sanitize instance name
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
                
                // Security: ensure path is within mods_dir
                if !path.starts_with(&mods_dir) {
                    continue;
                }
                
                if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        // Include both .jar and .jar.disabled files
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
    
    // Sort mods by filename
    mods.sort_by(|a, b| a.filename.to_lowercase().cmp(&b.filename.to_lowercase()));
    
    Ok(mods)
}

#[tauri::command]
pub async fn delete_mod(instance_name: String, filename: String) -> Result<String, String> {
    // Sanitize instance name and filename
    let safe_name = sanitize_instance_name(&instance_name)?;
    let safe_filename = sanitize_filename(&filename)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    let mods_dir = instance_dir.join("mods");
    let mod_path = mods_dir.join(&safe_filename);
    
    // Security: ensure the resolved path is still within mods_dir
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
    // Sanitize instance name
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

// ===== MODPACK COMMANDS =====

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

/// Get modpack versions
#[tauri::command]
pub async fn get_modpack_versions(
    id_or_slug: String,
    game_version: Option<String>,
) -> Result<Vec<ModrinthVersion>, String> {
    // Validate id/slug
    if !id_or_slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid modpack ID or slug format".to_string());
    }
    
    if id_or_slug.len() > 100 {
        return Err("Modpack ID or slug too long".to_string());
    }
    
    // Validate game version if present
    if let Some(ref version) = game_version {
        if !version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
            return Err("Invalid game version format".to_string());
        }
    }
    
    let client = ModrinthClient::new();
    client
        .get_project_versions(
            &id_or_slug,
            None, // No loader filter for modpacks
            game_version.map(|v| vec![v]),
        )
        .await
        .map_err(|e| format!("Failed to get modpack versions: {}", e))
}

/// Install a modpack by creating an instance and downloading all required files
#[tauri::command]
pub async fn install_modpack(
    modpack_slug: String,
    instance_name: String,
    version_id: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // Validate inputs
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    if !modpack_slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid modpack slug format".to_string());
    }
    
    if !version_id.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err("Invalid version ID format".to_string());
    }
    
    println!("Installing modpack: {}", modpack_slug);
    
    // Emit initial progress
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 0,
        "stage": "Fetching modpack information..."
    }));
    
    // Get the modpack version details
    let client = ModrinthClient::new();
    let versions = client
        .get_project_versions(&modpack_slug, None, None)
        .await
        .map_err(|e| format!("Failed to fetch modpack versions: {}", e))?;
    
    let version = versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| "Version not found".to_string())?;
    
    // Get the game version from the modpack
    let game_version = version.game_versions.first()
        .ok_or_else(|| "No game version found".to_string())?;
    
    // Determine the loader (fabric, forge, etc.)
    let loader = version.loaders.first()
        .map(|l| l.to_lowercase())
        .unwrap_or_else(|| "vanilla".to_string());
    
    println!("Game version: {}, Loader: {}", game_version, loader);
    
    // Emit progress
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 10,
        "stage": format!("Installing Minecraft {}...", game_version)
    }));
    
    // Install Minecraft version
    let meta_dir = get_meta_dir();
    let installer = MinecraftInstaller::new(meta_dir.clone());
    installer
        .install_version(game_version)
        .await
        .map_err(|e| format!("Failed to install Minecraft: {}", e))?;
    
    // Install loader if needed
    let final_version = if loader == "fabric" {
        let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
            "instance": safe_name,
            "progress": 20,
            "stage": "Installing Fabric loader..."
        }));
        
        let fabric_installer = FabricInstaller::new(meta_dir);
        
        // Get latest stable fabric loader
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
            .install_fabric(game_version, &fabric_version.version)
            .await
            .map_err(|e| format!("Failed to install Fabric: {}", e))?
    } else {
        game_version.clone()
    };
    
    // Create the instance
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
    
    // Download modpack file
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 40,
        "stage": "Downloading modpack..."
    }));
    
    // Get the primary modpack file
    let primary_file = version.files.iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| "No modpack file found".to_string())?;
    
    // Download the modpack file
    let temp_dir = std::env::temp_dir();
    let modpack_file = temp_dir.join(&primary_file.filename);
    
    // Validate download URL
    validate_download_url(&primary_file.url)?;
    
    client
        .download_mod_file(&primary_file.url, &modpack_file)
        .await
        .map_err(|e| format!("Failed to download modpack: {}", e))?;
    
    // Extract and process modpack
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 50,
        "stage": "Extracting modpack..."
    }));
    
    // Extract to temporary directory first
    let extract_dir = temp_dir.join(format!("modpack_extract_{}", safe_name));
    if extract_dir.exists() {
        let _ = std::fs::remove_dir_all(&extract_dir);
    }
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Failed to create extraction directory: {}", e))?;
    
    extract_modpack(&modpack_file, &extract_dir)
        .map_err(|e| format!("Failed to extract modpack: {}", e))?;
    
    // Read modrinth.index.json
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
    
    // Copy overrides
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
    
    // Download mods from manifest
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
            
            // Construct destination path
            let dest_path = instance_dir.join(path);
            
            // Ensure parent directory exists
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            }
            
            // Download the file
            validate_download_url(download_url)?;
            client.download_mod_file(download_url, &dest_path)
                .await
                .map_err(|e| format!("Failed to download mod: {}", e))?;
            
            // Update progress
            let progress = 70 + ((idx + 1) * 25 / total_files) as u32;
            let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
                "instance": safe_name,
                "progress": progress,
                "stage": format!("Downloading mods... ({}/{})", idx + 1, total_files)
            }));
        }
    }
    
    // Cleanup
    let _ = std::fs::remove_file(&modpack_file);
    let _ = std::fs::remove_dir_all(&extract_dir);
    
    // Emit completion
    let _ = app_handle.emit("modpack-install-progress", serde_json::json!({
        "instance": safe_name,
        "progress": 100,
        "stage": "Installation complete!"
    }));
    
    Ok(format!("Successfully installed modpack '{}'", safe_name))
}

/// Helper function to copy directory recursively
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

/// Extract a .mrpack file (Modrinth modpack format)
fn extract_modpack(
    archive_path: &std::path::Path,
    dest_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use zip::ZipArchive;
    use std::io::Read;
    
    let file = std::fs::File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };
        
        // Security: ensure path is within dest_dir
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

/// Get modpack manifest info (for displaying details before installation)
#[tauri::command]
pub async fn get_modpack_manifest(
    modpack_slug: String,
    version_id: String,
) -> Result<serde_json::Value, String> {
    // Validate inputs
    if !modpack_slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid modpack slug format".to_string());
    }
    
    if !version_id.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err("Invalid version ID format".to_string());
    }
    
    let client = ModrinthClient::new();
    
    // Get version details
    let versions = client
        .get_project_versions(&modpack_slug, None, None)
        .await
        .map_err(|e| format!("Failed to fetch versions: {}", e))?;
    
    let version = versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| "Version not found".to_string())?;
    
    // Return relevant information
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

// ===== SERVER MANAGEMENT COMMANDS =====

#[tauri::command]
pub async fn get_servers() -> Result<Vec<ServerInfo>, String> {
    let servers_file = get_launcher_dir().join("servers.json");
    
    if !servers_file.exists() {
        return Ok(Vec::new());
    }
    
    let content = std::fs::read_to_string(&servers_file)
        .map_err(|e| format!("Failed to read servers file: {}", e))?;
    
    let servers: Vec<ServerInfo> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse servers file: {}", e))?;
    
    Ok(servers)
}

#[tauri::command]
pub async fn add_server(
    name: String,
    address: String,
    port: u16,
) -> Result<String, String> {
    // Validate inputs
    let safe_name = sanitize_server_name(&name)?;
    validate_server_address(&address)?;
    
    if port == 0 {
        return Err("Port cannot be 0".to_string());
    }
    
    // Load existing servers
    let mut servers = get_servers().await?;
    
    // Check if server with same name already exists
    if servers.iter().any(|s| s.name.to_lowercase() == safe_name.to_lowercase()) {
        return Err(format!("Server '{}' already exists", safe_name));
    }
    
    // Create new server entry
    let new_server = ServerInfo {
        name: safe_name.clone(),
        address,
        port,
        status: "unknown".to_string(),
        players_online: None,
        players_max: None,
        version: None,
        motd: None,
        favicon: None,
        last_checked: None,
    };
    
    servers.push(new_server);
    
    // Save to file
    let servers_file = get_launcher_dir().join("servers.json");
    let json = serde_json::to_string_pretty(&servers)
        .map_err(|e| format!("Failed to serialize servers: {}", e))?;
    
    std::fs::write(&servers_file, json)
        .map_err(|e| format!("Failed to write servers file: {}", e))?;
    
    Ok(format!("Successfully added server '{}'", safe_name))
}

#[tauri::command]
pub async fn delete_server(server_name: String) -> Result<String, String> {
    let safe_name = sanitize_server_name(&server_name)?;
    
    let mut servers = get_servers().await?;
    
    let initial_len = servers.len();
    servers.retain(|s| s.name != safe_name);
    
    if servers.len() == initial_len {
        return Err(format!("Server '{}' not found", safe_name));
    }
    
    // Save updated list
    let servers_file = get_launcher_dir().join("servers.json");
    let json = serde_json::to_string_pretty(&servers)
        .map_err(|e| format!("Failed to serialize servers: {}", e))?;
    
    std::fs::write(&servers_file, json)
        .map_err(|e| format!("Failed to write servers file: {}", e))?;
    
    Ok(format!("Successfully deleted server '{}'", safe_name))
}

#[tauri::command]
pub async fn update_server_status(
    server_name: String,
    status: ServerInfo,
) -> Result<String, String> {
    let safe_name = sanitize_server_name(&server_name)?;
    
    let mut servers = get_servers().await?;
    
    // Find and update the server
    let server = servers.iter_mut()
        .find(|s| s.name == safe_name)
        .ok_or_else(|| format!("Server '{}' not found", safe_name))?;
    
    // Update fields
    server.status = status.status;
    server.players_online = status.players_online;
    server.players_max = status.players_max;
    server.version = status.version;
    server.motd = status.motd;
    server.favicon = status.favicon;
    server.last_checked = Some(chrono::Utc::now().timestamp());
    
    // Save updated list
    let servers_file = get_launcher_dir().join("servers.json");
    let json = serde_json::to_string_pretty(&servers)
        .map_err(|e| format!("Failed to serialize servers: {}", e))?;
    
    std::fs::write(&servers_file, json)
        .map_err(|e| format!("Failed to write servers file: {}", e))?;
    
    Ok(format!("Successfully updated server '{}'", safe_name))
}

// ===== TEMPLATE COMMANDS =====

#[tauri::command]
pub async fn create_template(
    name: String,
    description: Option<String>,
    launcher_settings: Option<crate::models::LauncherSettings>,
    minecraft_options: Option<MinecraftOptions>,
) -> Result<InstanceTemplate, String> {
    // Validate name
    if name.trim().is_empty() {
        return Err("Template name cannot be empty".to_string());
    }
    
    if name.len() > 100 {
        return Err("Template name too long (max 100 characters)".to_string());
    }
    
    // Validate launcher settings if provided
    if let Some(ref settings) = launcher_settings {
        if let Some(ref java_path) = settings.java_path {
            validate_java_path(java_path)?;
        }
        
        validate_memory_allocation(settings.memory_mb as u64)?;
    }
    
    TemplateManager::create_template(
        name,
        description,
        launcher_settings,
        minecraft_options,
    )
    .map_err(|e| format!("Failed to create template: {}", e))
}

#[tauri::command]
pub async fn get_templates() -> Result<Vec<InstanceTemplate>, String> {
    TemplateManager::get_all_templates()
        .map_err(|e| format!("Failed to get templates: {}", e))
}

#[tauri::command]
pub async fn get_template(template_id: String) -> Result<InstanceTemplate, String> {
    // Validate template_id format
    if !template_id.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err("Invalid template ID format".to_string());
    }
    
    TemplateManager::get_template(&template_id)
        .map_err(|e| format!("Failed to get template: {}", e))
}

#[tauri::command]
pub async fn update_template(template: InstanceTemplate) -> Result<String, String> {
    // Validate template name
    if template.name.trim().is_empty() {
        return Err("Template name cannot be empty".to_string());
    }
    
    if template.name.len() > 100 {
        return Err("Template name too long (max 100 characters)".to_string());
    }
    
    // Validate launcher settings if provided
    if let Some(ref settings) = template.launcher_settings {
        if let Some(ref java_path) = settings.java_path {
            validate_java_path(java_path)?;
        }
        
        validate_memory_allocation(settings.memory_mb as u64)?;
    }
    
    TemplateManager::update_template(template)
        .map_err(|e| format!("Failed to update template: {}", e))?;
    
    Ok("Template updated successfully".to_string())
}

#[tauri::command]
pub async fn delete_template(template_id: String) -> Result<String, String> {
    // Validate template_id
    if !template_id.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err("Invalid template ID format".to_string());
    }
    
    TemplateManager::delete_template(&template_id)
        .map_err(|e| format!("Failed to delete template: {}", e))?;
    
    Ok(format!("Template deleted successfully"))
}

#[tauri::command]
pub async fn create_template_from_instance(
    instance_name: String,
    template_name: String,
    description: Option<String>,
) -> Result<InstanceTemplate, String> {
    // Sanitize instance name
    let safe_instance_name = sanitize_instance_name(&instance_name)?;
    
    // Validate template name
    if template_name.trim().is_empty() {
        return Err("Template name cannot be empty".to_string());
    }
    
    if template_name.len() > 100 {
        return Err("Template name too long (max 100 characters)".to_string());
    }
    
    TemplateManager::create_from_instance(&safe_instance_name, template_name, description)
        .map_err(|e| format!("Failed to create template from instance: {}", e))
}

#[tauri::command]
pub async fn apply_template_to_instance(
    template_id: String,
    instance_name: String,
) -> Result<String, String> {
    // Validate template_id
    if !template_id.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err("Invalid template ID format".to_string());
    }
    
    // Sanitize instance name
    let safe_instance_name = sanitize_instance_name(&instance_name)?;
    
    TemplateManager::apply_template_to_instance(&template_id, &safe_instance_name)
        .map_err(|e| format!("Failed to apply template: {}", e))?;
    
    Ok(format!("Template applied successfully to instance '{}'", safe_instance_name))
}

#[tauri::command]
pub async fn create_instance_from_template(
    instance_name: String,
    version: String,
    template_id: String,
    loader: Option<String>,
    loader_version: Option<String>,
) -> Result<String, String> {
    // First create the instance normally
    create_instance(instance_name.clone(), version, loader, loader_version).await?;
    
    // Then apply the template
    apply_template_to_instance(template_id, instance_name).await?;
    
    Ok("Instance created from template successfully".to_string())
}

// ===== DEBUG COMMANDS =====

#[tauri::command]
pub async fn generate_debug_report(version: String) -> Result<String, String> {
    // Validate version
    if !version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid version format".to_string());
    }
    
    Ok(crate::utils::generate_debug_report(&version))
}

#[tauri::command]
pub async fn save_debug_report(version: String) -> Result<String, String> {
    // Validate version
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

// ===== MODRINTH API COMMANDS =====

#[tauri::command]
pub async fn search_mods(
    query: String,
    facets: Option<String>,
    index: Option<String>,
    offset: Option<u32>,
    limit: Option<u32>,
) -> Result<ModrinthSearchResult, String> {
    // Limit query length
    if query.len() > 200 {
        return Err("Search query too long (max 200 characters)".to_string());
    }
    
    // Limit results
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
    // Validate id/slug format
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
pub async fn get_mod_versions(
    id_or_slug: String,
    loaders: Option<Vec<String>>,
    game_versions: Option<Vec<String>>,
) -> Result<Vec<ModrinthVersion>, String> {
    // Validate id/slug
    if !id_or_slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid mod ID or slug format".to_string());
    }
    
    if id_or_slug.len() > 100 {
        return Err("Mod ID or slug too long".to_string());
    }
    
    // Validate loaders
    if let Some(ref loader_list) = loaders {
        for loader in loader_list {
            if !loader.chars().all(|c| c.is_alphanumeric() || c == '-') {
                return Err("Invalid loader name".to_string());
            }
        }
    }
    
    // Validate game versions
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
    // Sanitize instance name and filename
    let safe_name = sanitize_instance_name(&instance_name)?;
    let safe_filename = sanitize_filename(&filename)?;
    
    // Validate download URL
    validate_download_url(&download_url)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    let mods_dir = instance_dir.join("mods");

    if !mods_dir.exists() {
        std::fs::create_dir_all(&mods_dir)
            .map_err(|e| format!("Failed to create mods directory: {}", e))?;
    }

    let destination = mods_dir.join(&safe_filename);
    
    // Security: ensure destination is within mods_dir
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

// ===== SETTINGS COMMANDS =====

#[tauri::command]
pub async fn get_settings() -> Result<LauncherSettings, String> {
    SettingsManager::load()
        .map_err(|e| format!("Failed to load settings: {}", e))
}

#[tauri::command]
pub async fn save_settings(settings: LauncherSettings) -> Result<String, String> {
    // Validate Java path if provided
    if let Some(ref java_path) = settings.java_path {
        validate_java_path(java_path)?;
    }
    
    validate_memory_allocation(settings.memory_mb as u64)?;
    
    SettingsManager::save(&settings)
        .map_err(|e| format!("Failed to save settings: {}", e))?;
    
    Ok("Settings saved successfully".to_string())
}

#[tauri::command]
pub async fn get_instance_settings(instance_name: String) -> Result<Option<LauncherSettings>, String> {
    // Sanitize instance name
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    let instance_json = instance_dir.join("instance.json");
    
    if !instance_json.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }
    
    let content = std::fs::read_to_string(&instance_json)
        .map_err(|e| format!("Failed to read instance data: {}", e))?;
    
    let instance: Instance = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse instance data: {}", e))?;
    
    Ok(instance.settings_override)
}

#[tauri::command]
pub async fn save_instance_settings(
    instance_name: String,
    settings: Option<LauncherSettings>,
) -> Result<String, String> {
    // Sanitize instance name
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    // Validate settings if provided
    if let Some(ref s) = settings {
        if let Some(ref java_path) = s.java_path {
            validate_java_path(java_path)?;
        }
        
        validate_memory_allocation(s.memory_mb as u64)?;
    }
    
    let instance_dir = get_instance_dir(&safe_name);
    let instance_json = instance_dir.join("instance.json");
    
    if !instance_json.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }
    
    let content = std::fs::read_to_string(&instance_json)
        .map_err(|e| format!("Failed to read instance data: {}", e))?;
    
    let mut instance: Instance = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse instance data: {}", e))?;
    
    instance.settings_override = settings;
    
    let updated_json = serde_json::to_string_pretty(&instance)
        .map_err(|e| format!("Failed to serialize instance data: {}", e))?;
    
    std::fs::write(&instance_json, updated_json)
        .map_err(|e| format!("Failed to write instance data: {}", e))?;
    
    Ok("Instance settings saved successfully".to_string())
}

#[tauri::command]
pub async fn detect_java_installations() -> Result<Vec<String>, String> {
    let mut java_paths = Vec::new();
    
    #[cfg(target_os = "windows")]
    {
        // Check common installation directories
        let common_paths = vec![
            "C:\\Program Files\\Java",
            "C:\\Program Files (x86)\\Java",
            "C:\\Program Files\\Eclipse Adoptium",
            "C:\\Program Files\\Microsoft",
            "C:\\Program Files\\Zulu",
            "C:\\Program Files\\Amazon Corretto",
        ];
        
        for base_path in common_paths {
            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        // Look for javaw.exe in bin directory
                        let javaw_path = path.join("bin").join("javaw.exe");
                        if javaw_path.exists() {
                            if let Some(path_str) = javaw_path.to_str() {
                                // Validate before adding
                                if validate_java_path(path_str).is_ok() {
                                    java_paths.push(path_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Check PATH environment variable for javaw.exe
        if let Ok(path_var) = std::env::var("PATH") {
            for path in path_var.split(';') {
                let javaw_path = std::path::PathBuf::from(path).join("javaw.exe");
                if javaw_path.exists() {
                    if let Some(path_str) = javaw_path.to_str() {
                        if validate_java_path(path_str).is_ok() && !java_paths.contains(&path_str.to_string()) {
                            java_paths.push(path_str.to_string());
                        }
                    }
                }
            }
        }
        
        // Check JAVA_HOME
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            let javaw_path = std::path::PathBuf::from(java_home).join("bin").join("javaw.exe");
            if javaw_path.exists() {
                if let Some(path_str) = javaw_path.to_str() {
                    if validate_java_path(path_str).is_ok() && !java_paths.contains(&path_str.to_string()) {
                        java_paths.push(path_str.to_string());
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS Java detection
        let common_paths = vec![
            "/Library/Java/JavaVirtualMachines",
            "/System/Library/Java/JavaVirtualMachines",
        ];
        
        for base_path in common_paths {
            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let java_path = path.join("Contents").join("Home").join("bin").join("java");
                        if java_path.exists() {
                            if let Some(path_str) = java_path.to_str() {
                                if validate_java_path(path_str).is_ok() {
                                    java_paths.push(path_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Check JAVA_HOME
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            let java_path = std::path::PathBuf::from(java_home).join("bin").join("java");
            if java_path.exists() {
                if let Some(path_str) = java_path.to_str() {
                    if validate_java_path(path_str).is_ok() && !java_paths.contains(&path_str.to_string()) {
                        java_paths.push(path_str.to_string());
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        // Linux Java detection
        let common_paths = vec![
            "/usr/lib/jvm",
            "/usr/java",
            "/opt/java",
        ];
        
        for base_path in common_paths {
            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let java_path = path.join("bin").join("java");
                        if java_path.exists() {
                            if let Some(path_str) = java_path.to_str() {
                                if validate_java_path(path_str).is_ok() {
                                    java_paths.push(path_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Check JAVA_HOME
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            let java_path = std::path::PathBuf::from(java_home).join("bin").join("java");
            if java_path.exists() {
                if let Some(path_str) = java_path.to_str() {
                    if validate_java_path(path_str).is_ok() && !java_paths.contains(&path_str.to_string()) {
                        java_paths.push(path_str.to_string());
                    }
                }
            }
        }
    }
    
    // Remove duplicates and sort
    java_paths.sort();
    java_paths.dedup();
    
    Ok(java_paths)
}