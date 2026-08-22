use crate::services::accounts::AccountManager;
use crate::models::AppConfig;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

const MINECRAFT_SKIN_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";
const MINECRAFT_SKIN_RESET_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins/active";
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const MINECRAFT_SESSION_URL: &str = "https://sessionserver.mojang.com/session/minecraft/profile";

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct ProfileResponse {
    id: String,
    name: String,
    skins: Vec<SkinInfo>,
    capes: Option<Vec<CapeInfo>>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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

fn get_recent_skins_path(account_uuid: &str) -> Result<PathBuf, String> {
    crate::commands::validation::validate_uuid(account_uuid)?;
    let launcher_dir = crate::utils::get_launcher_dir();
    let skins_dir = launcher_dir.join("recent_skins");
    
    if !skins_dir.exists() {
        fs::create_dir_all(&skins_dir)
            .map_err(|e| e.to_string())?;
    }
    
    Ok(skins_dir.join(format!("{}.json", account_uuid)))
}

#[tauri::command]
pub async fn load_recent_skins(account_uuid: String) -> Result<Vec<RecentSkin>, String> {
    let file_path = get_recent_skins_path(&account_uuid)?;
    
    if !file_path.exists() {
        return Ok(Vec::new());
    }
    
    let content = fs::read_to_string(&file_path)
        .map_err(|e| e.to_string())?;
    
    let skins: Vec<RecentSkin> = serde_json::from_str(&content)
        .map_err(|e| e.to_string())?;
    
    Ok(skins.into_iter().map(|s| RecentSkin {
        url: s.url.replace("http://", "https://"),
        ..s
    }).collect())
}

#[tauri::command]
pub async fn save_recent_skin(
    account_uuid: String,
    skin_url: String,
    variant: String,
) -> Result<(), String> {
    let file_path = get_recent_skins_path(&account_uuid)?;
    
    let mut skins = if file_path.exists() {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| e.to_string())?;
        
        serde_json::from_str::<Vec<RecentSkin>>(&content)
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    
    skins.retain(|s| s.url != skin_url);
    
    let new_skin = RecentSkin {
        url: skin_url.replace("http://", "https://"),
        variant,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };
    
    skins.insert(0, new_skin);
    skins.truncate(3);
    
    let json = serde_json::to_string_pretty(&skins)
        .map_err(|e| e.to_string())?;
    
    fs::write(&file_path, json)
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn upload_skin(
    skin_data: String,
    variant: String,
    app_handle: tauri::AppHandle,
) -> Result<CurrentSkin, String> {
    if variant != "classic" && variant != "slim" {
        return Err("Invalid skin variant. Must be 'classic' or 'slim'".to_string());
    }
    
    let config = app_handle.state::<AppConfig>();
    
    let active_account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account".to_string())?;
    
    let access_token = AccountManager::get_valid_token(&active_account.uuid, &config.microsoft_client_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let image_bytes = general_purpose::STANDARD
        .decode(&skin_data)
        .map_err(|e| e.to_string())?;
    
    if image_bytes.len() > 1024 * 1024 {
        return Err("Skin image too large (max 1MB)".to_string());
    }
    
    let format = image::guess_format(&image_bytes)
        .map_err(|e| e.to_string())?;
    
    if format != image::ImageFormat::Png {
        return Err("Skin must be a PNG image".to_string());
    }
    
    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| e.to_string())?;
    
    let (width, height) = (img.width(), img.height());
    if !((width == 64 && height == 64) || (width == 64 && height == 32)) {
        return Err(format!("Invalid skin dimensions ({}x{}). Must be 64x64 or 64x32", width, height));
    }
    
    let client = crate::utils::http::get_client();
    
    let part = reqwest::multipart::Part::bytes(image_bytes)
        .file_name("skin.png")
        .mime_str("image/png")
        .map_err(|e| e.to_string())?;
    
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("variant", variant.clone());
    
    let response = client
        .post(MINECRAFT_SKIN_URL)
        .bearer_auth(&access_token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Skin upload failed ({}): {}", status, error_text));
    }
    
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let uuid_no_dashes = active_account.uuid.replace("-", "");
    let session_url = format!("{}/{}", MINECRAFT_SESSION_URL, uuid_no_dashes);
    
    let session_response = client
        .get(&session_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if session_response.status().is_success() {
        let session_profile: SessionProfileResponse = session_response
            .json()
            .await
            .map_err(|e| e.to_string())?;
        
        if let Some(textures_property) = session_profile.properties.iter().find(|p| p.name == "textures") {
            let decoded = general_purpose::STANDARD
                .decode(&textures_property.value)
                .map_err(|e| e.to_string())?;
            
            let textures_str = String::from_utf8(decoded)
                .map_err(|e| e.to_string())?;
            
            let textures_data: TexturesData = serde_json::from_str(&textures_str)
                .map_err(|e| e.to_string())?;
            
            if let Some(skin_texture) = textures_data.textures.skin {
                let skin_variant = skin_texture.metadata
                    .and_then(|m| m.model)
                    .unwrap_or_else(|| "classic".to_string());
                
                let cape_url = textures_data.textures.cape.map(|c| c.url);
                
                return Ok(CurrentSkin {
                    url: skin_texture.url.replace("http://", "https://"),
                    variant: skin_variant.to_lowercase(),
                    cape_url: cape_url.map(|u| u.replace("http://", "https://")),
                });
            }
        }
    }

    Ok(CurrentSkin {
        url: String::new(),
        variant: variant.to_lowercase(),
        cape_url: None,
    })
}

#[tauri::command]
pub async fn reset_skin(app_handle: tauri::AppHandle) -> Result<(), String> {
    let config = app_handle.state::<AppConfig>();
    
    let active_account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account".to_string())?;
    
    let access_token = AccountManager::get_valid_token(&active_account.uuid, &config.microsoft_client_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let client = crate::utils::http::get_client();

    let response = client
        .delete(MINECRAFT_SKIN_RESET_URL)
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Skin reset failed ({}): {}", status, error_text));
    }
    
    Ok(())
}

#[tauri::command]
pub async fn get_current_skin(app_handle: tauri::AppHandle) -> Result<Option<CurrentSkin>, String> {
    let config = app_handle.state::<AppConfig>();
    
    let active_account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account".to_string())?;
    
    let access_token = AccountManager::get_valid_token(&active_account.uuid, &config.microsoft_client_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let client = crate::utils::http::get_client();
    
    let response = client
        .get(MINECRAFT_PROFILE_URL)
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Failed to get profile ({}): {}", status, error_text));
    }
    
    let profile: ProfileResponse = response
        .json()
        .await
        .map_err(|e| e.to_string())?;
    
    let cape_url = get_player_cape(&profile.id).await.ok();
    
    if let Some(active_skin) = profile.skins.iter().find(|s| s.state == "ACTIVE") {
        Ok(Some(CurrentSkin {
            url: active_skin.url.replace("http://", "https://"),
            variant: active_skin.variant.to_lowercase(),
            cape_url,
        }))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn get_user_capes(app_handle: tauri::AppHandle) -> Result<UserCapesResponse, String> {
    let config = app_handle.state::<AppConfig>();
    
    let active_account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account".to_string())?;
    
    let access_token = AccountManager::get_valid_token(&active_account.uuid, &config.microsoft_client_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let client = crate::utils::http::get_client();
    
    let response = client
        .get(MINECRAFT_PROFILE_URL)
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Failed to get profile ({}): {}", status, error_text));
    }
    
    let profile: ProfileResponse = response
        .json()
        .await
        .map_err(|e| e.to_string())?;
    
    let capes = profile.capes.unwrap_or_default().into_iter().map(|c| CapeInfo {
        url: c.url.replace("http://", "https://"),
        ..c
    }).collect();
    
    Ok(UserCapesResponse { capes })
}

async fn get_player_cape(uuid: &str) -> Result<String, String> {
    let client = crate::utils::http::get_client();
    
    let uuid_no_dashes = uuid.replace("-", "");
    let url = format!("{}/{}", MINECRAFT_SESSION_URL, uuid_no_dashes);
    
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        return Err("Failed to get session profile".to_string());
    }
    
    let session_profile: SessionProfileResponse = response
        .json()
        .await
        .map_err(|e| e.to_string())?;
    
    let textures_property = session_profile
        .properties
        .iter()
        .find(|p| p.name == "textures")
        .ok_or("No textures property found".to_string())?;
    
    let decoded = general_purpose::STANDARD
        .decode(&textures_property.value)
        .map_err(|e| e.to_string())?;
    
    let textures_str = String::from_utf8(decoded)
        .map_err(|e| e.to_string())?;
    
    let textures_data: TexturesData = serde_json::from_str(&textures_str)
        .map_err(|e| e.to_string())?;
    
    textures_data
        .textures
        .cape
        .map(|cape| cape.url.replace("http://", "https://"))
        .ok_or("No cape found".to_string())
}

#[tauri::command]
pub async fn equip_cape(cape_id: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let config = app_handle.state::<AppConfig>();
    
    let active_account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account".to_string())?;
    
    let access_token = AccountManager::get_valid_token(&active_account.uuid, &config.microsoft_client_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let client = crate::utils::http::get_client();
    
    let url = "https://api.minecraftservices.com/minecraft/profile/capes/active";
    
    let body = serde_json::json!({
        "capeId": cape_id
    });
    
    let response = client
        .put(url)
        .bearer_auth(&access_token)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Cape equip failed ({}): {}", status, error_text));
    }
    
    Ok(())
}

#[tauri::command]
pub async fn remove_cape(app_handle: tauri::AppHandle) -> Result<(), String> {
    let config = app_handle.state::<AppConfig>();
    
    let active_account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account".to_string())?;
    
    let access_token = AccountManager::get_valid_token(&active_account.uuid, &config.microsoft_client_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let client = crate::utils::http::get_client();
    
    let url = "https://api.minecraftservices.com/minecraft/profile/capes/active";
    
    let response = client
        .delete(url)
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Cape removal failed ({}): {}", status, error_text));
    }
    
    Ok(())
}