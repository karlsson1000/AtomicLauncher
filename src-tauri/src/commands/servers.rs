use crate::commands::validation::{sanitize_server_name, validate_server_address};
use crate::utils::get_launcher_dir;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub address: String,
    pub port: u16,
    pub status: String,
    pub players_online: Option<u32>,
    pub players_max: Option<u32>,
    pub version: Option<String>,
    pub motd: Option<String>,
    pub favicon: Option<String>,
    pub last_checked: Option<i64>,
}

#[tauri::command]
pub async fn get_servers() -> Result<Vec<ServerInfo>, String> {
    let servers_file = get_launcher_dir().join("servers.json");
    
    if !servers_file.exists() {
        return Ok(Vec::new());
    }
    
    let content = std::fs::read_to_string(&servers_file)
        .map_err(|e| format!("Failed to read servers file: {}", e))?;
    
    let servers: Vec<ServerInfo> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse servers file: {}", e))?;
    
    Ok(servers)
}

#[tauri::command]
pub async fn add_server(
    name: String,
    address: String,
    port: u16,
) -> Result<String, String> {
    // Validate inputs
    let safe_name = sanitize_server_name(&name)?;
    validate_server_address(&address)?;
    
    if port == 0 {
        return Err("Port cannot be 0".to_string());
    }
    
    // Load existing servers
    let mut servers = get_servers().await?;
    
    // Check if server with same name already exists
    if servers.iter().any(|s| s.name.to_lowercase() == safe_name.to_lowercase()) {
        return Err(format!("Server '{}' already exists", safe_name));
    }
    
    // Create new server entry
    let new_server = ServerInfo {
        name: safe_name.clone(),
        address,
        port,
        status: "unknown".to_string(),
        players_online: None,
        players_max: None,
        version: None,
        motd: None,
        favicon: None,
        last_checked: None,
    };
    
    servers.push(new_server);
    
    // Save to file
    let servers_file = get_launcher_dir().join("servers.json");
    let json = serde_json::to_string_pretty(&servers)
        .map_err(|e| format!("Failed to serialize servers: {}", e))?;
    
    std::fs::write(&servers_file, json)
        .map_err(|e| format!("Failed to write servers file: {}", e))?;
    
    Ok(format!("Successfully added server '{}'", safe_name))
}

#[tauri::command]
pub async fn delete_server(server_name: String) -> Result<String, String> {
    let safe_name = sanitize_server_name(&server_name)?;
    
    let mut servers = get_servers().await?;
    
    let initial_len = servers.len();
    servers.retain(|s| s.name != safe_name);
    
    if servers.len() == initial_len {
        return Err(format!("Server '{}' not found", safe_name));
    }
    
    // Save updated list
    let servers_file = get_launcher_dir().join("servers.json");
    let json = serde_json::to_string_pretty(&servers)
        .map_err(|e| format!("Failed to serialize servers: {}", e))?;
    
    std::fs::write(&servers_file, json)
        .map_err(|e| format!("Failed to write servers file: {}", e))?;
    
    Ok(format!("Successfully deleted server '{}'", safe_name))
}

#[tauri::command]
pub async fn update_server_status(
    server_name: String,
    status: ServerInfo,
) -> Result<String, String> {
    let safe_name = sanitize_server_name(&server_name)?;
    
    let mut servers = get_servers().await?;
    
    // Find and update the server
    let server = servers.iter_mut()
        .find(|s| s.name == safe_name)
        .ok_or_else(|| format!("Server '{}' not found", safe_name))?;
    
    // Update fields
    server.status = status.status;
    server.players_online = status.players_online;
    server.players_max = status.players_max;
    server.version = status.version;
    server.motd = status.motd;
    server.favicon = status.favicon;
    server.last_checked = Some(chrono::Utc::now().timestamp());
    
    // Save updated list
    let servers_file = get_launcher_dir().join("servers.json");
    let json = serde_json::to_string_pretty(&servers)
        .map_err(|e| format!("Failed to serialize servers: {}", e))?;
    
    std::fs::write(&servers_file, json)
        .map_err(|e| format!("Failed to write servers file: {}", e))?;
    
    Ok(format!("Successfully updated server '{}'", safe_name))
}