use crate::commands::validation::sanitize_instance_name;
use crate::models::ImportableInstance;
use crate::services::import::InstanceImporter;

#[tauri::command]
pub async fn detect_importable_instances() -> Result<Vec<ImportableInstance>, String> {
    Ok(InstanceImporter::detect())
}

#[tauri::command]
pub async fn import_instance(
    source: String,
    source_path: String,
    name: String,
    target_name: String,
    mc_version: Option<String>,
    loader: Option<String>,
    loader_version: Option<String>,
    icon_path: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let safe_target = sanitize_instance_name(&target_name)?;

    if !std::path::Path::new(&source_path).is_absolute() {
        return Err("Invalid source path".to_string());
    }

    let detected = ImportableInstance {
        source,
        name,
        path: source_path,
        mc_version,
        loader,
        loader_version,
        size_bytes: 0,
        icon_path,
    };

    InstanceImporter::import(&detected, &safe_target, &app_handle)
        .await
        .map_err(|e| e.to_string())
}