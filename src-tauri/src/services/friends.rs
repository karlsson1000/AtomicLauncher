use crate::models::{Friend, FriendRequest, FriendStatus, RequestStatus, UserStatusUpdate};
use chrono::Utc;
use serde_json::json;

pub struct FriendsService {
    client: reqwest::Client,
    supabase_url: String,
    supabase_key: String,
}

impl FriendsService {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let supabase_url = env!("SUPABASE_URL").to_string();
        let supabase_key = env!("SUPABASE_ANON_KEY").to_string();

        Ok(Self {
            client: reqwest::Client::new(),
            supabase_url,
            supabase_key,
        })
    }

pub async fn register_user(&self, uuid: &str, username: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/rest/v1/users", self.supabase_url);
        
    println!("Registering user: {} ({})", username, uuid);
        
    // Check if user already exists and their current status
    let check_url = format!("{}/rest/v1/users?uuid=eq.{}", self.supabase_url, uuid);
    let existing = self.client
        .get(&check_url)
        .header("apikey", &self.supabase_key)
        .header("Authorization", format!("Bearer {}", self.supabase_key))
        .send()
        .await?;
        
    let existing_users: Vec<serde_json::Value> = existing.json().await?;

    let payload = if existing_users.is_empty() {
        json!({
            "uuid": uuid,
            "username": username,
            "status": "online",
            "last_seen": Utc::now()
        })
    } else {
        json!({
            "uuid": uuid,
            "username": username,
            "last_seen": Utc::now()
        })
    };

    let response = self.client
        .post(&url)
        .header("apikey", &self.supabase_key)
        .header("Authorization", format!("Bearer {}", self.supabase_key))
        .header("Content-Type", "application/json")
        .header("Prefer", "resolution=merge-duplicates")
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    println!("Registration response status: {}", status);
    
    if !status.is_success() {
        let error_text = response.text().await?;
        println!("Registration error: {}", error_text);
        return Err(format!("Failed to register user: {}", error_text).into());
    }
    
    println!("User registered successfully!");
    Ok(())
}

    // Update user status
    pub async fn update_status(&self, uuid: &str, status: FriendStatus, current_instance: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/rest/v1/users?uuid=eq.{}", self.supabase_url, uuid);
        
        let status_str = match status {
            FriendStatus::Online => "online",
            FriendStatus::Offline => "offline",
            FriendStatus::InGame => "ingame",
        };

        let payload = json!({
            "status": status_str,
            "current_instance": current_instance,
            "last_seen": Utc::now()
        });

        self.client
            .patch(&url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    // Send friend request
    pub async fn send_friend_request(&self, from_uuid: &str, to_username: &str) -> Result<(), Box<dyn std::error::Error>> {
        // First, find the user by username
        let users_url = format!("{}/rest/v1/users?username=eq.{}", self.supabase_url, to_username);
        
        println!("Looking for user: {}", to_username);
        println!("Query URL: {}", users_url);
        
        let response = self.client
            .get(&users_url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .send()
            .await?;

        let status = response.status();
        println!("Response status: {}", status);
        
        let response_text = response.text().await?;
        println!("Response body: {}", response_text);
        
        let users: Vec<serde_json::Value> = serde_json::from_str(&response_text)?;
        
        if users.is_empty() {
            println!("User '{}' not found in database. They need to log in first.", to_username);
            return Err(format!("User '{}' not found. They need to sign in to the launcher first.", to_username).into());
        }

        let to_uuid = users[0]["uuid"].as_str().ok_or("Invalid user data")?;

        // Check if they're already friends
        let friendship_check = format!(
            "{}/rest/v1/friendships?or=(and(user_uuid.eq.{},friend_uuid.eq.{}),and(user_uuid.eq.{},friend_uuid.eq.{}))",
            self.supabase_url, from_uuid, to_uuid, to_uuid, from_uuid
        );

        let existing_friendship: Vec<serde_json::Value> = self.client
            .get(&friendship_check)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .send()
            .await?
            .json()
            .await?;

        if !existing_friendship.is_empty() {
            return Err("Already friends".into());
        }

        // Check if there's a pending request in either direction
        let request_check = format!(
            "{}/rest/v1/friend_requests?or=(and(from_uuid.eq.{},to_uuid.eq.{}),and(from_uuid.eq.{},to_uuid.eq.{}))&status=eq.pending",
            self.supabase_url, from_uuid, to_uuid, to_uuid, from_uuid
        );

        let existing_request: Vec<serde_json::Value> = self.client
            .get(&request_check)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .send()
            .await?
            .json()
            .await?;

        if !existing_request.is_empty() {
            // Check if the existing request is TO us (we should accept it instead)
            if existing_request[0]["to_uuid"].as_str() == Some(from_uuid) {
                return Err("This user has already sent you a friend request. Please accept it from your requests.".into());
            }
            return Err("Friend request already sent".into());
        }

        // Delete any old rejected/accepted requests between these users to allow re-sending
        let cleanup_url = format!(
            "{}/rest/v1/friend_requests?or=(and(from_uuid.eq.{},to_uuid.eq.{}),and(from_uuid.eq.{},to_uuid.eq.{}))&status=neq.pending",
            self.supabase_url, from_uuid, to_uuid, to_uuid, from_uuid
        );

        let _ = self.client
            .delete(&cleanup_url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .send()
            .await;

        // Create friend request
        let url = format!("{}/rest/v1/friend_requests", self.supabase_url);
        
        let payload = json!({
            "from_uuid": from_uuid,
            "to_uuid": to_uuid,
            "status": "pending"
        });

        self.client
            .post(&url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    // Get incoming friend requests
    pub async fn get_friend_requests(&self, user_uuid: &str) -> Result<Vec<FriendRequest>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/rest/v1/friend_requests?to_uuid=eq.{}&status=eq.pending&select=*,from_user:users!friend_requests_from_uuid_fkey(uuid,username)",
            self.supabase_url, user_uuid
        );

        let response = self.client
            .get(&url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .send()
            .await?;

        let data: Vec<serde_json::Value> = response.json().await?;
        
        let mut requests = Vec::new();
        for item in data {
            if let Some(from_user) = item.get("from_user") {
                requests.push(FriendRequest {
                    id: item["id"].as_str().unwrap_or("").to_string(),
                    from_uuid: from_user["uuid"].as_str().unwrap_or("").to_string(),
                    from_username: from_user["username"].as_str().unwrap_or("").to_string(),
                    to_uuid: user_uuid.to_string(),
                    status: RequestStatus::Pending,
                    created_at: item["created_at"].as_str()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or_else(Utc::now),
                });
            }
        }

        Ok(requests)
    }

    // Accept friend request
    pub async fn accept_friend_request(&self, request_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Get the request details
        let url = format!("{}/rest/v1/friend_requests?id=eq.{}", self.supabase_url, request_id);
        
        let response = self.client
            .get(&url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .send()
            .await?;

        let requests: Vec<serde_json::Value> = response.json().await?;
        
        if requests.is_empty() {
            return Err("Request not found".into());
        }

        let request = &requests[0];
        let from_uuid = request["from_uuid"].as_str().ok_or("Invalid request")?;
        let to_uuid = request["to_uuid"].as_str().ok_or("Invalid request")?;

        // Create friendship (both directions)
        let friendship_url = format!("{}/rest/v1/friendships", self.supabase_url);
        
        let payload1 = json!({
            "user_uuid": from_uuid,
            "friend_uuid": to_uuid
        });

        let payload2 = json!({
            "user_uuid": to_uuid,
            "friend_uuid": from_uuid
        });

        self.client
            .post(&friendship_url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .header("Content-Type", "application/json")
            .json(&payload1)
            .send()
            .await?;

        self.client
            .post(&friendship_url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .header("Content-Type", "application/json")
            .json(&payload2)
            .send()
            .await?;

        // Update request status
        let update_url = format!("{}/rest/v1/friend_requests?id=eq.{}", self.supabase_url, request_id);
        
        let update_payload = json!({
            "status": "accepted"
        });

        self.client
            .patch(&update_url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .header("Content-Type", "application/json")
            .json(&update_payload)
            .send()
            .await?;

        Ok(())
    }

    // Reject friend request (deletes it to allow re-sending)
    pub async fn reject_friend_request(&self, request_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/rest/v1/friend_requests?id=eq.{}", self.supabase_url, request_id);
        
        // Delete the request entirely instead of marking as rejected
        // This allows users to send new requests after rejection
        self.client
            .delete(&url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .send()
            .await?;

        Ok(())
    }

    // Get friends list with status
    pub async fn get_friends(&self, user_uuid: &str) -> Result<Vec<Friend>, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/rest/v1/friendships?user_uuid=eq.{}&select=friend:users!friendships_friend_uuid_fkey(uuid,username,status,last_seen,current_instance)",
            self.supabase_url, user_uuid
        );

        let response = self.client
            .get(&url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .send()
            .await?;

        let data: Vec<serde_json::Value> = response.json().await?;
        
        let mut friends = Vec::new();
        for item in data {
            if let Some(friend) = item.get("friend") {
                let status_str = friend["status"].as_str().unwrap_or("offline");
                let status = match status_str {
                    "online" => FriendStatus::Online,
                    "ingame" => FriendStatus::InGame,
                    _ => FriendStatus::Offline,
                };

                friends.push(Friend {
                    uuid: friend["uuid"].as_str().unwrap_or("").to_string(),
                    username: friend["username"].as_str().unwrap_or("").to_string(),
                    status,
                    last_seen: friend["last_seen"].as_str()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or_else(Utc::now),
                    current_instance: friend["current_instance"].as_str().map(String::from),
                });
            }
        }

        Ok(friends)
    }

    // Remove friend
    pub async fn remove_friend(&self, user_uuid: &str, friend_uuid: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Delete both friendship entries
        let url1 = format!(
            "{}/rest/v1/friendships?user_uuid=eq.{}&friend_uuid=eq.{}",
            self.supabase_url, user_uuid, friend_uuid
        );

        let url2 = format!(
            "{}/rest/v1/friendships?user_uuid=eq.{}&friend_uuid=eq.{}",
            self.supabase_url, friend_uuid, user_uuid
        );

        self.client
            .delete(&url1)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .send()
            .await?;

        self.client
            .delete(&url2)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .send()
            .await?;

        Ok(())
    }
}