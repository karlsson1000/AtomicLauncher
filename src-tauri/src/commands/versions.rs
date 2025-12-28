use crate::services::installer::MinecraftInstaller;
use crate::services::fabric::FabricInstaller;
use crate::models::FabricLoaderVersion;
use crate::utils::get_meta_dir;

#[tauri::command]
pub async fn get_minecraft_versions() -> Result<Vec<String>, String> {
    let installer = MinecraftInstaller::new(get_meta_dir());
    installer
        .get_versions()
        .await
        .map_err(|e| format!("Failed to fetch versions: {}", e))
}

#[tauri::command]
pub async fn get_minecraft_versions_with_metadata() -> Result<Vec<crate::models::MinecraftVersion>, String> {
    let installer = MinecraftInstaller::new(get_meta_dir());
    installer
        .get_versions_with_metadata()
        .await
        .map_err(|e| format!("Failed to fetch versions: {}", e))
}

#[tauri::command]
pub async fn get_minecraft_versions_by_type(version_type: String) -> Result<Vec<String>, String> {
    let valid_types = ["release", "snapshot", "old_beta", "old_alpha"];
    if !valid_types.contains(&version_type.as_str()) {
        return Err(format!("Invalid version type. Must be one of: {}", valid_types.join(", ")));
    }
    
    let installer = MinecraftInstaller::new(get_meta_dir());
    installer
        .get_versions_by_type(&version_type)
        .await
        .map_err(|e| format!("Failed to fetch versions: {}", e))
}

#[tauri::command]
pub async fn install_minecraft(version: String) -> Result<String, String> {
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
    if !version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
        return Err("Invalid version format".to_string());
    }
    
    let meta_dir = get_meta_dir();
    let installer = MinecraftInstaller::new(meta_dir);
    Ok(installer.check_version_installed(&version))
}

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