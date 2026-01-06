use crate::models::{AccountInfo, AccountsData, StoredAccount};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::PathBuf;

pub struct AccountManager;

impl AccountManager {
    fn get_accounts_file() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let data_dir = dirs::data_dir()
            .ok_or("Could not find data directory")?
            .join("atomic-launcher");
        
        fs::create_dir_all(&data_dir)?;
        Ok(data_dir.join("accounts.json"))
    }

    fn load_accounts() -> Result<AccountsData, Box<dyn std::error::Error>> {
        let path = Self::get_accounts_file()?;
        
        if !path.exists() {
            return Ok(AccountsData::default());
        }

        let contents = fs::read_to_string(path)?;
        let data: AccountsData = serde_json::from_str(&contents)?;
        Ok(data)
    }

    fn save_accounts(data: &AccountsData) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::get_accounts_file()?;
        let json = serde_json::to_string_pretty(data)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn add_account(
        uuid: String,
        username: String,
        access_token: String,
        refresh_token: String,
        token_expiry: DateTime<Utc>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = Self::load_accounts()?;
        
        let account = StoredAccount {
            uuid: uuid.clone(),
            username,
            access_token,
            refresh_token,
            token_expiry,
            added_at: Utc::now().to_rfc3339(),
            last_used: Some(Utc::now().to_rfc3339()),
        };

        data.accounts.insert(uuid.clone(), account);
        
        // Set as active if it's the first account
        if data.active_account_uuid.is_none() {
            data.active_account_uuid = Some(uuid);
        }

        Self::save_accounts(&data)?;
        Ok(())
    }

    pub fn account_exists(uuid: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let data = Self::load_accounts()?;
        Ok(data.accounts.contains_key(uuid))
    }

    pub fn get_all_accounts() -> Result<Vec<AccountInfo>, Box<dyn std::error::Error>> {
        let data = Self::load_accounts()?;
        
        let accounts: Vec<AccountInfo> = data
            .accounts
            .values()
            .map(|acc| AccountInfo {
                uuid: acc.uuid.clone(),
                username: acc.username.clone(),
                is_active: data.active_account_uuid.as_ref() == Some(&acc.uuid),
                added_at: acc.added_at.clone(),
                last_used: acc.last_used.clone(),
            })
            .collect();

        Ok(accounts)
    }

    pub fn get_active_account() -> Result<Option<StoredAccount>, Box<dyn std::error::Error>> {
        let data = Self::load_accounts()?;
        
        if let Some(uuid) = &data.active_account_uuid {
            Ok(data.accounts.get(uuid).cloned())
        } else {
            Ok(None)
        }
    }

    pub fn set_active_account(uuid: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = Self::load_accounts()?;
        
        if !data.accounts.contains_key(uuid) {
            return Err("Account not found".into());
        }

        data.active_account_uuid = Some(uuid.to_string());
        
        // Update last_used timestamp
        if let Some(account) = data.accounts.get_mut(uuid) {
            account.last_used = Some(Utc::now().to_rfc3339());
        }

        Self::save_accounts(&data)?;
        Ok(())
    }

    pub fn remove_account(uuid: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = Self::load_accounts()?;
        
        data.accounts.remove(uuid);
        
        // If removed account was active, clear active account
        if data.active_account_uuid.as_ref() == Some(&uuid.to_string()) {
            data.active_account_uuid = None;
        }

        Self::save_accounts(&data)?;
        Ok(())
    }

    pub fn update_account_tokens(
        uuid: &str,
        access_token: String,
        refresh_token: String,
        token_expiry: DateTime<Utc>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = Self::load_accounts()?;
        
        let account = data
            .accounts
            .get_mut(uuid)
            .ok_or("Account not found")?;

        account.access_token = access_token;
        account.refresh_token = refresh_token;
        account.token_expiry = token_expiry;
        account.last_used = Some(Utc::now().to_rfc3339());

        Self::save_accounts(&data)?;
        Ok(())
    }

    pub async fn get_valid_token(uuid: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Load fresh data to check token status
        let data = Self::load_accounts()?;
        let account = data
            .accounts
            .get(uuid)
            .ok_or("Account not found")?
            .clone();

        let now = Utc::now();
        let buffer = chrono::Duration::minutes(5);
        
        // Check if token is still valid (expires more than 5 minutes from now)
        if account.token_expiry > now + buffer {
            let minutes_until_expiry = (account.token_expiry - now).num_minutes();
            println!("Token still valid (expires in {} minutes)", minutes_until_expiry);
            return Ok(account.access_token);
        }
        
        // Token is expired or expiring soon, refresh it
        println!("Token expired or expiring soon, refreshing...");
        
        let authenticator = crate::auth::Authenticator::new()?;
        let refreshed = authenticator.refresh_tokens(&account.refresh_token).await?;
        
        // Update the account with new tokens
        Self::update_account_tokens(
            uuid,
            refreshed.access_token.clone(),
            refreshed.refresh_token,
            refreshed.token_expiry,
        )?;
        
        println!("✓ Token refreshed successfully");
        Ok(refreshed.access_token)
    }
}