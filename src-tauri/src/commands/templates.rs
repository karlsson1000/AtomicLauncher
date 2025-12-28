use crate::commands::validation::{
    sanitize_instance_name, validate_java_path, validate_memory_allocation,
};
use crate::models::{InstanceTemplate, LauncherSettings, MinecraftOptions};
use crate::services::template::TemplateManager;

#[tauri::command]
pub async fn create_template(
    name: String,
    description: Option<String>,
    launcher_settings: Option<LauncherSettings>,
    minecraft_options: Option<MinecraftOptions>,
) -> Result<InstanceTemplate, String> {
    if name.trim().is_empty() {
        return Err("Template name cannot be empty".to_string());
    }
    
    if name.len() > 100 {
        return Err("Template name too long (max 100 characters)".to_string());
    }
    
    if let Some(ref settings) = launcher_settings {
        if let Some(ref java_path) = settings.java_path {
            validate_java_path(java_path)?;
        }
        validate_memory_allocation(settings.memory_mb as u64)?;
    }
    
    TemplateManager::create_template(
        name,
        description,
        launcher_settings,
        minecraft_options,
    )
    .map_err(|e| format!("Failed to create template: {}", e))
}

#[tauri::command]
pub async fn get_templates() -> Result<Vec<InstanceTemplate>, String> {
    TemplateManager::get_all_templates()
        .map_err(|e| format!("Failed to get templates: {}", e))
}

#[tauri::command]
pub async fn get_template(template_id: String) -> Result<InstanceTemplate, String> {
    if !template_id.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err("Invalid template ID format".to_string());
    }
    
    TemplateManager::get_template(&template_id)
        .map_err(|e| format!("Failed to get template: {}", e))
}

#[tauri::command]
pub async fn update_template(template: InstanceTemplate) -> Result<String, String> {
    if template.name.trim().is_empty() {
        return Err("Template name cannot be empty".to_string());
    }
    
    if template.name.len() > 100 {
        return Err("Template name too long (max 100 characters)".to_string());
    }
    
    if let Some(ref settings) = template.launcher_settings {
        if let Some(ref java_path) = settings.java_path {
            validate_java_path(java_path)?;
        }
        validate_memory_allocation(settings.memory_mb as u64)?;
    }
    
    TemplateManager::update_template(template)
        .map_err(|e| format!("Failed to update template: {}", e))?;
    
    Ok("Template updated successfully".to_string())
}

#[tauri::command]
pub async fn delete_template(template_id: String) -> Result<String, String> {
    if !template_id.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err("Invalid template ID format".to_string());
    }
    
    TemplateManager::delete_template(&template_id)
        .map_err(|e| format!("Failed to delete template: {}", e))?;
    
    Ok("Template deleted successfully".to_string())
}

#[tauri::command]
pub async fn create_template_from_instance(
    instance_name: String,
    template_name: String,
    description: Option<String>,
) -> Result<InstanceTemplate, String> {
    let safe_instance_name = sanitize_instance_name(&instance_name)?;
    
    if template_name.trim().is_empty() {
        return Err("Template name cannot be empty".to_string());
    }
    
    if template_name.len() > 100 {
        return Err("Template name too long (max 100 characters)".to_string());
    }
    
    TemplateManager::create_from_instance(&safe_instance_name, template_name, description)
        .map_err(|e| format!("Failed to create template from instance: {}", e))
}

#[tauri::command]
pub async fn apply_template_to_instance(
    template_id: String,
    instance_name: String,
) -> Result<String, String> {
    if !template_id.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err("Invalid template ID format".to_string());
    }
    
    let safe_instance_name = sanitize_instance_name(&instance_name)?;
    
    TemplateManager::apply_template_to_instance(&template_id, &safe_instance_name)
        .map_err(|e| format!("Failed to apply template: {}", e))?;
    
    Ok(format!("Template applied successfully to instance '{}'", safe_instance_name))
}

#[tauri::command]
pub async fn create_instance_from_template(
    instance_name: String,
    version: String,
    template_id: String,
    loader: Option<String>,
    loader_version: Option<String>,
) -> Result<String, String> {
    // Import the create_instance function from instances module
    crate::commands::instances::create_instance(
        instance_name.clone(), 
        version, 
        loader, 
        loader_version
    ).await?;
    
    apply_template_to_instance(template_id, instance_name).await?;
    
    Ok("Instance created from template successfully".to_string())
}