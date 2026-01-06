use crate::services::accounts::AccountManager;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const MINECRAFT_SKIN_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";
const MINECRAFT_SKIN_RESET_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins/active";
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const MINECRAFT_SESSION_URL: &str = "https://sessionserver.mojang.com/session/minecraft/profile";

#[derive(Serialize, Deserialize)]
pub struct SkinUploadResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Deserialize, Debug)]
struct ProfileResponse {
    id: String,
    name: String,
    skins: Vec<SkinInfo>,
    capes: Option<Vec<CapeInfo>>,
}

#[derive(Deserialize, Debug)]
struct SkinInfo {
    id: String,
    state: String,
    url: String,
    variant: String,
    alias: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct CapeInfo {
    pub id: String,
    pub state: String,
    pub url: String,
    pub alias: String,
}

#[derive(Serialize)]
pub struct CurrentSkin {
    pub url: String,
    pub variant: String,
    pub cape_url: Option<String>,
}

#[derive(Serialize)]
pub struct UserCapesResponse {
    pub capes: Vec<CapeInfo>,
}

#[derive(Deserialize, Debug)]
struct SessionProfileResponse {
    id: String,
    name: String,
    properties: Vec<ProfileProperty>,
}

#[derive(Deserialize, Debug)]
struct ProfileProperty {
    name: String,
    value: String,
}

#[derive(Deserialize, Debug)]
struct TexturesData {
    timestamp: u64,
    #[serde(rename = "profileId")]
    profile_id: String,
    #[serde(rename = "profileName")]
    profile_name: String,
    textures: Textures,
}

#[derive(Deserialize, Debug)]
struct Textures {
    #[serde(rename = "SKIN")]
    skin: Option<SkinTexture>,
    #[serde(rename = "CAPE")]
    cape: Option<CapeTexture>,
}

#[derive(Deserialize, Debug)]
struct SkinTexture {
    url: String,
    metadata: Option<SkinMetadata>,
}

#[derive(Deserialize, Debug)]
struct SkinMetadata {
    model: Option<String>,
}

#[derive(Deserialize, Debug)]
struct CapeTexture {
    url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecentSkin {
    pub url: String,
    pub variant: String,
    pub timestamp: u64,
}

/// Helper function to get the recent skins file path
fn get_recent_skins_path(account_uuid: &str) -> Result<PathBuf, String> {
    let app_data_dir = dirs::data_dir()
        .ok_or_else(|| "Failed to get app data directory".to_string())?;
    
    let launcher_dir = app_data_dir.join("AtomicLauncher");
    let skins_dir = launcher_dir.join("recent_skins");
    
    // Create directory if it doesn't exist
    if !skins_dir.exists() {
        fs::create_dir_all(&skins_dir)
            .map_err(|e| format!("Failed to create skins directory: {}", e))?;
    }
    
    Ok(skins_dir.join(format!("{}.json", account_uuid)))
}

/// Load recent skins for an account
#[tauri::command]
pub async fn load_recent_skins(account_uuid: String) -> Result<Vec<RecentSkin>, String> {
    let file_path = get_recent_skins_path(&account_uuid)?;
    
    if !file_path.exists() {
        return Ok(Vec::new());
    }
    
    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read recent skins file: {}", e))?;
    
    let skins: Vec<RecentSkin> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse recent skins: {}", e))?;
    
    Ok(skins)
}

/// Save a recent skin for an account
#[tauri::command]
pub async fn save_recent_skin(
    account_uuid: String,
    skin_url: String,
    variant: String,
) -> Result<(), String> {
    let file_path = get_recent_skins_path(&account_uuid)?;
    
    // Load existing skins
    let mut skins = if file_path.exists() {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read recent skins file: {}", e))?;
        
        serde_json::from_str::<Vec<RecentSkin>>(&content)
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    
    // Remove the skin if it already exists
    skins.retain(|s| s.url != skin_url);
    
    // Add the new skin at the beginning
    let new_skin = RecentSkin {
        url: skin_url,
        variant,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };
    
    skins.insert(0, new_skin);
    
    // Keep only the last 3 skins
    skins.truncate(3);
    
    // Save to file
    let json = serde_json::to_string_pretty(&skins)
        .map_err(|e| format!("Failed to serialize recent skins: {}", e))?;
    
    fs::write(&file_path, json)
        .map_err(|e| format!("Failed to write recent skins file: {}", e))?;
    
    Ok(())
}

/// Upload a skin to Minecraft
#[tauri::command]
pub async fn upload_skin(
    skin_data: String,
    variant: String,
) -> Result<String, String> {
    if variant != "classic" && variant != "slim" {
        return Err("Invalid skin variant. Must be 'classic' or 'slim'".to_string());
    }
    
    let active_account = AccountManager::get_active_account()
        .map_err(|e| format!("Failed to get active account: {}", e))?
        .ok_or_else(|| "No active account. Please sign in first.".to_string())?;
    
    // Get a fresh token
    let access_token = AccountManager::get_valid_token(&active_account.uuid)
        .await
        .map_err(|e| format!("Failed to get valid token: {}", e))?;
    
    let image_bytes = general_purpose::STANDARD
        .decode(&skin_data)
        .map_err(|e| format!("Invalid base64 image data: {}", e))?;
    
    if image_bytes.len() > 1024 * 1024 {
        return Err("Skin image too large (max 1MB)".to_string());
    }
    
    let format = image::guess_format(&image_bytes)
        .map_err(|e| format!("Invalid image format: {}", e))?;
    
    if format != image::ImageFormat::Png {
        return Err("Skin must be a PNG image".to_string());
    }
    
    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| format!("Failed to load image: {}", e))?;
    
    let (width, height) = (img.width(), img.height());
    if !((width == 64 && height == 64) || (width == 64 && height == 32)) {
        return Err(format!("Invalid skin dimensions ({}x{}). Must be 64x64 or 64x32", width, height));
    }
    
    let client = reqwest::Client::new();
    
    let part = reqwest::multipart::Part::bytes(image_bytes)
        .file_name("skin.png")
        .mime_str("image/png")
        .map_err(|e| format!("Failed to create form part: {}", e))?;
    
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("variant", variant);
    
    let response = client
        .post(MINECRAFT_SKIN_URL)
        .bearer_auth(&access_token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to upload skin: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Skin upload failed ({}): {}", status, error_text));
    }
    
    Ok("Skin uploaded successfully".to_string())
}

/// Reset skin to default (Steve/Alex)
#[tauri::command]
pub async fn reset_skin() -> Result<String, String> {
    let active_account = AccountManager::get_active_account()
        .map_err(|e| format!("Failed to get active account: {}", e))?
        .ok_or_else(|| "No active account. Please sign in first.".to_string())?;
    
    // Get a fresh token
    let access_token = AccountManager::get_valid_token(&active_account.uuid)
        .await
        .map_err(|e| format!("Failed to get valid token: {}", e))?;
    
    let client = reqwest::Client::new();
    
    let response = client
        .delete(MINECRAFT_SKIN_RESET_URL)
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to reset skin: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Skin reset failed ({}): {}", status, error_text));
    }
    
    Ok("Skin reset to default successfully".to_string())
}

/// Get current skin URL and variant from Minecraft profile
#[tauri::command]
pub async fn get_current_skin() -> Result<Option<CurrentSkin>, String> {
    let active_account = AccountManager::get_active_account()
        .map_err(|e| format!("Failed to get active account: {}", e))?
        .ok_or_else(|| "No active account. Please sign in first.".to_string())?;
    
    // Get a fresh token
    let access_token = AccountManager::get_valid_token(&active_account.uuid)
        .await
        .map_err(|e| format!("Failed to get valid token: {}", e))?;
    
    let client = reqwest::Client::new();
    
    // Get profile from Microsoft API for skin info
    let response = client
        .get(MINECRAFT_PROFILE_URL)
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch profile: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Failed to get profile ({}): {}", status, error_text));
    }
    
    let profile: ProfileResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse profile response: {}", e))?;
    
    // Get cape from session server
    let cape_url = get_player_cape(&profile.id).await.ok();
    
    if let Some(active_skin) = profile.skins.iter().find(|s| s.state == "ACTIVE") {
        Ok(Some(CurrentSkin {
            url: active_skin.url.clone(),
            variant: active_skin.variant.to_lowercase(),
            cape_url,
        }))
    } else {
        Ok(None)
    }
}

/// Get user's capes from Microsoft profile
#[tauri::command]
pub async fn get_user_capes() -> Result<UserCapesResponse, String> {
    let active_account = AccountManager::get_active_account()
        .map_err(|e| format!("Failed to get active account: {}", e))?
        .ok_or_else(|| "No active account. Please sign in first.".to_string())?;
    
    // Get a fresh token
    let access_token = AccountManager::get_valid_token(&active_account.uuid)
        .await
        .map_err(|e| format!("Failed to get valid token: {}", e))?;
    
    let client = reqwest::Client::new();
    
    let response = client
        .get(MINECRAFT_PROFILE_URL)
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch profile: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Failed to get profile ({}): {}", status, error_text));
    }
    
    let profile: ProfileResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse profile response: {}", e))?;
    
    let capes = profile.capes.unwrap_or_default();
    
    Ok(UserCapesResponse { capes })
}

/// Helper function to get player's cape from session server
async fn get_player_cape(uuid: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    
    // Remove dashes from UUID for session server
    let uuid_no_dashes = uuid.replace("-", "");
    let url = format!("{}/{}", MINECRAFT_SESSION_URL, uuid_no_dashes);
    
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch session profile: {}", e))?;
    
    if !response.status().is_success() {
        return Err("Failed to get session profile".to_string());
    }
    
    let session_profile: SessionProfileResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse session profile: {}", e))?;
    
    // Find textures property
    let textures_property = session_profile
        .properties
        .iter()
        .find(|p| p.name == "textures")
        .ok_or_else(|| "No textures property found".to_string())?;
    
    // Decode base64 value
    let decoded = general_purpose::STANDARD
        .decode(&textures_property.value)
        .map_err(|e| format!("Failed to decode textures: {}", e))?;
    
    let textures_str = String::from_utf8(decoded)
        .map_err(|e| format!("Invalid UTF-8 in textures: {}", e))?;
    
    let textures_data: TexturesData = serde_json::from_str(&textures_str)
        .map_err(|e| format!("Failed to parse textures JSON: {}", e))?;
    
    // Extract cape URL if present
    textures_data
        .textures
        .cape
        .map(|cape| cape.url)
        .ok_or_else(|| "No cape found".to_string())
}

/// Equip a cape by its ID
#[tauri::command]
pub async fn equip_cape(cape_id: String) -> Result<String, String> {
    let active_account = AccountManager::get_active_account()
        .map_err(|e| format!("Failed to get active account: {}", e))?
        .ok_or_else(|| "No active account. Please sign in first.".to_string())?;
    
    // Get a fresh token
    let access_token = AccountManager::get_valid_token(&active_account.uuid)
        .await
        .map_err(|e| format!("Failed to get valid token: {}", e))?;
    
    let client = reqwest::Client::new();
    
    let url = format!("https://api.minecraftservices.com/minecraft/profile/capes/active");
    
    let body = serde_json::json!({
        "capeId": cape_id
    });
    
    let response = client
        .put(&url)
        .bearer_auth(&access_token)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to equip cape: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Cape equip failed ({}): {}", status, error_text));
    }
    
    Ok("Cape equipped successfully".to_string())
}

/// Remove the active cape
#[tauri::command]
pub async fn remove_cape() -> Result<String, String> {
    let active_account = AccountManager::get_active_account()
        .map_err(|e| format!("Failed to get active account: {}", e))?
        .ok_or_else(|| "No active account. Please sign in first.".to_string())?;
    
    // Get a fresh token
    let access_token = AccountManager::get_valid_token(&active_account.uuid)
        .await
        .map_err(|e| format!("Failed to get valid token: {}", e))?;
    
    let client = reqwest::Client::new();
    
    let url = "https://api.minecraftservices.com/minecraft/profile/capes/active";
    
    let response = client
        .delete(url)
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| format!("Failed to remove cape: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Cape removal failed ({}): {}", status, error_text));
    }
    
    Ok("Cape removed successfully".to_string())
}