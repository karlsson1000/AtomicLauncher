use crate::commands::validation::{sanitize_server_name, validate_server_address};
use crate::services::accounts::AccountManager;
use crate::services::instance::InstanceManager;
use crate::models::AppConfig;
use crate::utils::{get_launcher_dir, get_instance_dir};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use std::io::Write;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

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
        .map_err(|e| e.to_string())?;
    
    let servers: Vec<ServerInfo> = serde_json::from_str(&content)
        .map_err(|e| e.to_string())?;
    
    Ok(servers)
}

#[tauri::command]
pub async fn add_server(
    name: String,
    address: String,
    port: u16,
) -> Result<(), String> {
    let safe_name = sanitize_server_name(&name)?;
    validate_server_address(&address)?;
    
    if port == 0 {
        return Err("Port cannot be 0".to_string());
    }
    
    let mut servers = get_servers().await?;
    
    if servers.iter().any(|s| s.name.to_lowercase() == safe_name.to_lowercase()) {
        return Err(format!("Server '{}' already exists", safe_name));
    }
    
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
    
    let servers_file = get_launcher_dir().join("servers.json");
    let json = serde_json::to_string_pretty(&servers)
        .map_err(|e| e.to_string())?;
    
    std::fs::write(&servers_file, json)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_server(server_name: String) -> Result<(), String> {
    let safe_name = sanitize_server_name(&server_name)?;
    
    let mut servers = get_servers().await?;
    
    let initial_len = servers.len();
    servers.retain(|s| s.name != safe_name);
    
    if servers.len() == initial_len {
        return Err(format!("Server '{}' not found", safe_name));
    }
    
    let servers_file = get_launcher_dir().join("servers.json");
    let json = serde_json::to_string_pretty(&servers)
        .map_err(|e| e.to_string())?;
    
    std::fs::write(&servers_file, json)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_server_status(
    server_name: String,
    status: ServerInfo,
) -> Result<(), String> {
    let safe_name = sanitize_server_name(&server_name)?;
    
    let mut servers = get_servers().await?;
    
    let server = servers.iter_mut()
        .find(|s| s.name == safe_name)
        .ok_or(format!("Server '{}' not found", safe_name))?;
    
    server.status = status.status;
    server.players_online = status.players_online;
    server.players_max = status.players_max;
    server.version = status.version;
    server.motd = status.motd;
    server.favicon = status.favicon;
    server.last_checked = Some(chrono::Utc::now().timestamp());
    
    let servers_file = get_launcher_dir().join("servers.json");
    let json = serde_json::to_string_pretty(&servers)
        .map_err(|e| e.to_string())?;
    
    std::fs::write(&servers_file, json)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reorder_servers(server_names: Vec<String>) -> Result<(), String> {
    let mut servers = get_servers().await?;

    let mut reordered: Vec<ServerInfo> = Vec::with_capacity(server_names.len());
    for name in &server_names {
        if let Some(idx) = servers.iter().position(|s| s.name == *name) {
            reordered.push(servers.remove(idx));
        }
    }

    reordered.extend(servers);

    let servers_file = get_launcher_dir().join("servers.json");
    let json = serde_json::to_string_pretty(&reordered)
        .map_err(|e| e.to_string())?;

    std::fs::write(&servers_file, json)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ping_server(address: String, port: u16) -> Result<u32, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let addr_str = format!("{}:{}", address, port);

        let socket_addr = addr_str
            .to_socket_addrs()
            .map_err(|e| format!("Failed to resolve address: {}", e))?
            .next()
            .ok_or_else(|| "No addresses found for host".to_string())?;

        let start = Instant::now();

        TcpStream::connect_timeout(&socket_addr, Duration::from_secs(5))
            .map_err(|e| format!("Connection failed: {}", e))?;

        Ok(start.elapsed().as_millis() as u32)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn launch_server(
    server_address: String,
    server_port: u16,
    server_name: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let safe_name = sanitize_server_name(&server_name)?;
    validate_server_address(&server_address)?;
    
    if server_port == 0 {
        return Err("Invalid server port".to_string());
    }

    let config = app_handle.state::<AppConfig>();

    let active_account = AccountManager::get_active_account()
        .map_err(|e| e.to_string())?
        .ok_or("No active account")?;

    let access_token = AccountManager::get_valid_token(&active_account.uuid, &config.microsoft_client_id)
        .await
        .map_err(|e| e.to_string())?;

    let instances = InstanceManager::get_all()
        .map_err(|e| e.to_string())?;
    
    let most_recent_instance = instances
        .iter()
        .filter(|inst| inst.last_played.is_some())
        .max_by_key(|inst| inst.last_played.as_ref());

    let instance_name = if let Some(recent_inst) = most_recent_instance {
        recent_inst.name.clone()
    } else {
        return Err("No instances found. Please create an instance first.".to_string());
    };

    let instance_dir = get_instance_dir(&instance_name);

    if !instance_dir.exists() {
        return Err(format!("Instance '{}' not found", instance_name));
    }

    add_server_to_instance(&instance_dir, &safe_name, &server_address, server_port)?;

    let server_arg = if server_port == 25565 {
        server_address.clone()
    } else {
        format!("{}:{}", server_address, server_port)
    };

    let _ = app_handle.emit("server-instance-launching", serde_json::json!({
        "instance": instance_name,
        "server": safe_name
    }));

    let username = active_account.username.clone();
    let uuid = active_account.uuid.clone();

    tauri::async_runtime::spawn_blocking(move || {
        InstanceManager::launch_with_server(
            &instance_name,
            &username,
            &uuid,
            &access_token,
            &server_arg,
            app_handle,
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn add_server_to_instance(
    instance_dir: &std::path::Path,
    server_name: &str,
    server_address: &str,
    server_port: u16,
) -> Result<(), String> {
    let servers_dat = instance_dir.join("servers.dat");

    if servers_dat.exists() {
        return Ok(());
    }
    
    let nbt_data = create_servers_nbt(server_name, server_address, server_port);
    
    let mut file = std::fs::File::create(&servers_dat)
        .map_err(|e| e.to_string())?;
    
    file.write_all(&nbt_data)
        .map_err(|e| e.to_string())?;

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