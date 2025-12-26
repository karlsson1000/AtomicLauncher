use crate::models::{AccountsData, StoredAccount, AccountInfo};
use crate::utils::get_launcher_dir;
use chrono::Utc;
use std::fs;

pub struct AccountManager;

impl AccountManager {
    fn get_accounts_file() -> std::path::PathBuf {
        get_launcher_dir().join("accounts.json")
    }

    /// Load all accounts from disk
    pub fn load() -> Result<AccountsData, Box<dyn std::error::Error>> {
        let accounts_file = Self::get_accounts_file();
        
        if !accounts_file.exists() {
            return Ok(AccountsData::default());
        }
        
        let content = fs::read_to_string(&accounts_file)?;
        let accounts_data: AccountsData = serde_json::from_str(&content)?;
        
        Ok(accounts_data)
    }

    /// Save accounts to disk
    pub fn save(accounts_data: &AccountsData) -> Result<(), Box<dyn std::error::Error>> {
        let accounts_file = Self::get_accounts_file();
        let json = serde_json::to_string_pretty(accounts_data)?;
        fs::write(&accounts_file, json)?;
        Ok(())
    }

    /// Add a new account
    pub fn add_account(
        uuid: String,
        username: String,
        access_token: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut accounts_data = Self::load()?;
        
        let stored_account = StoredAccount {
            uuid: uuid.clone(),
            username,
            access_token,
            added_at: Utc::now().to_rfc3339(),
            last_used: Some(Utc::now().to_rfc3339()),
        };
        
        // Add the account
        accounts_data.accounts.insert(uuid.clone(), stored_account);
        
        // Set as active account if it's the first one
        if accounts_data.active_account_uuid.is_none() {
            accounts_data.active_account_uuid = Some(uuid);
        }
        
        Self::save(&accounts_data)?;
        Ok(())
    }

    /// Remove an account
    pub fn remove_account(uuid: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut accounts_data = Self::load()?;
        
        // Remove the account
        accounts_data.accounts.remove(uuid);
        
        // If this was the active account, switch to another one
        if accounts_data.active_account_uuid.as_deref() == Some(uuid) {
            accounts_data.active_account_uuid = accounts_data
                .accounts
                .keys()
                .next()
                .map(|k| k.to_string());
        }
        
        Self::save(&accounts_data)?;
        Ok(())
    }

    /// Set the active account
    pub fn set_active_account(uuid: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut accounts_data = Self::load()?;
        
        // Verify account exists
        if !accounts_data.accounts.contains_key(uuid) {
            return Err(format!("Account with UUID {} not found", uuid).into());
        }
        
        // Update last used timestamp
        if let Some(account) = accounts_data.accounts.get_mut(uuid) {
            account.last_used = Some(Utc::now().to_rfc3339());
        }
        
        accounts_data.active_account_uuid = Some(uuid.to_string());
        Self::save(&accounts_data)?;
        Ok(())
    }

    /// Get the active account
    pub fn get_active_account() -> Result<Option<StoredAccount>, Box<dyn std::error::Error>> {
        let accounts_data = Self::load()?;
        
        if let Some(uuid) = &accounts_data.active_account_uuid {
            Ok(accounts_data.accounts.get(uuid).cloned())
        } else {
            Ok(None)
        }
    }

    /// Get all accounts as a list
    pub fn get_all_accounts() -> Result<Vec<AccountInfo>, Box<dyn std::error::Error>> {
        let accounts_data = Self::load()?;
        
        let mut accounts: Vec<AccountInfo> = accounts_data
            .accounts
            .values()
            .map(|acc| AccountInfo {
                uuid: acc.uuid.clone(),
                username: acc.username.clone(),
                is_active: accounts_data.active_account_uuid.as_deref() == Some(&acc.uuid),
                added_at: acc.added_at.clone(),
                last_used: acc.last_used.clone(),
            })
            .collect();
        
        // Sort by last used (most recent first)
        accounts.sort_by(|a, b| {
            match (&b.last_used, &a.last_used) {
                (Some(b_time), Some(a_time)) => b_time.cmp(a_time),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
        
        Ok(accounts)
    }

    /// Check if an account with this UUID already exists
    pub fn account_exists(uuid: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let accounts_data = Self::load()?;
        Ok(accounts_data.accounts.contains_key(uuid))
    }
}