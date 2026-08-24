use crate::services::installer::MinecraftInstaller;
use crate::services::fabric::FabricInstaller;
use crate::services::neoforge::NeoForgeInstaller;
use crate::services::forge::ForgeInstaller;
use crate::models::{FabricLoaderVersion, NeoForgeVersion, ForgeVersion};
use crate::utils::get_meta_dir;

#[tauri::command]
pub async fn get_minecraft_versions() -> Result<Vec<String>, String> {
    let installer = MinecraftInstaller::new(get_meta_dir())
        .map_err(|e| e.to_string())?;
    installer
        .get_versions()
        .await
        .map_err(|e| format!("Failed to fetch versions: {}", e))
}

#[tauri::command]
pub async fn get_minecraft_versions_with_metadata() -> Result<Vec<crate::models::MinecraftVersion>, String> {
    let installer = MinecraftInstaller::new(get_meta_dir())
        .map_err(|e| e.to_string())?;
    installer
        .get_versions_with_metadata()
        .await
        .map_err(|e| format!("Failed to fetch versions: {}", e))
}

#[tauri::command]
pub async fn get_minecraft_versions_by_type(version_type: String) -> Result<Vec<String>, String> {
    let valid_types = ["release", "snapshot"];
    if !valid_types.contains(&version_type.as_str()) {
        return Err(format!("Invalid version type. Must be one of: {}", valid_types.join(", ")));
    }
    
    let installer = MinecraftInstaller::new(get_meta_dir())
        .map_err(|e| e.to_string())?;
    installer
        .get_versions_by_type(&version_type)
        .await
        .map_err(|e| format!("Failed to fetch versions: {}", e))
}

#[tauri::command]
pub async fn get_supported_game_versions() -> Result<Vec<String>, String> {
    let installer = FabricInstaller::new(get_meta_dir())
        .map_err(|e| e.to_string())?;
    installer
        .get_supported_game_versions()
        .await
        .map_err(|e| format!("Failed to fetch Fabric supported versions: {}", e))
}

#[tauri::command]
pub async fn get_neoforge_supported_game_versions() -> Result<Vec<String>, String> {
    let installer = NeoForgeInstaller::new(get_meta_dir())
        .map_err(|e| e.to_string())?;
    installer
        .get_supported_game_versions()
        .await
        .map_err(|e| format!("Failed to fetch NeoForge supported versions: {}", e))
}

#[tauri::command]
pub async fn get_fabric_versions() -> Result<Vec<FabricLoaderVersion>, String> {
    let installer = FabricInstaller::new(get_meta_dir())
        .map_err(|e| e.to_string())?;
    installer
        .get_loader_versions()
        .await
        .map_err(|e| format!("Failed to fetch Fabric versions: {}", e))
}

#[tauri::command]
pub async fn get_neoforge_versions() -> Result<Vec<NeoForgeVersion>, String> {
    let installer = NeoForgeInstaller::new(get_meta_dir())
        .map_err(|e| e.to_string())?;
    installer
        .get_loader_versions()
        .await
        .map_err(|e| format!("Failed to fetch NeoForge versions: {}", e))
}

#[tauri::command]
pub async fn get_forge_versions() -> Result<Vec<ForgeVersion>, String> {
    let installer = ForgeInstaller::new(get_meta_dir())
        .map_err(|e| e.to_string())?;
    installer
        .get_loader_versions()
        .await
        .map_err(|e| format!("Failed to fetch Forge versions: {}", e))
}

#[tauri::command]
pub async fn get_forge_supported_game_versions() -> Result<Vec<String>, String> {
    let installer = ForgeInstaller::new(get_meta_dir())
        .map_err(|e| e.to_string())?;
    installer
        .get_supported_game_versions()
        .await
        .map_err(|e| format!("Failed to fetch Forge supported versions: {}", e))
}