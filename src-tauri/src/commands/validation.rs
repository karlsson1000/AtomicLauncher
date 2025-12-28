use std::path::PathBuf;

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

/// Sanitize filenames to prevent path traversal
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
    
    // Only allow .jar files for mods
    if !filename.ends_with(".jar") {
        return Err("Only .jar files are allowed".to_string());
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
    
    // Basic validation: alphanumeric, dots, hyphens, colons (for ports)
    if !address.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == ':') {
        return Err("Server address contains invalid characters".to_string());
    }
    
    Ok(())
}

/// Validate Java executable path
pub fn validate_java_path(path: &str) -> Result<(), String> {
    let path_buf = PathBuf::from(path);
    
    if !path_buf.exists() {
        return Err("Java path does not exist".to_string());
    }
    
    if !path_buf.is_file() {
        return Err("Java path must be a file".to_string());
    }
    
    let filename = path_buf.file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?;
    
    let valid_names = if cfg!(windows) {
        vec!["java.exe", "javaw.exe"]
    } else {
        vec!["java"]
    };
    
    if !valid_names.contains(&filename) {
        return Err(format!("Java executable must be named: {}", valid_names.join(" or ")));
    }
    
    let output = std::process::Command::new(path)
        .arg("-version")
        .output()
        .map_err(|e| format!("Failed to execute Java: {}", e))?;
    
    let version_output = String::from_utf8_lossy(&output.stderr);
    if !version_output.to_lowercase().contains("java") && 
       !version_output.to_lowercase().contains("openjdk") {
        return Err("Not a valid Java executable".to_string());
    }
    
    Ok(())
}

/// Validate download URL is from trusted sources
pub fn validate_download_url(url: &str) -> Result<url::Url, String> {
    let parsed_url = url::Url::parse(url)
        .map_err(|_| "Invalid URL format".to_string())?;
    
    if parsed_url.scheme() != "https" {
        return Err("Only HTTPS URLs are allowed".to_string());
    }
    
    let allowed_hosts = vec![
        "cdn.modrinth.com",
        "github.com",
        "raw.githubusercontent.com",
    ];
    
    let host = parsed_url.host_str()
        .ok_or("URL has no host")?;
    
    if !allowed_hosts.contains(&host) {
        return Err(format!("Downloads only allowed from: {}", allowed_hosts.join(", ")));
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
            memory_mb, system_memory * 80 / 100
        ));
    }
    
    Ok(())
}