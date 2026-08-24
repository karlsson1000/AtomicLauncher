use crate::utils::get_instances_dir;
use std::fs;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screenshot {
    pub path: String,
    pub filename: String,
    pub instance_name: String,
    pub timestamp: i64,
    pub size: u64,
}

#[tauri::command]
pub async fn get_all_screenshots() -> Result<Vec<Screenshot>, String> {
    let instances_dir = get_instances_dir();

    tauri::async_runtime::spawn_blocking(move || {
        if !instances_dir.exists() {
            return Ok(Vec::new());
        }

        let mut screenshots = Vec::new();

        for entry in fs::read_dir(&instances_dir).map_err(|e| e.to_string())?.flatten() {
            let instance_path = entry.path();

            if !instance_path.is_dir() {
                continue;
            }

            let instance_name = instance_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let screenshots_dir = instance_path.join("screenshots");

            if !screenshots_dir.exists() {
                continue;
            }

            for screenshot_entry in fs::read_dir(&screenshots_dir).map_err(|e| e.to_string())?.flatten() {
                let screenshot_path = screenshot_entry.path();

                if !screenshot_path.is_file() {
                    continue;
                }

                let extension = screenshot_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");

                if !matches!(extension, "png" | "jpg" | "jpeg") {
                    continue;
                }

                let filename = screenshot_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                if let Ok(metadata) = fs::metadata(&screenshot_path) {
                    let timestamp = metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);

                    screenshots.push(Screenshot {
                        path: screenshot_path.to_string_lossy().to_string(),
                        filename,
                        instance_name: instance_name.clone(),
                        timestamp,
                        size: metadata.len(),
                    });
                }
            }
        }

        screenshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(screenshots)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_screenshot(path: String) -> Result<(), String> {
    let screenshot_path = PathBuf::from(&path);
    let instances_dir = get_instances_dir();
    let canonical_screenshot = screenshot_path
        .canonicalize()
        .map_err(|e| format!("Invalid screenshot path: {}", e))?;
    
    let canonical_instances = instances_dir
        .canonicalize()
        .map_err(|_| "Instances directory not found".to_string())?;
    
    if !canonical_screenshot.starts_with(&canonical_instances) {
        return Err("Invalid screenshot path".to_string());
    }

    if !path.contains("screenshots") {
        return Err("Invalid screenshot path".to_string());
    }

    if !screenshot_path.exists() {
        return Err("Screenshot does not exist".to_string());
    }

    fs::remove_file(&screenshot_path)
        .map_err(|e| format!("Failed to delete screenshot: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn open_screenshot(path: String) -> Result<(), String> {
    let screenshot_path = PathBuf::from(&path);
    let instances_dir = get_instances_dir();
    let canonical_screenshot = screenshot_path
        .canonicalize()
        .map_err(|e| format!("Invalid screenshot path: {}", e))?;
    
    let canonical_instances = instances_dir
        .canonicalize()
        .map_err(|_| "Instances directory not found".to_string())?;
    
    if !canonical_screenshot.starts_with(&canonical_instances) {
        return Err("Invalid screenshot path".to_string());
    }

    if !screenshot_path.exists() {
        return Err("Screenshot does not exist".to_string());
    }

    open::that(&screenshot_path)
        .map_err(|e| format!("Failed to open screenshot: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn open_screenshots_folder(instance_name: Option<String>) -> Result<(), String> {
    let instances_dir = get_instances_dir();
    
    if !instances_dir.exists() {
        return Err("Instances directory does not exist".to_string());
    }

    if let Some(name) = instance_name {
        let screenshots_dir = instances_dir.join(&name).join("screenshots");
        
        if screenshots_dir.exists() {
            open::that(&screenshots_dir)
                .map_err(|e| format!("Failed to open screenshots folder: {}", e))?;
            return Ok(());
        } else {
            return Err(format!("Screenshots folder for instance '{}' does not exist", name));
        }
    }

    for entry in fs::read_dir(&instances_dir).map_err(|e| e.to_string())?.flatten() {
        let instance_path = entry.path();
        
        if !instance_path.is_dir() {
            continue;
        }

        let screenshots_dir = instance_path.join("screenshots");
        
        if screenshots_dir.exists() {
            open::that(&screenshots_dir)
                .map_err(|e| format!("Failed to open screenshots folder: {}", e))?;
            return Ok(());
        }
    }

    open::that(&instances_dir)
        .map_err(|e| format!("Failed to open instances folder: {}", e))?;

    Ok(())
}