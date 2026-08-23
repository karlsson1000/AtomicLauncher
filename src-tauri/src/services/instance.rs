use crate::models::Instance;
use crate::utils::*;
use chrono::Utc;
use std::fs;

pub struct InstanceManager;

impl InstanceManager {
    pub fn create(
        instance_name: &str,
        version: &str,
        loader: Option<String>,
        loader_version: Option<String>,
    ) -> Result<Instance, Box<dyn std::error::Error>> {
        let instance_dir = get_instance_dir(instance_name);

        if instance_dir.exists() {
            return Err(format!("Instance '{}' already exists!", instance_name).into());
        }

        fs::create_dir_all(&instance_dir)?;
        fs::create_dir_all(instance_dir.join("saves"))?;
        fs::create_dir_all(instance_dir.join("resourcepacks"))?;
        fs::create_dir_all(instance_dir.join("shaderpacks"))?;
        fs::create_dir_all(instance_dir.join("mods"))?;
        fs::create_dir_all(instance_dir.join("logs"))?;

        let instance = Instance {
            name: instance_name.to_string(),
            version: version.to_string(),
            created_at: Utc::now().to_rfc3339(),
            last_played: None,
            loader,
            loader_version,
            settings_override: None,
            icon_path: None,
            total_playtime_seconds: 0,
        };

        let instance_json = serde_json::to_string_pretty(&instance)?;
        fs::write(instance_dir.join("instance.json"), instance_json)?;

        Ok(instance)
    }

    pub fn get_all() -> Result<Vec<Instance>, Box<dyn std::error::Error>> {
        let instances_dir = get_instances_dir();

        if !instances_dir.exists() {
            return Ok(Vec::new());
        }

        let mut instances = Vec::new();

        let entries = fs::read_dir(&instances_dir)?;

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    let instance_json = path.join("instance.json");
                    if instance_json.exists() {
                        if let Ok(content) = fs::read_to_string(instance_json) {
                            if let Ok(instance) = serde_json::from_str::<Instance>(&content) {
                                instances.push(instance);
                            }
                        }
                    }
                }
            }
        }

        Ok(instances)
    }

    pub fn delete(instance_name: &str, permanent: bool) -> Result<(), Box<dyn std::error::Error>> {
        let instance_dir = get_instance_dir(instance_name);

        if !instance_dir.exists() {
            return Err(format!("Instance '{}' does not exist", instance_name).into());
        }

        if permanent {
            fs::remove_dir_all(&instance_dir)?;
        } else {
            let trash_dir = get_trash_dir();
            fs::create_dir_all(&trash_dir)?;

            let timestamp = Utc::now().format("%Y%m%d%H%M%S");
            let folder_name = format!("{}_{}", instance_name, timestamp);
            let trash_path = trash_dir.join(&folder_name);

            if trash_path.exists() {
                fs::remove_dir_all(&trash_path)?;
            }

            fs::rename(&instance_dir, &trash_path)?;

            crate::services::trash::TrashManager::add_item(
                instance_name,
                "instance",
                &folder_name,
            )?;
        }

        Ok(())
    }

}
