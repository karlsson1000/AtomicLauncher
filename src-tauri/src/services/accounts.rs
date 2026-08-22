use crate::models::{AccountInfo, AccountsData, StoredAccount};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

const KEYRING_SERVICE: &str = "OctaneLauncher";

struct CachedAccess {
    access_token: String,
    token_expiry: DateTime<Utc>,
}

static ACCESS_CACHE: LazyLock<Mutex<HashMap<String, CachedAccess>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn keyring_entry(uuid: &str) -> Result<keyring::Entry, Box<dyn std::error::Error>> {
    Ok(keyring::Entry::new(KEYRING_SERVICE, uuid)?)
}

fn store_tokens(uuid: &str, refresh_token: &str) -> Result<(), Box<dyn std::error::Error>> {
    keyring_entry(uuid)?.set_password(refresh_token)?;
    Ok(())
}

fn load_tokens(uuid: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    match keyring_entry(uuid)?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn delete_tokens(uuid: &str) {
    if let Ok(entry) = keyring_entry(uuid) {
        let _ = entry.delete_credential();
    }
}

pub struct AccountManager;

impl AccountManager {
    fn get_accounts_file() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let data_dir = crate::utils::get_launcher_dir();

        fs::create_dir_all(&data_dir)?;
        Ok(data_dir.join("accounts.json"))
    }

    fn load_accounts() -> Result<AccountsData, Box<dyn std::error::Error>> {
        let path = Self::get_accounts_file()?;

        if !path.exists() {
            return Ok(AccountsData::default());
        }

        let contents = fs::read_to_string(path)?;
        let mut data: AccountsData = serde_json::from_str(&contents)?;

        let mut changed = false;
        let mut dropped: Vec<String> = Vec::new();

        for (uuid, account) in data.accounts.iter_mut() {
            match load_tokens(uuid) {
                Ok(Some(_)) => {
                    if !account.access_token.is_empty() {
                        account.access_token.clear();
                        account.refresh_token.clear();
                        changed = true;
                    }
                }
                Ok(None) => {
                    dropped.push(uuid.clone());
                    changed = true;
                }
                Err(_) => {}
            }
        }

        for uuid in &dropped {
            data.accounts.remove(uuid);
        }

        if data
            .active_account_uuid
            .as_ref()
            .is_some_and(|active| dropped.contains(active))
        {
            data.active_account_uuid = data.accounts.keys().next().cloned();
        }

        if changed {
            Self::save_accounts(&data)?;
        }

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
        store_tokens(&uuid, &refresh_token)?;
        ACCESS_CACHE
            .lock()
            .unwrap()
            .insert(uuid.clone(), CachedAccess {
                access_token,
                token_expiry,
            });

        let mut data = Self::load_accounts()?;

        let account = StoredAccount {
            uuid: uuid.clone(),
            username,
            access_token: String::new(),
            refresh_token: String::new(),
            token_expiry,
            added_at: Utc::now().to_rfc3339(),
            last_used: Some(Utc::now().to_rfc3339()),
        };

        data.accounts.insert(uuid.clone(), account);

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

        if let Some(account) = data.accounts.get_mut(uuid) {
            account.last_used = Some(Utc::now().to_rfc3339());
        }

        Self::save_accounts(&data)?;
        Ok(())
    }

    pub fn remove_account(uuid: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = Self::load_accounts()?;

        let was_active = data.active_account_uuid.as_ref() == Some(&uuid.to_string());
        data.accounts.remove(uuid);
        delete_tokens(uuid);

        if was_active {
            if let Some(first_remaining) = data.accounts.keys().next().cloned() {
                data.active_account_uuid = Some(first_remaining);
            } else {
                data.active_account_uuid = None;
            }
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
        store_tokens(uuid, &refresh_token)?;
        ACCESS_CACHE
            .lock()
            .unwrap()
            .insert(uuid.to_string(), CachedAccess {
                access_token,
                token_expiry,
            });

        let mut data = Self::load_accounts()?;

        let account = data
            .accounts
            .get_mut(uuid)
            .ok_or("Account not found")?;

        account.token_expiry = token_expiry;
        account.last_used = Some(Utc::now().to_rfc3339());

        Self::save_accounts(&data)?;
        Ok(())
    }

    pub async fn get_valid_token(uuid: &str, client_id: &str) -> Result<String, Box<dyn std::error::Error>> {
        let data = Self::load_accounts()?;
        data.accounts
            .get(uuid)
            .ok_or("Account not found")?;

        let now = Utc::now();
        let buffer = chrono::Duration::minutes(5);

        if let Some(cached) = ACCESS_CACHE.lock().unwrap().get(uuid) {
            if cached.token_expiry > now + buffer {
                return Ok(cached.access_token.clone());
            }
        }

        let refresh_token = load_tokens(uuid)?.ok_or("Account credentials not found")?;

        let authenticator = crate::auth::Authenticator::new(client_id)?;
        let refreshed = authenticator.refresh_tokens(&refresh_token).await?;

        store_tokens(uuid, &refreshed.refresh_token)?;
        ACCESS_CACHE.lock().unwrap().insert(
            uuid.to_string(),
            CachedAccess {
                access_token: refreshed.access_token.clone(),
                token_expiry: refreshed.token_expiry,
            },
        );
        Self::update_account_expiry(uuid, refreshed.token_expiry)?;

        Ok(refreshed.access_token)
    }

    fn update_account_expiry(
        uuid: &str,
        token_expiry: DateTime<Utc>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = Self::load_accounts()?;
        if let Some(account) = data.accounts.get_mut(uuid) {
            account.token_expiry = token_expiry;
            account.last_used = Some(Utc::now().to_rfc3339());
            Self::save_accounts(&data)?;
        }
        Ok(())
    }
}