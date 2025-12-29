use crate::models::{Instance, InstanceTemplate, LauncherSettings, MinecraftOptions};
use crate::services::template::TemplateManager;
use crate::utils::get_instance_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateExport {
    pub version: String,
    pub template: TemplateExportData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateExportData {
    pub name: String,
    pub description: Option<String>,
    pub launcher_settings: Option<LauncherSettings>,
    pub minecraft_options: Option<MinecraftOptions>,
}

#[command]
pub async fn create_template(
    name: String,
    description: Option<String>,
    launcher_settings: Option<LauncherSettings>,
    minecraft_options: Option<MinecraftOptions>,
) -> Result<InstanceTemplate, String> {
    TemplateManager::create_template(name, description, launcher_settings, minecraft_options)
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_templates() -> Result<Vec<InstanceTemplate>, String> {
    TemplateManager::get_all_templates().map_err(|e| e.to_string())
}

#[command]
pub async fn get_template(template_id: String) -> Result<InstanceTemplate, String> {
    TemplateManager::get_template(&template_id).map_err(|e| e.to_string())
}

#[command]
pub async fn update_template(template: InstanceTemplate) -> Result<(), String> {
    TemplateManager::update_template(template).map_err(|e| e.to_string())
}

#[command]
pub async fn delete_template(template_id: String) -> Result<(), String> {
    TemplateManager::delete_template(&template_id).map_err(|e| e.to_string())
}

#[command]
pub async fn create_template_from_instance(
    instance_name: String,
    template_name: String,
    description: Option<String>,
) -> Result<InstanceTemplate, String> {
    TemplateManager::create_from_instance(&instance_name, template_name, description)
        .map_err(|e| e.to_string())
}

#[command]
pub async fn apply_template_to_instance(
    template_id: String,
    instance_name: String,
) -> Result<(), String> {
    TemplateManager::apply_template_to_instance(&template_id, &instance_name)
        .map_err(|e| e.to_string())
}

#[command]
pub async fn create_instance_from_template(
    template_id: String,
    instance_name: String,
    minecraft_version: String,
    loader: String,
    loader_version: Option<String>,
) -> Result<Instance, String> {
    let template = TemplateManager::get_template(&template_id).map_err(|e| e.to_string())?;

    let instance_dir = get_instance_dir(&instance_name);
    if instance_dir.exists() {
        return Err(format!("Instance '{}' already exists", instance_name));
    }

    fs::create_dir_all(&instance_dir).map_err(|e| format!("Failed to create instance directory: {}", e))?;

    let instance = Instance {
        name: instance_name.clone(),
        version: minecraft_version,
        loader: Some(loader),
        loader_version,
        created_at: chrono::Utc::now().to_rfc3339(),
        last_played: None,
        icon_path: None,
        settings_override: template.launcher_settings,
    };

    let instance_json = instance_dir.join("instance.json");
    let json = serde_json::to_string_pretty(&instance)
        .map_err(|e| format!("Failed to serialize instance: {}", e))?;
    fs::write(&instance_json, json)
        .map_err(|e| format!("Failed to write instance.json: {}", e))?;

    if let Some(minecraft_options) = template.minecraft_options {
        let options_path = instance_dir.join("options.txt");
        let mut options_content = String::new();
        TemplateManager::merge_options_txt(&mut options_content, &minecraft_options)
            .map_err(|e| format!("Failed to create options.txt: {}", e))?;
        fs::write(&options_path, options_content)
            .map_err(|e| format!("Failed to write options.txt: {}", e))?;
    }

    Ok(instance)
}

#[command]
pub async fn export_template(
    template_id: String,
    export_path: String,
) -> Result<String, String> {
    let template = TemplateManager::get_template(&template_id)
        .map_err(|e| format!("Failed to get template: {}", e))?;

    let export = TemplateExport {
        version: "1.0.0".to_string(),
        template: TemplateExportData {
            name: template.name,
            description: template.description,
            launcher_settings: template.launcher_settings,
            minecraft_options: template.minecraft_options,
        },
    };

    let json = serde_json::to_string_pretty(&export)
        .map_err(|e| format!("Failed to serialize template: {}", e))?;

    fs::write(&export_path, json)
        .map_err(|e| format!("Failed to write template file: {}", e))?;

    Ok(format!("Template exported successfully to {}", export_path))
}

#[command]
pub async fn import_template(
    import_path: String,
) -> Result<InstanceTemplate, String> {
    let content = fs::read_to_string(&import_path)
        .map_err(|e| format!("Failed to read template file: {}", e))?;

    let export: TemplateExport = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse template file: {}", e))?;

    if export.version != "1.0.0" {
        return Err(format!("Unsupported template version: {}. Expected 1.0.0", export.version));
    }

    let template = TemplateManager::create_template(
        export.template.name,
        export.template.description,
        export.template.launcher_settings,
        export.template.minecraft_options,
    )
    .map_err(|e| format!("Failed to create template: {}", e))?;

    Ok(template)
}