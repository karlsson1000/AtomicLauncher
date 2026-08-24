use crate::auth::Authenticator;
use crate::services::accounts::AccountManager;
use crate::models::{AppConfig, AccountInfo};
use tauri::Manager;

fn make_authenticator(client_id: &str) -> Result<Authenticator, String> {
    Authenticator::new(client_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_accounts() -> Result<Vec<AccountInfo>, String> {
    AccountManager::get_all_accounts()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn switch_account(uuid: String) -> Result<(), String> {
    crate::commands::validation::validate_uuid(&uuid)?;
    AccountManager::set_active_account(&uuid)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_account(uuid: String) -> Result<(), String> {
    crate::commands::validation::validate_uuid(&uuid)?;
    AccountManager::remove_account(&uuid)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn microsoft_login_and_store(app_handle: tauri::AppHandle) -> Result<AccountInfo, String> {
    let config = app_handle.state::<AppConfig>();
    let auth_response = make_authenticator(&config.microsoft_client_id)?
        .authenticate()
        .await
        .map_err(|e| e.to_string())?;

    let account_exists = AccountManager::account_exists(&auth_response.uuid)
        .map_err(|e| e.to_string())?;

    if account_exists {
        AccountManager::update_account_tokens(
            &auth_response.uuid,
            auth_response.access_token.clone(),
            auth_response.refresh_token.clone(),
            auth_response.token_expiry,
        )
        .map_err(|e| e.to_string())?;

        AccountManager::set_active_account(&auth_response.uuid)
            .map_err(|e| e.to_string())?;
    } else {
        AccountManager::add_account(
            auth_response.uuid.clone(),
            auth_response.username.clone(),
            auth_response.access_token.clone(),
            auth_response.refresh_token.clone(),
            auth_response.token_expiry,
        )
        .map_err(|e| e.to_string())?;

        AccountManager::set_active_account(&auth_response.uuid)
            .map_err(|e| e.to_string())?;
    }

    AccountManager::get_all_accounts()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|acc| acc.uuid == auth_response.uuid)
        .ok_or_else(|| "Account not found".to_string())
}