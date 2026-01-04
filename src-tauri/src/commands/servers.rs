use crate::commands::validation::{sanitize_server_name, validate_server_address};
use crate::services::accounts::AccountManager;
use crate::services::instance::InstanceManager;
use crate::utils::{get_launcher_dir, get_instance_dir};
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use std::io::Write;

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

#[tauri::command]
pub async fn launch_server(
    server_address: String,
    server_port: u16,
    server_name: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    println!("=== Launching Server Connection ===");
    println!("Server: {} ({}:{})", server_name, server_address, server_port);

    // Validate inputs
    let safe_name = sanitize_server_name(&server_name)?;
    validate_server_address(&server_address)?;
    
    if server_port == 0 {
        return Err("Invalid server port".to_string());
    }

    // Get active account
    let active_account = AccountManager::get_active_account()
        .map_err(|e| format!("Failed to get active account: {}", e))?
        .ok_or_else(|| "No active account. Please sign in first.".to_string())?;

    println!("Using account: {}", active_account.username);

    // Try to find the most recently played instance
    let instances = InstanceManager::get_all()
        .map_err(|e| format!("Failed to get instances: {}", e))?;
    
    let most_recent_instance = instances
        .iter()
        .filter(|inst| inst.last_played.is_some())
        .max_by_key(|inst| inst.last_played.as_ref());

    let instance_name = if let Some(recent_inst) = most_recent_instance {
        println!("Using most recently played instance: {}", recent_inst.name);
        recent_inst.name.clone()
    } else {
        return Err("No instances found. Please create an instance first before connecting to a server.".to_string());
    };

    let instance_dir = get_instance_dir(&instance_name);

    if !instance_dir.exists() {
        return Err(format!("Instance '{}' not found", instance_name));
    }

    // Add server to servers.dat in the instance directory
    add_server_to_instance(&instance_dir, &safe_name, &server_address, server_port)?;

    // Format server address for command line argument
    let server_arg = if server_port == 25565 {
        server_address.clone()
    } else {
        format!("{}:{}", server_address, server_port)
    };

    // Emit event that instance is launching for server connection
    let _ = app_handle.emit("server-instance-launching", serde_json::json!({
        "instance": instance_name,
        "server": safe_name
    }));

    // Launch the instance with server connection arguments
    println!("Launching Minecraft and connecting to server...");
    InstanceManager::launch_with_server(
        &instance_name,
        &active_account.username,
        &active_account.uuid,
        &active_account.access_token,
        &server_arg,
        app_handle.clone(),
    )
    .map_err(|e| format!("Failed to launch Minecraft: {}", e))?;

    Ok(format!(
        "Launching {} and connecting to {}",
        instance_name, safe_name
    ))
}

fn add_server_to_instance(
    instance_dir: &std::path::Path,
    server_name: &str,
    server_address: &str,
    server_port: u16,
) -> Result<(), String> {
    // Minecraft reads servers.dat from the gameDir
    let servers_dat = instance_dir.join("servers.dat");
    
    println!("Updating servers.dat at: {:?}", servers_dat);
    
    // Check if servers.dat already exists
    let existing_exists = servers_dat.exists();
    
    if existing_exists {
        println!("Found existing servers.dat - will preserve existing servers");
        println!("✓ Skipping servers.dat modification to preserve existing server list");
        return Ok(());
    }
    
    // Only create servers.dat if it doesn't exist
    println!("Creating new servers.dat with launch target");
    
    // Create NBT structure for servers.dat
    let nbt_data = create_servers_nbt(server_name, server_address, server_port);
    
    let mut file = std::fs::File::create(&servers_dat)
        .map_err(|e| format!("Failed to create servers.dat: {}", e))?;
    
    file.write_all(&nbt_data)
        .map_err(|e| format!("Failed to write servers.dat: {}", e))?;

    println!("✓ servers.dat created with {} bytes", nbt_data.len());
    println!("✓ Server entry: {} -> {}:{}", server_name, server_address, server_port);
    Ok(())
}

fn create_servers_nbt(server_name: &str, server_address: &str, server_port: u16) -> Vec<u8> {
    
    let mut data = Vec::new();
    
    // TAG_Compound
    data.push(0x0A); // TAG_Compound
    data.extend_from_slice(&[0x00, 0x00]);
    
    // TAG_List "servers"
    data.push(0x09); // TAG_List
    data.extend_from_slice(&[0x00, 0x07]);
    data.extend_from_slice(b"servers");
    data.push(0x0A);
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    
    // Server entry (TAG_Compound)
    // TAG_String "name"
    data.push(0x08); // TAG_String
    data.extend_from_slice(&[0x00, 0x04]);
    data.extend_from_slice(b"name");
    let name_bytes = server_name.as_bytes();
    data.extend_from_slice(&(name_bytes.len() as u16).to_be_bytes());
    data.extend_from_slice(name_bytes);
    
    // TAG_String "ip"
    data.push(0x08); // TAG_String
    data.extend_from_slice(&[0x00, 0x02]);
    data.extend_from_slice(b"ip");
    let ip_string = if server_port == 25565 {
        server_address.to_string()
    } else {
        format!("{}:{}", server_address, server_port)
    };
    let ip_bytes = ip_string.as_bytes();
    data.extend_from_slice(&(ip_bytes.len() as u16).to_be_bytes());
    data.extend_from_slice(ip_bytes);
    
    // TAG_Byte "hideAddress"
    data.push(0x01); // TAG_Byte
    data.extend_from_slice(&[0x00, 0x0B]);
    data.extend_from_slice(b"hideAddress");
    data.push(0x00);
    
    // End of server compound
    data.push(0x00);
    
    // End of root compound
    data.push(0x00);
    
    data
}