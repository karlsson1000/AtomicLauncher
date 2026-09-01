use crate::services::trash::TrashManager;

#[tauri::command]
pub async fn empty_trash() -> Result<(), String> {
    TrashManager::empty_trash().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_trash_items() -> Result<Vec<crate::models::TrashItem>, String> {
    TrashManager::get_all().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_trash_size() -> Result<(usize, u64), String> {
    let items = TrashManager::get_all().map_err(|e| e.to_string())?;
    let total: u64 = items.iter().map(|i| i.size).sum();
    Ok((items.len(), total))
}
