use std::path::PathBuf;
use crate::models::DetectedJava;

/// Validate  Minecraft/Microsoft account UUID
pub fn validate_uuid(uuid: &str) -> Result<(), String> {
    if uuid.len() > 36 || !uuid.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err("Invalid UUID format".to_string());
    }
    Ok(())
}

/// Sanitize instance names to prevent path traversal
pub fn sanitize_instance_name(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Instance name cannot be empty".to_string());
    }

    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err("Instance name contains invalid characters".to_string());
    }

    if name.starts_with('.') {
        return Err("Instance name cannot start with a dot".to_string());
    }

    if name.contains('\0') {
        return Err("Instance name contains null bytes".to_string());
    }

    Ok(name.to_string())
}

/// Sanitize mod filenames (only .jar files)
pub fn sanitize_mod_filename(filename: &str) -> Result<String, String> {
    if filename.is_empty() {
        return Err("Filename cannot be empty".to_string());
    }

    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err("Filename contains invalid characters".to_string());
    }

    if filename.starts_with('.') {
        return Err("Filename cannot start with a dot".to_string());
    }

    if filename.contains('\0') {
        return Err("Filename contains null bytes".to_string());
    }

    if !filename.ends_with(".jar") {
        return Err("Only .jar files are allowed for mods".to_string());
    }

    Ok(filename.to_string())
}

/// Sanitize filenames without extension restriction (for temp downloads)
pub fn sanitize_filename(filename: &str) -> Result<String, String> {
    if filename.is_empty() {
        return Err("Filename cannot be empty".to_string());
    }

    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err("Filename contains invalid characters".to_string());
    }

    if filename.starts_with('.') {
        return Err("Filename cannot start with a dot".to_string());
    }

    if filename.contains('\0') {
        return Err("Filename contains null bytes".to_string());
    }

    Ok(filename.to_string())
}

/// Sanitize resource pack filenames (allow .zip and .jar files)
pub fn sanitize_resourcepack_filename(filename: &str) -> Result<String, String> {
    if filename.is_empty() {
        return Err("Filename cannot be empty".to_string());
    }

    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err("Filename contains invalid characters".to_string());
    }

    if filename.starts_with('.') {
        return Err("Filename cannot start with a dot".to_string());
    }

    if filename.contains('\0') {
        return Err("Filename contains null bytes".to_string());
    }

    let lower = filename.to_lowercase();
    if !lower.ends_with(".zip") && !lower.ends_with(".jar") {
        return Err("Only .zip or .jar files are allowed for resource packs".to_string());
    }

    Ok(filename.to_string())
}

/// Sanitize shader pack filenames (allow .zip and .jar files)
pub fn sanitize_shaderpack_filename(filename: &str) -> Result<String, String> {
    if filename.is_empty() {
        return Err("Filename cannot be empty".to_string());
    }

    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err("Filename contains invalid characters".to_string());
    }

    if filename.starts_with('.') {
        return Err("Filename cannot start with a dot".to_string());
    }

    if filename.contains('\0') {
        return Err("Filename contains null bytes".to_string());
    }

    let lower = filename.to_lowercase();
    if !lower.ends_with(".zip") && !lower.ends_with(".jar") {
        return Err("Only .zip or .jar files are allowed for shader packs".to_string());
    }

    Ok(filename.to_string())
}

/// Sanitize server names
pub fn sanitize_server_name(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Server name cannot be empty".to_string());
    }

    if name.len() > 100 {
        return Err("Server name too long (max 100 characters)".to_string());
    }

    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err("Server name contains invalid characters".to_string());
    }

    if name.contains('\0') {
        return Err("Server name contains null bytes".to_string());
    }

    Ok(name.to_string())
}

/// Validate server address
pub fn validate_server_address(address: &str) -> Result<(), String> {
    if address.is_empty() {
        return Err("Server address cannot be empty".to_string());
    }

    if address.len() > 255 {
        return Err("Server address too long".to_string());
    }

    if !address.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == ':') {
        return Err("Server address contains invalid characters".to_string());
    }

    Ok(())
}

/// Validate Java executable path and return version + architecture info
pub fn get_java_info(path: &str) -> Result<DetectedJava, String> {
    let path_buf = PathBuf::from(path);

    if !path_buf.exists() {
        return Err("Java path does not exist".to_string());
    }

    if !path_buf.is_file() {
        return Err("Java path must be a file".to_string());
    }

    let filename = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?;

    let valid_names = if cfg!(windows) {
        vec!["java.exe", "javaw.exe"]
    } else {
        vec!["java"]
    };

    if !valid_names.contains(&filename) {
        return Err(format!(
            "Java executable must be named: {}",
            valid_names.join(" or ")
        ));
    }

    let mut cmd = std::process::Command::new(path);
    cmd.arg("-XshowSettings:properties").arg("-version");

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute Java: {}", e))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);

    if !combined.to_lowercase().contains("java") && !combined.to_lowercase().contains("openjdk") {
        return Err("Not a valid Java executable".to_string());
    }

    let mut java_version = String::new();
    let mut full_version = String::new();
    let mut architecture = String::new();

    for line in combined.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("java.version = ") {
            full_version = val.to_string();
            java_version = parse_major_version_str(&full_version);
        } else if let Some(val) = line.strip_prefix("java.version=") {
            if full_version.is_empty() {
                full_version = val.to_string();
            }
            java_version = parse_major_version_str(&full_version);
        } else if let Some(val) = line.strip_prefix("os.arch = ") {
            architecture = val.to_string();
        } else if let Some(val) = line.strip_prefix("os.arch=") {
            if architecture.is_empty() {
                architecture = val.to_string();
            }
        }
    }

    let major_version: u32 = java_version.parse().map_err(|_| {
        format!("Could not parse Java major version from: {}", full_version)
    })?;

    Ok(DetectedJava {
        major_version,
        full_version,
        architecture,
        path: path_buf.to_string_lossy().to_string(),
    })
}

fn parse_major_version_str(version_str: &str) -> String {
    let parts: Vec<&str> = version_str.split('.').collect();
    if parts.is_empty() {
        return "0".to_string();
    }
    if parts[0] == "1" && parts.len() > 1 {
        parts[1].to_string()
    } else {
        parts[0].to_string()
    }
}

/// Validate download URL is from trusted sources
pub fn validate_download_url(url: &str) -> Result<url::Url, String> {
    let parsed_url = url::Url::parse(url).map_err(|_| "Invalid URL format".to_string())?;

    if parsed_url.scheme() != "https" {
        return Err("Only HTTPS URLs are allowed".to_string());
    }

    let allowed_hosts = [
        "cdn.modrinth.com",
        "github.com",
        "raw.githubusercontent.com",
        "edge.forgecdn.net",
        "mediafilez.forgecdn.net",
        "media.forgecdn.net",
    ];

    let host = parsed_url.host_str().ok_or("URL has no host")?;

    if !allowed_hosts.contains(&host) {
        return Err(format!(
            "Downloads only allowed from: {}",
            allowed_hosts.join(", ")
        ));
    }

    Ok(parsed_url)
}

/// Validate memory allocation against system memory
pub fn validate_memory_allocation(memory_mb: u64) -> Result<(), String> {
    use sysinfo::System;

    if memory_mb < 512 {
        return Err("Memory allocation must be at least 512MB".to_string());
    }

    let mut sys = System::new_all();
    sys.refresh_memory();
    let system_memory = sys.total_memory() / 1024 / 1024;

    if memory_mb > system_memory {
        return Err(format!(
            "Memory allocation ({} MB) exceeds system memory ({} MB)",
            memory_mb, system_memory
        ));
    }

    if memory_mb > (system_memory * 80 / 100) {
        return Err(format!(
            "Memory allocation ({} MB) is too high. Recommended maximum: {} MB (80% of system memory)",
            memory_mb,
            system_memory * 80 / 100
        ));
    }

    Ok(())
}
