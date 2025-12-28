use crate::auth::Authenticator;
use crate::services::accounts::AccountManager;
use crate::models::{AuthResponse, AccountInfo};

#[tauri::command]
pub async fn microsoft_login() -> Result<AuthResponse, String> {
    let authenticator = Authenticator::new()
        .map_err(|e| format!("Failed to initialize authenticator: {}", e))?;
    
    authenticator
        .authenticate()
        .await
        .map_err(|e| format!("Authentication failed: {}", e))
}

#[tauri::command]
pub async fn get_accounts() -> Result<Vec<AccountInfo>, String> {
    AccountManager::get_all_accounts()
        .map_err(|e| format!("Failed to get accounts: {}", e))
}

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

#[tauri::command]
pub async fn switch_account(uuid: String) -> Result<String, String> {
    if !uuid.chars().all(|c| c.is_alphanumeric() || c == '-') || uuid.len() > 36 {
        return Err("Invalid UUID format".to_string());
    }
    
    AccountManager::set_active_account(&uuid)
        .map_err(|e| format!("Failed to switch account: {}", e))?;
    
    Ok(format!("Switched to account {}", uuid))
}

#[tauri::command]
pub async fn remove_account(uuid: String) -> Result<String, String> {
    if !uuid.chars().all(|c| c.is_alphanumeric() || c == '-') || uuid.len() > 36 {
        return Err("Invalid UUID format".to_string());
    }
    
    AccountManager::remove_account(&uuid)
        .map_err(|e| format!("Failed to remove account: {}", e))?;
    
    Ok(format!("Account {} removed", uuid))
}

#[tauri::command]
pub async fn microsoft_login_and_store() -> Result<AccountInfo, String> {
    let authenticator = crate::auth::Authenticator::new()
        .map_err(|e| format!("Failed to initialize authenticator: {}", e))?;
    
    let auth_response = authenticator
        .authenticate()
        .await
        .map_err(|e| format!("Authentication failed: {}", e))?;
    
    let account_exists = AccountManager::account_exists(&auth_response.uuid)
        .map_err(|e| format!("Failed to check account: {}", e))?;
    
    if account_exists {
        AccountManager::set_active_account(&auth_response.uuid)
            .map_err(|e| format!("Failed to switch account: {}", e))?;
    } else {
        AccountManager::add_account(
            auth_response.uuid.clone(),
            auth_response.username.clone(),
            auth_response.access_token.clone(),
        )
        .map_err(|e| format!("Failed to store account: {}", e))?;
    }
    
    let accounts = AccountManager::get_all_accounts()
        .map_err(|e| format!("Failed to get accounts: {}", e))?;
    
    accounts
        .into_iter()
        .find(|acc| acc.uuid == auth_response.uuid)
        .ok_or_else(|| "Failed to retrieve account info".to_string())
}