use crate::models::{Friend, FriendRequest, FriendStatus};
use crate::services::accounts::AccountManager;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use lazy_static::lazy_static;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::Manager;

const SESSION_REFRESH_MARGIN_SECS: i64 = 60;

static EXCHANGE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

lazy_static! {
    static ref SESSION_CACHE: Mutex<HashMap<String, (String, DateTime<Utc>)>> =
        Mutex::new(HashMap::new());
}

pub struct DatabaseService {
    client: reqwest::Client,
    base_url: String,
    anon_key: String,
}

impl DatabaseService {
    pub fn new(base_url: &str, anon_key: &str) -> Self {
        Self {
            client: crate::utils::http::get_client(),
            base_url: base_url.trim_end_matches('/').to_string(),
            anon_key: anon_key.to_string(),
        }
    }

    async fn check_success(response: reqwest::Response) -> Result<(), String> {
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        let text = response.text().await.unwrap_or_default();
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(message) = value["message"].as_str() {
                return Err(message.to_string());
            }
        }
        Err(format!("Database error ({status})"))
    }

    fn rpc_url(&self, name: &str) -> String {
        format!("{}/rest/v1/rpc/{}", self.base_url, name)
    }

    pub async fn register_user(
        &self,
        bearer: &str,
        uuid: &str,
        username: &str,
    ) -> Result<(), String> {
        let response = self
            .client
            .post(format!("{}/rest/v1/users", self.base_url))
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", bearer))
            .header("Prefer", "resolution=merge-duplicates")
            .json(&json!({
                "uuid": uuid,
                "username": username,
                "status": "online",
                "current_instance": null,
                "last_seen": Utc::now()
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Self::check_success(response).await
    }

    pub async fn update_status(
        &self,
        bearer: &str,
        uuid: &str,
        status: FriendStatus,
        current_instance: Option<String>,
    ) -> Result<(), String> {
        let status_str = match status {
            FriendStatus::Online => "online",
            FriendStatus::Offline => "offline",
            FriendStatus::InGame => "ingame",
        };

        let response = self
            .client
            .patch(format!("{}/rest/v1/users?uuid=eq.{}", self.base_url, uuid))
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", bearer))
            .json(&json!({
                "status": status_str,
                "current_instance": current_instance,
                "last_seen": Utc::now()
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Self::check_success(response).await
    }

    pub async fn send_friend_request(
        &self,
        bearer: &str,
        to_username: &str,
    ) -> Result<(), String> {
        let response = self
            .client
            .post(self.rpc_url("send_friend_request"))
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", bearer))
            .json(&json!({ "to_username": to_username }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Self::check_success(response).await
    }

    pub async fn accept_friend_request(
        &self,
        bearer: &str,
        request_id: &str,
    ) -> Result<(), String> {
        let response = self
            .client
            .post(self.rpc_url("accept_friend_request"))
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", bearer))
            .json(&json!({ "p_request_id": request_id }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Self::check_success(response).await
    }

    pub async fn reject_friend_request(
        &self,
        bearer: &str,
        request_id: &str,
    ) -> Result<(), String> {
        let response = self
            .client
            .post(self.rpc_url("reject_friend_request"))
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", bearer))
            .json(&json!({ "p_request_id": request_id }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Self::check_success(response).await
    }

    pub async fn remove_friend(&self, bearer: &str, friend_uuid: &str) -> Result<(), String> {
        let response = self
            .client
            .post(self.rpc_url("remove_friend"))
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", bearer))
            .json(&json!({ "f": friend_uuid }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Self::check_success(response).await
    }

    pub async fn get_friend_requests(
        &self,
        bearer: &str,
        user_uuid: &str,
    ) -> Result<Vec<FriendRequest>, String> {
        let url = format!(
            "{}/rest/v1/friend_requests?to_uuid=eq.{}&status=eq.pending&select=*,from_user:users!friend_requests_from_uuid_fkey(uuid,username)",
            self.base_url, user_uuid
        );

        let data: Vec<serde_json::Value> = self
            .client
            .get(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", bearer))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

        let mut requests = Vec::new();
        for item in data {
            if let Some(from_user) = item.get("from_user") {
                requests.push(FriendRequest {
                    id: item["id"].as_str().unwrap_or("").to_string(),
                    from_uuid: from_user["uuid"].as_str().unwrap_or("").to_string(),
                    from_username: from_user["username"].as_str().unwrap_or("").to_string(),
                    to_uuid: user_uuid.to_string(),
                    status: crate::models::RequestStatus::Pending,
                    created_at: item["created_at"]
                        .as_str()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or_else(Utc::now),
                });
            }
        }

        Ok(requests)
    }

    pub async fn get_friends(&self, bearer: &str, user_uuid: &str) -> Result<Vec<Friend>, String> {
        let url = format!(
            "{}/rest/v1/friendships?user_uuid=eq.{}&select=friend:users!friendships_friend_uuid_fkey(uuid,username,status,last_seen,current_instance)",
            self.base_url, user_uuid
        );

        let data: Vec<serde_json::Value> = self
            .client
            .get(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {}", bearer))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

        let staleness_cutoff = Utc::now() - ChronoDuration::seconds(120);

        let mut friends = Vec::new();
        for item in data {
            if let Some(friend) = item.get("friend") {
                let last_seen: DateTime<Utc> = friend["last_seen"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(Utc::now);

                let status_str = friend["status"].as_str().unwrap_or("offline");
                let status = if last_seen < staleness_cutoff {
                    FriendStatus::Offline
                } else {
                    match status_str {
                        "online" => FriendStatus::Online,
                        "ingame" => FriendStatus::InGame,
                        _ => FriendStatus::Offline,
                    }
                };

                friends.push(Friend {
                    uuid: friend["uuid"].as_str().unwrap_or("").to_string(),
                    username: friend["username"].as_str().unwrap_or("").to_string(),
                    status,
                    last_seen,
                    current_instance: friend["current_instance"].as_str().map(String::from),
                });
            }
        }

        Ok(friends)
    }
}

pub async fn get_session_token(
    app: &tauri::AppHandle,
    account_uuid: &str,
) -> Result<String, String> {
    {
        let cache = SESSION_CACHE.lock().map_err(|_| "Session cache poisoned")?;
        if let Some((token, expiry)) = cache.get(account_uuid) {
            if *expiry > Utc::now() + ChronoDuration::seconds(SESSION_REFRESH_MARGIN_SECS) {
                return Ok(token.clone());
            }
        }
    }

    let _exchange_guard = EXCHANGE.lock().await;

    {
        let cache = SESSION_CACHE.lock().map_err(|_| "Session cache poisoned")?;
        if let Some((token, expiry)) = cache.get(account_uuid) {
            if *expiry > Utc::now() + ChronoDuration::seconds(SESSION_REFRESH_MARGIN_SECS) {
                return Ok(token.clone());
            }
        }
    }

    let config = app.state::<crate::models::AppConfig>();
    let client_id = config.microsoft_client_id.clone();
    let base_url = config.database_url.clone();
    let anon_key = config.database_key.clone();

    let mc_token = AccountManager::get_valid_token(account_uuid, &client_id)
        .await
        .map_err(|e| e.to_string())?;

    let response = crate::utils::http::get_client()
        .post(format!(
            "{}/functions/v1/database-auth",
            base_url.trim_end_matches('/')
        ))
        .header("apikey", &anon_key)
        .json(&json!({ "access_token": mc_token }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    let text = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("Database authentication failed ({status})"));
    }

    let parsed: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Invalid auth response: {e}"))?;
    let token = parsed["token"]
        .as_str()
        .ok_or("Auth response missing token")?
        .to_string();
    let expires_in = parsed["expires_in"].as_i64().unwrap_or(3600);

    {
        let mut cache = SESSION_CACHE.lock().map_err(|_| "Session cache poisoned")?;
        cache.insert(
            account_uuid.to_string(),
            (
                token.clone(),
                Utc::now() + ChronoDuration::seconds(expires_in),
            ),
        );
    }

    Ok(token)
}

pub fn has_live_session(account_uuid: &str) -> bool {
    let Ok(cache) = SESSION_CACHE.lock() else {
        return false;
    };
    cache
        .get(account_uuid)
        .is_some_and(|(_, expiry)| *expiry > Utc::now() + ChronoDuration::seconds(SESSION_REFRESH_MARGIN_SECS))
}

pub async fn set_status_for_account(
    app: &tauri::AppHandle,
    account_uuid: &str,
    status: FriendStatus,
    current_instance: Option<String>,
) -> Result<(), String> {
    let config = app.state::<crate::models::AppConfig>();
    let service = DatabaseService::new(&config.database_url, &config.database_key);
    drop(config);

    let bearer = get_session_token(app, account_uuid).await?;
    service
        .update_status(&bearer, account_uuid, status, current_instance)
        .await
}
