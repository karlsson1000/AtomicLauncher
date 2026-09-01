use crate::models::{TrashItem, TrashIndex};
use crate::utils::{get_trash_dir, get_trash_index_path};
use chrono::{Utc, DateTime, Duration};
use std::fs;

pub struct TrashManager;

impl TrashManager {
    fn load_index() -> Result<TrashIndex, Box<dyn std::error::Error>> {
        let path = get_trash_index_path();
        if !path.exists() {
            return Ok(TrashIndex::default());
        }
        let content = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&content)?)
    }

    fn save_index(index: &TrashIndex) -> Result<(), Box<dyn std::error::Error>> {
        let path = get_trash_index_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(index)?;
        fs::write(&path, json)?;
        Ok(())
    }

    pub fn add_item(
        original_name: &str,
        original_type: &str,
        folder_name: &str,
    ) -> Result<TrashItem, Box<dyn std::error::Error>> {
        let trash_dir = get_trash_dir();
        let trash_path = trash_dir.join(folder_name);
        let size = if trash_path.is_file() {
            trash_path.metadata().map(|m| m.len()).unwrap_or(0)
        } else if trash_path.exists() {
            dir_size(&trash_path)
        } else {
            0
        };

        let item = TrashItem {
            id: folder_name.to_string(),
            original_name: original_name.to_string(),
            original_type: original_type.to_string(),
            folder_name: folder_name.to_string(),
            moved_at: Utc::now().to_rfc3339(),
            size,
        };

        let mut index = Self::load_index()?;
        index.items.push(item.clone());
        Self::save_index(&index)?;
        Ok(item)
    }

    pub fn move_file_to_trash(file_path: &std::path::Path, item_type: &str) -> Result<TrashItem, Box<dyn std::error::Error>> {
        let trash_dir = get_trash_dir();
        fs::create_dir_all(&trash_dir)?;

        let file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        let folder_name = format!("{}_{}_{}", item_type, Utc::now().format("%Y%m%d_%H%M%S%3f"), file_name);
        let trash_path = trash_dir.join(&folder_name);

        if fs::rename(file_path, &trash_path).is_err() {
            fs::copy(file_path, &trash_path)?;
            fs::remove_file(file_path)?;
        }

        Self::add_item(&file_name, item_type, &folder_name)
    }

    pub fn get_all() -> Result<Vec<TrashItem>, Box<dyn std::error::Error>> {
        let index = Self::load_index()?;
        let trash_dir = get_trash_dir();

        let mut valid = Vec::new();
        for item in &index.items {
            if trash_dir.join(&item.folder_name).exists() {
                valid.push(item.clone());
            }
        }

        if valid.len() != index.items.len() {
            let clean_index = TrashIndex { items: valid.clone() };
            Self::save_index(&clean_index)?;
        }

        valid.sort_by(|a, b| b.moved_at.cmp(&a.moved_at));
        Ok(valid)
    }

    pub fn empty_trash() -> Result<(), Box<dyn std::error::Error>> {
        let index = Self::load_index()?;
        let trash_dir = get_trash_dir();

        for item in &index.items {
            let path = trash_dir.join(&item.folder_name);
            if path.exists() {
                if path.is_dir() {
                    let _ = fs::remove_dir_all(&path);
                } else {
                    let _ = fs::remove_file(&path);
                }
            }
        }

        let empty = TrashIndex::default();
        Self::save_index(&empty)?;
        Ok(())
    }

    pub fn clean_old_items(days: u32) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut index = Self::load_index()?;
        let trash_dir = get_trash_dir();
        let cutoff = Utc::now() - Duration::days(days as i64);
        let mut removed = Vec::new();

        index.items.retain(|item| {
            let keep = match DateTime::parse_from_rfc3339(&item.moved_at) {
                Ok(t) => t.with_timezone(&Utc) > cutoff,
                Err(_) => true,
            };
            if !keep {
                let path = trash_dir.join(&item.folder_name);
                if path.exists() {
                    if path.is_dir() {
                        let _ = fs::remove_dir_all(&path);
                    } else {
                        let _ = fs::remove_file(&path);
                    }
                }
                removed.push(item.original_name.clone());
            }
            keep
        });

        Self::save_index(&index)?;
        Ok(removed)
    }

}

fn dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                total += dir_size(&path);
            } else if path.is_file() {
                total += path.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}
