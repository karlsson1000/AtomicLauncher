use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity::{Activity, Assets}};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct DiscordRpc {
    client_id: String,
    client: Arc<Mutex<Option<DiscordIpcClient>>>,
}

impl DiscordRpc {
    pub fn new(client_id: &str) -> Self {
        Self {
            client_id: client_id.to_string(),
            client: Arc::new(Mutex::new(None)),
        }
    }
    
    fn ensure_connected(&self) -> bool {
        if let Ok(mut client_guard) = self.client.lock() {
            if client_guard.is_none() {
                match DiscordIpcClient::new(&self.client_id) {
                    Ok(mut new_client) => {
                        if new_client.connect().is_ok() {
                            *client_guard = Some(new_client);
                            return true;
                        }
                    }
                    Err(_) => return false,
                }
            } else {
                return true;
            }
        }
        false
    }
    
    pub fn disconnect(&self) {
        if let Ok(mut client_guard) = self.client.lock() {
            if let Some(mut client) = client_guard.take() {
                let _ = client.close();
            }
        }
    }
    
    pub fn set_activity(&self, details: &str, state: Option<&str>, large_image: &str, large_text: &str) {
        if !self.ensure_connected() {
            return;
        }
        
        let client = self.client.clone();
        let details = details.to_string();
        let state = state.map(|s| s.to_string());
        let large_image = large_image.to_string();
        let large_text = large_text.to_string();
        
        thread::spawn(move || {
            if let Ok(mut client_guard) = client.lock() {
                if let Some(ref mut c) = *client_guard {
                    let assets = Assets::new()
                        .large_image(&large_image)
                        .large_text(&large_text);
                    
                    let mut activity = Activity::new()
                        .details(&details)
                        .assets(assets);
                    
                    if let Some(ref state_text) = state {
                        activity = activity.state(state_text);
                    }
                    
                    let _ = c.set_activity(activity);
                }
            }
        });
    }
    
    pub fn clear_activity(&self) {
        self.disconnect();
    }
    
    pub fn close(&self) {
        self.disconnect();
    }
}

impl Drop for DiscordRpc {
    fn drop(&mut self) {
        self.close();
    }
}