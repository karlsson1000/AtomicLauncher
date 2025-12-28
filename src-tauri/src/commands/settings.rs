use crate::commands::validation::{
    sanitize_instance_name, validate_java_path, validate_memory_allocation,
};
use crate::models::{Instance, LauncherSettings};
use crate::services::settings::SettingsManager;
use crate::utils::get_instance_dir;
use std::path::PathBuf;

#[tauri::command]
pub async fn get_settings() -> Result<LauncherSettings, String> {
    SettingsManager::load()
        .map_err(|e| format!("Failed to load settings: {}", e))
}

#[tauri::command]
pub async fn save_settings(settings: LauncherSettings) -> Result<String, String> {
    if let Some(ref java_path) = settings.java_path {
        validate_java_path(java_path)?;
    }
    
    validate_memory_allocation(settings.memory_mb as u64)?;
    
    SettingsManager::save(&settings)
        .map_err(|e| format!("Failed to save settings: {}", e))?;
    
    Ok("Settings saved successfully".to_string())
}

#[tauri::command]
pub async fn get_instance_settings(instance_name: String) -> Result<Option<LauncherSettings>, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    let instance_dir = get_instance_dir(&safe_name);
    let instance_json = instance_dir.join("instance.json");
    
    if !instance_json.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }
    
    let content = std::fs::read_to_string(&instance_json)
        .map_err(|e| format!("Failed to read instance data: {}", e))?;
    
    let instance: Instance = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse instance data: {}", e))?;
    
    Ok(instance.settings_override)
}

#[tauri::command]
pub async fn save_instance_settings(
    instance_name: String,
    settings: Option<LauncherSettings>,
) -> Result<String, String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    
    if let Some(ref s) = settings {
        if let Some(ref java_path) = s.java_path {
            validate_java_path(java_path)?;
        }
        validate_memory_allocation(s.memory_mb as u64)?;
    }
    
    let instance_dir = get_instance_dir(&safe_name);
    let instance_json = instance_dir.join("instance.json");
    
    if !instance_json.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }
    
    let content = std::fs::read_to_string(&instance_json)
        .map_err(|e| format!("Failed to read instance data: {}", e))?;
    
    let mut instance: Instance = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse instance data: {}", e))?;
    
    instance.settings_override = settings;
    
    let updated_json = serde_json::to_string_pretty(&instance)
        .map_err(|e| format!("Failed to serialize instance data: {}", e))?;
    
    std::fs::write(&instance_json, updated_json)
        .map_err(|e| format!("Failed to write instance data: {}", e))?;
    
    Ok("Instance settings saved successfully".to_string())
}

#[tauri::command]
pub async fn detect_java_installations() -> Result<Vec<String>, String> {
    let mut java_paths = Vec::new();
    
    #[cfg(target_os = "windows")]
    {
        let common_paths = vec![
            "C:\\Program Files\\Java",
            "C:\\Program Files (x86)\\Java",
            "C:\\Program Files\\Eclipse Adoptium",
            "C:\\Program Files\\Microsoft",
            "C:\\Program Files\\Zulu",
            "C:\\Program Files\\Amazon Corretto",
        ];
        
        for base_path in common_paths {
            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let javaw_path = path.join("bin").join("javaw.exe");
                        if javaw_path.exists() {
                            if let Some(path_str) = javaw_path.to_str() {
                                if validate_java_path(path_str).is_ok() {
                                    java_paths.push(path_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if let Ok(path_var) = std::env::var("PATH") {
            for path in path_var.split(';') {
                let javaw_path = PathBuf::from(path).join("javaw.exe");
                if javaw_path.exists() {
                    if let Some(path_str) = javaw_path.to_str() {
                        if validate_java_path(path_str).is_ok() && !java_paths.contains(&path_str.to_string()) {
                            java_paths.push(path_str.to_string());
                        }
                    }
                }
            }
        }
        
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            let javaw_path = PathBuf::from(java_home).join("bin").join("javaw.exe");
            if javaw_path.exists() {
                if let Some(path_str) = javaw_path.to_str() {
                    if validate_java_path(path_str).is_ok() && !java_paths.contains(&path_str.to_string()) {
                        java_paths.push(path_str.to_string());
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        let common_paths = vec![
            "/Library/Java/JavaVirtualMachines",
            "/System/Library/Java/JavaVirtualMachines",
        ];
        
        for base_path in common_paths {
            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let java_path = path.join("Contents").join("Home").join("bin").join("java");
                        if java_path.exists() {
                            if let Some(path_str) = java_path.to_str() {
                                if validate_java_path(path_str).is_ok() {
                                    java_paths.push(path_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            let java_path = PathBuf::from(java_home).join("bin").join("java");
            if java_path.exists() {
                if let Some(path_str) = java_path.to_str() {
                    if validate_java_path(path_str).is_ok() && !java_paths.contains(&path_str.to_string()) {
                        java_paths.push(path_str.to_string());
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        let common_paths = vec![
            "/usr/lib/jvm",
            "/usr/java",
            "/opt/java",
        ];
        
        for base_path in common_paths {
            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let java_path = path.join("bin").join("java");
                        if java_path.exists() {
                            if let Some(path_str) = java_path.to_str() {
                                if validate_java_path(path_str).is_ok() {
                                    java_paths.push(path_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            let java_path = PathBuf::from(java_home).join("bin").join("java");
            if java_path.exists() {
                if let Some(path_str) = java_path.to_str() {
                    if validate_java_path(path_str).is_ok() && !java_paths.contains(&path_str.to_string()) {
                        java_paths.push(path_str.to_string());
                    }
                }
            }
        }
    }
    
    java_paths.sort();
    java_paths.dedup();
    
    Ok(java_paths)
}