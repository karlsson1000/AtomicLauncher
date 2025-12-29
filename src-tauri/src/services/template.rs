use crate::models::{InstanceTemplate, MinecraftOptions};
use crate::utils::get_launcher_dir;
use chrono::Utc;
use std::{fs, path::PathBuf};

pub struct TemplateManager;

impl TemplateManager {
    fn get_templates_dir() -> PathBuf {
        get_launcher_dir().join("templates")
    }

    fn get_template_path(template_id: &str) -> PathBuf {
        Self::get_templates_dir().join(format!("{}.json", template_id))
    }

    pub fn create_template(
        name: String,
        description: Option<String>,
        launcher_settings: Option<crate::models::LauncherSettings>,
        minecraft_options: Option<MinecraftOptions>,
    ) -> Result<InstanceTemplate, Box<dyn std::error::Error>> {
        let templates_dir = Self::get_templates_dir();
        fs::create_dir_all(&templates_dir)?;

        // Generate a simple unique ID using timestamp and random component
        let timestamp = Utc::now().timestamp_millis();
        let random_part: u32 = (timestamp % 10000) as u32;
        let template_id = format!("{}-{}", timestamp, random_part);

        let template = InstanceTemplate {
            id: template_id,
            name,
            description,
            created_at: Utc::now().to_rfc3339(),
            launcher_settings,
            minecraft_options,
        };

        let template_path = Self::get_template_path(&template.id);
        let json = serde_json::to_string_pretty(&template)?;
        fs::write(&template_path, json)?;

        Ok(template)
    }

    pub fn get_all_templates() -> Result<Vec<InstanceTemplate>, Box<dyn std::error::Error>> {
        let templates_dir = Self::get_templates_dir();

        if !templates_dir.exists() {
            return Ok(Vec::new());
        }

        let mut templates = Vec::new();

        for entry in fs::read_dir(&templates_dir)? {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(template) = serde_json::from_str::<InstanceTemplate>(&content) {
                            templates.push(template);
                        }
                    }
                }
            }
        }

        templates.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(templates)
    }

    pub fn get_template(template_id: &str) -> Result<InstanceTemplate, Box<dyn std::error::Error>> {
        let template_path = Self::get_template_path(template_id);

        if !template_path.exists() {
            return Err(format!("Template '{}' not found", template_id).into());
        }

        let content = fs::read_to_string(&template_path)?;
        let template: InstanceTemplate = serde_json::from_str(&content)?;
        Ok(template)
    }

    pub fn update_template(template: InstanceTemplate) -> Result<(), Box<dyn std::error::Error>> {
        let template_path = Self::get_template_path(&template.id);

        if !template_path.exists() {
            return Err(format!("Template '{}' not found", template.id).into());
        }

        let json = serde_json::to_string_pretty(&template)?;
        fs::write(&template_path, json)?;

        Ok(())
    }

    pub fn delete_template(template_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let template_path = Self::get_template_path(template_id);

        if !template_path.exists() {
            return Err(format!("Template '{}' not found", template_id).into());
        }

        fs::remove_file(&template_path)?;
        Ok(())
    }

    pub fn create_from_instance(
        instance_name: &str,
        template_name: String,
        description: Option<String>,
    ) -> Result<InstanceTemplate, Box<dyn std::error::Error>> {
        let instance_dir = crate::utils::get_instance_dir(instance_name);

        if !instance_dir.exists() {
            return Err(format!("Instance '{}' not found", instance_name).into());
        }

        // Load instance metadata
        let instance_json = instance_dir.join("instance.json");
        let instance: crate::models::Instance =
            serde_json::from_str(&fs::read_to_string(&instance_json)?)?;

        // Read options.txt for Minecraft settings
        let options_path = instance_dir.join("options.txt");
        let minecraft_options = if options_path.exists() {
            Some(Self::parse_options_txt(&options_path)?)
        } else {
            None
        };

        Self::create_template(
            template_name,
            description,
            instance.settings_override,
            minecraft_options,
        )
    }

    fn parse_options_txt(path: &PathBuf) -> Result<MinecraftOptions, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let mut options = MinecraftOptions {
            fov: None,
            render_distance: None,
            max_fps: None,
            fullscreen: None,
            vsync: None,
            gui_scale: None,
            brightness: None,
            entity_shadows: None,
            particles: None,
            graphics: None,
            smooth_lighting: None,
            biome_blend: None,
            master_volume: None,
            music_volume: None,
            weather_volume: None,
            blocks_volume: None,
            hostile_volume: None,
            neutral_volume: None,
            players_volume: None,
            ambient_volume: None,
            voice_volume: None,
            mouse_sensitivity: None,
            invert_mouse: None,
            auto_jump: None,
            sneak_toggles: None,
            sprint_toggles: None,
            raw_input: None,
            keybinds: Some(std::collections::HashMap::new()),
        };

        for line in content.lines() {
            if let Some((key, value)) = line.split_once(':') {
                match key {
                    "fov" => options.fov = value.parse().ok(),
                    "renderDistance" => options.render_distance = value.parse().ok(),
                    "maxFps" => options.max_fps = value.parse().ok(),
                    "fullscreen" => options.fullscreen = Some(value == "true"),
                    "enableVsync" => options.vsync = Some(value == "true"),
                    "guiScale" => options.gui_scale = value.parse().ok(),
                    "gamma" => options.brightness = value.parse().ok(),
                    "entityShadows" => options.entity_shadows = Some(value == "true"),
                    "particles" => options.particles = Some(value.to_string()),
                    "graphicsMode" => options.graphics = Some(value.to_string()),
                    "ao" => options.smooth_lighting = Some(value != "0"),
                    "biomeBlendRadius" => options.biome_blend = value.parse().ok(),
                    "soundCategory_master" => options.master_volume = value.parse().ok(),
                    "soundCategory_music" => options.music_volume = value.parse().ok(),
                    "soundCategory_weather" => options.weather_volume = value.parse().ok(),
                    "soundCategory_block" => options.blocks_volume = value.parse().ok(),
                    "soundCategory_hostile" => options.hostile_volume = value.parse().ok(),
                    "soundCategory_neutral" => options.neutral_volume = value.parse().ok(),
                    "soundCategory_player" => options.players_volume = value.parse().ok(),
                    "soundCategory_ambient" => options.ambient_volume = value.parse().ok(),
                    "soundCategory_voice" => options.voice_volume = value.parse().ok(),
                    "mouseSensitivity" => options.mouse_sensitivity = value.parse().ok(),
                    "invertYMouse" => options.invert_mouse = Some(value == "true"),
                    "autoJump" => options.auto_jump = Some(value == "true"),
                    "toggleCrouch" => options.sneak_toggles = Some(value == "true"),
                    "toggleSprint" => options.sprint_toggles = Some(value == "true"),
                    "rawMouseInput" => options.raw_input = Some(value == "true"),
                    _ => {
                        if key.starts_with("key_") {
                            if let Some(ref mut keybinds) = options.keybinds {
                                keybinds.insert(key.to_string(), value.to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(options)
    }

    pub fn apply_template_to_instance(
        template_id: &str,
        instance_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let template = Self::get_template(template_id)?;
        let instance_dir = crate::utils::get_instance_dir(instance_name);

        if !instance_dir.exists() {
            return Err(format!("Instance '{}' not found", instance_name).into());
        }

        println!("Applying template '{}' to instance '{}'", template.name, instance_name);

        // Apply launcher settings
        if let Some(launcher_settings) = template.launcher_settings {
            println!("  → Applying launcher settings (RAM: {}MB)", launcher_settings.memory_mb);
            let instance_json = instance_dir.join("instance.json");
            let mut instance: crate::models::Instance =
                serde_json::from_str(&fs::read_to_string(&instance_json)?)?;

            instance.settings_override = Some(launcher_settings);

            let json = serde_json::to_string_pretty(&instance)?;
            fs::write(&instance_json, json)?;
            println!("  ✓ Launcher settings applied");
        }

        // Apply Minecraft options
        if let Some(minecraft_options) = template.minecraft_options {
            println!("  → Applying game options");
            let options_path = instance_dir.join("options.txt");
            
            // Read existing options if they exist
            let mut existing_options = if options_path.exists() {
                fs::read_to_string(&options_path)?
            } else {
                String::new()
            };

            // Merge template options into existing options
            Self::merge_options_txt(&mut existing_options, &minecraft_options)?;
            fs::write(&options_path, existing_options)?;
            println!("  ✓ Game options applied");
        }

        println!("✓ Template applied successfully to '{}'", instance_name);
        Ok(())
    }

    pub fn merge_options_txt(
        existing: &mut String,
        options: &MinecraftOptions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut lines: Vec<String> = existing.lines().map(|s| s.to_string()).collect();
        
        // Helper to update or add a line
        let update_line = |lines: &mut Vec<String>, key: &str, value: String| {
            if let Some(pos) = lines.iter().position(|l| l.starts_with(&format!("{}:", key))) {
                lines[pos] = format!("{}:{}", key, value);
            } else {
                lines.push(format!("{}:{}", key, value));
            }
        };

        if let Some(fov) = options.fov {
            update_line(&mut lines, "fov", fov.to_string());
        }
        if let Some(render_distance) = options.render_distance {
            update_line(&mut lines, "renderDistance", render_distance.to_string());
        }
        if let Some(max_fps) = options.max_fps {
            update_line(&mut lines, "maxFps", max_fps.to_string());
        }
        if let Some(fullscreen) = options.fullscreen {
            update_line(&mut lines, "fullscreen", fullscreen.to_string());
        }
        if let Some(vsync) = options.vsync {
            update_line(&mut lines, "enableVsync", vsync.to_string());
        }
        if let Some(gui_scale) = options.gui_scale {
            update_line(&mut lines, "guiScale", gui_scale.to_string());
        }
        if let Some(brightness) = options.brightness {
            update_line(&mut lines, "gamma", brightness.to_string());
        }
        if let Some(entity_shadows) = options.entity_shadows {
            update_line(&mut lines, "entityShadows", entity_shadows.to_string());
        }
        if let Some(ref particles) = options.particles {
            update_line(&mut lines, "particles", particles.clone());
        }
        if let Some(ref graphics) = options.graphics {
            update_line(&mut lines, "graphicsMode", graphics.clone());
        }
        if let Some(smooth_lighting) = options.smooth_lighting {
            update_line(&mut lines, "ao", if smooth_lighting { "2".to_string() } else { "0".to_string() });
        }
        if let Some(biome_blend) = options.biome_blend {
            update_line(&mut lines, "biomeBlendRadius", biome_blend.to_string());
        }
        if let Some(master_volume) = options.master_volume {
            update_line(&mut lines, "soundCategory_master", master_volume.to_string());
        }
        if let Some(music_volume) = options.music_volume {
            update_line(&mut lines, "soundCategory_music", music_volume.to_string());
        }
        if let Some(weather_volume) = options.weather_volume {
            update_line(&mut lines, "soundCategory_weather", weather_volume.to_string());
        }
        if let Some(blocks_volume) = options.blocks_volume {
            update_line(&mut lines, "soundCategory_block", blocks_volume.to_string());
        }
        if let Some(hostile_volume) = options.hostile_volume {
            update_line(&mut lines, "soundCategory_hostile", hostile_volume.to_string());
        }
        if let Some(neutral_volume) = options.neutral_volume {
            update_line(&mut lines, "soundCategory_neutral", neutral_volume.to_string());
        }
        if let Some(players_volume) = options.players_volume {
            update_line(&mut lines, "soundCategory_player", players_volume.to_string());
        }
        if let Some(ambient_volume) = options.ambient_volume {
            update_line(&mut lines, "soundCategory_ambient", ambient_volume.to_string());
        }
        if let Some(voice_volume) = options.voice_volume {
            update_line(&mut lines, "soundCategory_voice", voice_volume.to_string());
        }
        if let Some(mouse_sensitivity) = options.mouse_sensitivity {
            update_line(&mut lines, "mouseSensitivity", mouse_sensitivity.to_string());
        }
        if let Some(invert_mouse) = options.invert_mouse {
            update_line(&mut lines, "invertYMouse", invert_mouse.to_string());
        }
        if let Some(auto_jump) = options.auto_jump {
            update_line(&mut lines, "autoJump", auto_jump.to_string());
        }
        if let Some(sneak_toggles) = options.sneak_toggles {
            update_line(&mut lines, "toggleCrouch", sneak_toggles.to_string());
        }
        if let Some(sprint_toggles) = options.sprint_toggles {
            update_line(&mut lines, "toggleSprint", sprint_toggles.to_string());
        }
        if let Some(raw_input) = options.raw_input {
            update_line(&mut lines, "rawMouseInput", raw_input.to_string());
        }

        if let Some(ref keybinds) = options.keybinds {
            for (key, value) in keybinds {
                update_line(&mut lines, key, value.clone());
            }
        }

        *existing = lines.join("\n");
        Ok(())
    }
}