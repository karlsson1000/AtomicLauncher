use crate::services::accounts::AccountManager;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

const MINECRAFT_SKIN_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";
const MINECRAFT_SKIN_RESET_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins/active";
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

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

#[derive(Deserialize, Debug)]
struct CapeInfo {
    id: String,
    state: String,
    url: String,
    alias: String,
}

#[derive(Serialize)]
pub struct CurrentSkin {
    pub url: String,
    pub variant: String,
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
        .bearer_auth(&active_account.access_token)
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
    
    let client = reqwest::Client::new();
    
    let response = client
        .delete(MINECRAFT_SKIN_RESET_URL)
        .bearer_auth(&active_account.access_token)
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
    
    let client = reqwest::Client::new();
    
    let response = client
        .get(MINECRAFT_PROFILE_URL)
        .bearer_auth(&active_account.access_token)
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
    
    if let Some(active_skin) = profile.skins.iter().find(|s| s.state == "ACTIVE") {
        Ok(Some(CurrentSkin {
            url: active_skin.url.clone(),
            variant: active_skin.variant.to_lowercase(),
        }))
    } else {
        Ok(None)
    }
}