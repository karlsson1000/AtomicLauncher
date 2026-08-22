use crate::models::{AppConfig, Friend, FriendRequest, FriendStatus};
use crate::services::friends::DatabaseService;
use crate::services::accounts::AccountManager;
use tauri::Manager;

fn get_database_service(config: &AppConfig) -> DatabaseService {
    DatabaseService::new(&config.database_url, &config.database_key)
}

async fn require_session(
    app_handle: &tauri::AppHandle,
    account_uuid: &str,
) -> Result<String, String> {
    crate::services::friends::get_session_token(app_handle, account_uuid).await
}

#[tauri::command]
pub async fn send_friend_request(username: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account")?;

    let config = app_handle.state::<AppConfig>();
    let service = get_database_service(&config);
    drop(config);

    let bearer = require_session(&app_handle, &account.uuid).await?;

    service.register_user(&bearer, &account.uuid, &account.username).await?;
    service.send_friend_request(&bearer, &username).await
}

#[tauri::command]
pub async fn get_friend_requests(app_handle: tauri::AppHandle) -> Result<Vec<FriendRequest>, String> {
    let account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account")?;

    let bearer = require_session(&app_handle, &account.uuid).await?;
    get_database_service(&app_handle.state::<AppConfig>())
        .get_friend_requests(&bearer, &account.uuid)
        .await
}

#[tauri::command]
pub async fn accept_friend_request(request_id: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account")?;

    let bearer = require_session(&app_handle, &account.uuid).await?;
    get_database_service(&app_handle.state::<AppConfig>())
        .accept_friend_request(&bearer, &request_id)
        .await
}

#[tauri::command]
pub async fn reject_friend_request(request_id: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account")?;

    let bearer = require_session(&app_handle, &account.uuid).await?;
    get_database_service(&app_handle.state::<AppConfig>())
        .reject_friend_request(&bearer, &request_id)
        .await
}

#[tauri::command]
pub async fn get_friends(app_handle: tauri::AppHandle) -> Result<Vec<Friend>, String> {
    let account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account")?;

    let bearer = require_session(&app_handle, &account.uuid).await?;
    get_database_service(&app_handle.state::<AppConfig>())
        .get_friends(&bearer, &account.uuid)
        .await
}

#[tauri::command]
pub async fn remove_friend(friend_uuid: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account")?;

    let bearer = require_session(&app_handle, &account.uuid).await?;
    get_database_service(&app_handle.state::<AppConfig>())
        .remove_friend(&bearer, &friend_uuid)
        .await
}

#[tauri::command]
pub async fn update_user_status(status: String, current_instance: Option<String>, app_handle: tauri::AppHandle) -> Result<(), String> {
    let friend_status = match status.as_str() {
        "online" => FriendStatus::Online,
        "ingame" => FriendStatus::InGame,
        "offline" => FriendStatus::Offline,
        _ => return Err("Invalid status".to_string()),
    };

    let account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account")?;

    crate::services::friends::set_status_for_account(&app_handle, &account.uuid, friend_status, current_instance)
        .await
}

#[tauri::command]
pub async fn register_user_in_friends_system(app_handle: tauri::AppHandle) -> Result<(), String> {
    let account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account")?;

    let bearer = require_session(&app_handle, &account.uuid).await?;
    get_database_service(&app_handle.state::<AppConfig>())
        .register_user(&bearer, &account.uuid, &account.username)
        .await
}
