use crate::services::installer::should_include_library;
use crate::models::{FabricProfileJson, Instance, VersionDetails};
use crate::utils::*;
use chrono::Utc;
use std::io::{BufRead, BufReader};
use std::{fs, process::{Command, Stdio}};
use tauri::Emitter;
use zip::ZipArchive;

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

        // Create instance directory structure
        fs::create_dir_all(&instance_dir)?;
        fs::create_dir_all(instance_dir.join("saves"))?;
        fs::create_dir_all(instance_dir.join("resourcepacks"))?;
        fs::create_dir_all(instance_dir.join("shaderpacks"))?;
        fs::create_dir_all(instance_dir.join("mods"))?;
        fs::create_dir_all(instance_dir.join("logs"))?;

        // Save instance metadata
        let instance = Instance {
            name: instance_name.to_string(),
            version: version.to_string(),
            created_at: Utc::now().to_rfc3339(),
            last_played: None,
            loader,
            loader_version,
            settings_override: None,
            icon_path: None,
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

    pub fn delete(instance_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let instance_dir = get_instance_dir(instance_name);

        if !instance_dir.exists() {
            return Err(format!("Instance '{}' does not exist", instance_name).into());
        }

        fs::remove_dir_all(&instance_dir)?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn rename(old_name: &str, new_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let old_dir = get_instance_dir(old_name);
        let new_dir = get_instance_dir(new_name);

        if !old_dir.exists() {
            return Err(format!("Instance '{}' does not exist", old_name).into());
        }

        if new_dir.exists() {
            return Err(format!("Instance '{}' already exists", new_name).into());
        }

        // Rename directory
        fs::rename(&old_dir, &new_dir)?;

        // Update metadata
        let instance_json = new_dir.join("instance.json");
        let mut instance: Instance = serde_json::from_str(&fs::read_to_string(&instance_json)?)?;

        instance.name = new_name.to_string();

        let updated_json = serde_json::to_string_pretty(&instance)?;
        fs::write(instance_json, updated_json)?;

        Ok(())
    }

    fn emit_error_log(app_handle: &tauri::AppHandle, instance_name: &str, error_msg: &str) {
        let _ = app_handle.emit("console-log", serde_json::json!({
            "instance": instance_name,
            "message": format!("ERROR: {}", error_msg),
            "type": "stderr"
        }));
    }

    fn get_java_version(java_path: &str) -> Result<u32, Box<dyn std::error::Error>> {
        let output = Command::new(java_path)
            .arg("-version")
            .output()?;
        
        let version_text = String::from_utf8_lossy(&output.stderr);
        
        // Try multiple parsing strategies
        for line in version_text.lines() {
            // Strategy 1: Modern format - openjdk 17.0.8 or java version "17.0.8"
            if let Some(captures) = line.split('"').nth(1) {
                if let Some(major) = Self::parse_major_version(captures) {
                    return Ok(major);
                }
            }
            
            // Strategy 2: Simple format - openjdk 21
            if line.starts_with("openjdk") || line.starts_with("java") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Some(major) = Self::parse_major_version(parts[1]) {
                        return Ok(major);
                    }
                }
            }
        }
        
        Err("Could not parse Java version from output".into())
    }

    fn parse_major_version(version_str: &str) -> Option<u32> {
        let parts: Vec<&str> = version_str.split('.').collect();
        
        if parts.is_empty() {
            return None;
        }
        
        // Handle old format: 1.8.0_xxx -> 8
        if parts[0] == "1" && parts.len() > 1 {
            parts[1].parse::<u32>().ok()
        } else {
            // Handle modern format: 17.0.8 -> 17
            parts[0].parse::<u32>().ok()
        }
    }

    fn get_required_java_version(minecraft_version: &str) -> u32 {
        let base_version = if let Some(pos) = minecraft_version.find('-') {
            &minecraft_version[..pos]
        } else {
            minecraft_version
        };

        let parts: Vec<&str> = base_version.split('.').collect();
        
        // New format: 26.x (snapshots after version naming change)
        if parts.len() >= 1 {
            if let Ok(major) = parts[0].parse::<u32>() {
                if major >= 26 {
                    return 25; // Minecraft 26+ requires Java 25
                }
            }
        }
        
        // Old format: 1.x.y
        if parts.len() >= 2 {
            if let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                if major == 1 {
                    // Check patch version for 1.20.5+
                    if minor == 20 && parts.len() >= 3 {
                        if let Ok(patch) = parts[2].parse::<u32>() {
                            if patch >= 5 {
                                return 21; // 1.20.5+ requires Java 21
                            }
                        }
                    }
                    
                    // Version-specific requirements
                    if minor >= 20 { return 17; } // 1.20-1.20.4 requires Java 17
                    if minor >= 18 { return 17; } // 1.18+ requires Java 17
                    if minor >= 17 { return 16; } // 1.17+ requires Java 16
                    if minor >= 16 { return 8; }  // 1.16 works with Java 8
                }
            }
        }

        8
    }

    // Regular launch
    pub fn launch(
        instance_name: &str,
        username: &str,
        uuid: &str,
        access_token: &str,
        app_handle: tauri::AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::launch_internal(instance_name, username, uuid, access_token, None, app_handle)
    }

    // Launch with server connection
    pub fn launch_with_server(
        instance_name: &str,
        username: &str,
        uuid: &str,
        access_token: &str,
        server_address: &str,
        app_handle: tauri::AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::launch_internal(instance_name, username, uuid, access_token, Some(server_address), app_handle)
    }

    // Internal launch method with optional server connection
    fn launch_internal(
        instance_name: &str,
        username: &str,
        uuid: &str,
        access_token: &str,
        server_address: Option<&str>,
        app_handle: tauri::AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("=== Launching Instance: {} ===", instance_name);
        if let Some(server) = server_address {
            println!("Server connection: {}", server);
        }

        let meta_dir = get_meta_dir();
        let instance_dir = get_instance_dir(instance_name);

        if !instance_dir.exists() {
            let err_msg = format!("Instance '{}' does not exist", instance_name);
            Self::emit_error_log(&app_handle, instance_name, &err_msg);
            return Err(err_msg.into());
        }

        // Load instance metadata
        let instance_json = instance_dir.join("instance.json");
        let instance: Instance = match fs::read_to_string(&instance_json) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(inst) => inst,
                Err(e) => {
                    let err_msg = format!("Failed to parse instance.json: {}", e);
                    Self::emit_error_log(&app_handle, instance_name, &err_msg);
                    return Err(err_msg.into());
                }
            },
            Err(e) => {
                let err_msg = format!("Failed to read instance.json: {}", e);
                Self::emit_error_log(&app_handle, instance_name, &err_msg);
                return Err(err_msg.into());
            }
        };

        let version = instance.version.clone();
        println!("Version: {}", version);
        println!("Username: {}", username);

        // Load settings
        let global_settings = crate::services::settings::SettingsManager::load()
            .unwrap_or_default();

        let effective_settings = if let Some(override_settings) = &instance.settings_override {
            override_settings.clone()
        } else {
            global_settings
        };

        // Use the settings for Java path
        let java_path = if let Some(custom_java) = &effective_settings.java_path {
            custom_java.clone()
        } else {
            match find_java() {
                Some(path) => path,
                None => {
                    let err_msg = "Java not found. Please install Java or specify a custom Java path in settings.";
                    Self::emit_error_log(&app_handle, instance_name, err_msg);
                    return Err(err_msg.into());
                }
            }
        };

        println!("Java found: {}", java_path);

        // Check Java version
        let required_java = Self::get_required_java_version(&version);
        println!("Required Java version: {}", required_java);

        match Self::get_java_version(&java_path) {
            Ok(java_version) => {
                println!("Detected Java version: {}", java_version);
                if java_version < required_java {
                    let err_msg = format!(
                        "Java {} detected, but Minecraft {} requires Java {} or higher. Please update Java in Settings.",
                        java_version, version, required_java
                    );
                    Self::emit_error_log(&app_handle, instance_name, &err_msg);
                    return Err(err_msg.into());
                }
            }
            Err(e) => {
                // Make this a hard error for critical versions
                if required_java >= 17 {
                    let err_msg = format!(
                        "Could not detect Java version: {}. Minecraft {} requires Java {} or higher. Please ensure Java is correctly installed.",
                        e, version, required_java
                    );
                    Self::emit_error_log(&app_handle, instance_name, &err_msg);
                    return Err(err_msg.into());
                } else {
                    // Only warn for older versions
                    let warning = format!("Could not detect Java version ({}). Proceeding with caution...", e);
                    println!("Warning: {}", warning);
                    Self::emit_error_log(&app_handle, instance_name, &format!("WARNING: {}", warning));
                }
            }
        }

        println!("RAM allocation: {}MB", effective_settings.memory_mb);

        // Check if this is a Fabric instance
        let is_fabric = version.contains("fabric-loader");
        println!("Is Fabric: {}", is_fabric);

        let versions_dir = meta_dir.join("versions").join(&version);
        let json_path = versions_dir.join(format!("{}.json", version));

        if !json_path.exists() {
            let err_msg = format!("Version {} is not installed!", version);
            Self::emit_error_log(&app_handle, instance_name, &err_msg);
            return Err(err_msg.into());
        }

        let json_content = match fs::read_to_string(&json_path) {
            Ok(content) => content,
            Err(e) => {
                let err_msg = format!("Failed to read version JSON: {}", e);
                Self::emit_error_log(&app_handle, instance_name, &err_msg);
                return Err(err_msg.into());
            }
        };

        let current_os = get_current_os();
        println!("Current OS: {}", current_os);

        // Parse the profile based on type
        let (main_class, base_version_id, all_libraries, assets_id) = if is_fabric {
            println!("Parsing as Fabric profile...");
            
            let fabric_profile: FabricProfileJson = match serde_json::from_str(&json_content) {
                Ok(profile) => profile,
                Err(e) => {
                    let err_msg = format!("Failed to parse Fabric profile: {}", e);
                    Self::emit_error_log(&app_handle, instance_name, &err_msg);
                    return Err(err_msg.into());
                }
            };
            
            println!("Fabric main class: {}", fabric_profile.main_class);
            println!("Inherits from: {}", fabric_profile.inherits_from);
            
            let base_version_dir = meta_dir.join("versions").join(&fabric_profile.inherits_from);
            let base_json_path = base_version_dir.join(format!("{}.json", fabric_profile.inherits_from));
            
            if !base_json_path.exists() {
                let err_msg = format!(
                    "Base Minecraft version {} not found! Please install it first.",
                    fabric_profile.inherits_from
                );
                Self::emit_error_log(&app_handle, instance_name, &err_msg);
                return Err(err_msg.into());
            }
            
            let base_json_content = match fs::read_to_string(&base_json_path) {
                Ok(content) => content,
                Err(e) => {
                    let err_msg = format!("Failed to read base version JSON: {}", e);
                    Self::emit_error_log(&app_handle, instance_name, &err_msg);
                    return Err(err_msg.into());
                }
            };
            
            let base_version: VersionDetails = match serde_json::from_str(&base_json_content) {
                Ok(version) => version,
                Err(e) => {
                    let err_msg = format!("Failed to parse base version: {}", e);
                    Self::emit_error_log(&app_handle, instance_name, &err_msg);
                    return Err(err_msg.into());
                }
            };
            
            println!("Loaded base Minecraft version: {}", base_version.id);
            
            let mut combined_libs = Vec::new();

            for lib in &fabric_profile.libraries {
                combined_libs.push((lib.name.clone(), lib.url.clone(), None));
            }

            for lib in &base_version.libraries {
                // Skip native libraries
                if lib.name.contains(":natives-") {
                    continue;
                }
                
                // Check rules if they exist
                if let Some(rules) = &lib.rules {
                    if !should_include_library(rules, &current_os) {
                        println!("Skipping base library {} (OS rules don't match)", lib.name);
                        continue;
                    }
                }
                
                if let Some(downloads) = &lib.downloads {
                    if let Some(artifact) = &downloads.artifact {
                        combined_libs.push((
                            lib.name.clone(),
                            String::new(),
                            Some(artifact.path.clone())
                        ));
                    }
                } else {
                    combined_libs.push((lib.name.clone(), String::new(), None));
                }
            }
            
            (
                fabric_profile.main_class,
                fabric_profile.inherits_from,
                combined_libs,
                base_version.assets,
            )
        } else {
            println!("Parsing as vanilla Minecraft profile...");
            
            let version_details: VersionDetails = match serde_json::from_str(&json_content) {
                Ok(details) => details,
                Err(e) => {
                    let err_msg = format!("Failed to parse Minecraft profile: {}", e);
                    Self::emit_error_log(&app_handle, instance_name, &err_msg);
                    return Err(err_msg.into());
                }
            };
            
            let mut libs = Vec::new();
            for lib in &version_details.libraries {
                // Skip native libraries
                if lib.name.contains(":natives-") {
                    continue;
                }
                
                // Check rules if they exist
                if let Some(rules) = &lib.rules {
                    if !should_include_library(rules, &current_os) {
                        println!("Skipping library {} (OS rules don't match)", lib.name);
                        continue;
                    }
                }
                
                if let Some(downloads) = &lib.downloads {
                    if let Some(artifact) = &downloads.artifact {
                        libs.push((
                            lib.name.clone(),
                            String::new(),
                            Some(artifact.path.clone())
                        ));
                    }
                } else {
                    libs.push((lib.name.clone(), String::new(), None));
                }
            }
            
            (
                version_details.main_class,
                version_details.id.clone(),
                libs,
                version_details.assets,
            )
        };

        // Create natives directory
        let natives_dir = instance_dir.join("natives");
        if let Err(e) = fs::create_dir_all(&natives_dir) {
            let err_msg = format!("Failed to create natives directory: {}", e);
            Self::emit_error_log(&app_handle, instance_name, &err_msg);
            return Err(err_msg.into());
        }

        // Load the base version to extract native libraries
        let base_version_dir = meta_dir.join("versions").join(&base_version_id);
        let base_json_path = base_version_dir.join(format!("{}.json", base_version_id));
        let base_json_content = match fs::read_to_string(&base_json_path) {
            Ok(content) => content,
            Err(e) => {
                let err_msg = format!("Failed to read base version JSON: {}", e);
                Self::emit_error_log(&app_handle, instance_name, &err_msg);
                return Err(err_msg.into());
            }
        };
        
        let base_version: VersionDetails = match serde_json::from_str(&base_json_content) {
            Ok(version) => version,
            Err(e) => {
                let err_msg = format!("Failed to parse base version: {}", e);
                Self::emit_error_log(&app_handle, instance_name, &err_msg);
                return Err(err_msg.into());
            }
        };
        
        let libraries_dir = meta_dir.join("libraries");
        
        println!("Extracting native libraries for OS: {}", current_os);
        let mut natives_extracted = 0;
        let mut natives_attempted = 0;
        
        for library in &base_version.libraries {
            let is_native = library.name.contains(":natives-");
            
            if !is_native {
                continue;
            }
            
            let platform_suffix = if library.name.contains(":natives-windows") {
                "windows"
            } else if library.name.contains(":natives-linux") {
                "linux"
            } else if library.name.contains(":natives-macos") || library.name.contains(":natives-osx") {
                "osx"
            } else {
                ""
            };
            
            if platform_suffix != current_os {
                continue;
            }
            
            if let Some(rules) = &library.rules {
                if !should_include_library(rules, &current_os) {
                    continue;
                }
            }
            
            if let Some(downloads) = &library.downloads {
                if let Some(artifact) = &downloads.artifact {
                    natives_attempted += 1;
                    let native_path = libraries_dir.join(&artifact.path);
                    
                    println!("  → Processing native: {} ({})", library.name, artifact.path);
                    
                    if native_path.exists() {
                        match fs::File::open(&native_path) {
                            Ok(file) => {
                                match ZipArchive::new(file) {
                                    Ok(mut archive) => {
                                        for i in 0..archive.len() {
                                            if let Ok(mut file) = archive.by_index(i) {
                                                let file_name = file.name().to_string();
                                                
                                                if file_name.ends_with('/') || file_name.starts_with("META-INF") {
                                                    continue;
                                                }
                                                
                                                let outpath = natives_dir.join(&file_name);
                                                
                                                if let Some(parent) = outpath.parent() {
                                                    let _ = fs::create_dir_all(parent);
                                                }
                                                
                                                if let Ok(mut outfile) = fs::File::create(&outpath) {
                                                    if std::io::copy(&mut file, &mut outfile).is_ok() {
                                                        natives_extracted += 1;
                                                    }
                                                }
                                            }
                                        }
                                        println!("    ✓ Extracted native library");
                                    }
                                    Err(e) => {
                                        let err_msg = format!("Failed to open native archive: {}", e);
                                        println!("    ✗ {}", err_msg);
                                        Self::emit_error_log(&app_handle, instance_name, &err_msg);
                                    }
                                }
                            }
                            Err(e) => {
                                let err_msg = format!("Failed to open native file: {}", e);
                                println!("    ✗ {}", err_msg);
                                Self::emit_error_log(&app_handle, instance_name, &err_msg);
                            }
                        }
                    } else {
                        let err_msg = format!(
                            "Native library not found: {}. This will cause LWJGL to fail!",
                            artifact.path
                        );
                        println!("    ✗ {}", err_msg);
                        Self::emit_error_log(&app_handle, instance_name, &err_msg);
                        return Err(format!(
                            "Native library missing: {}. Please reinstall Minecraft {}",
                            artifact.path, base_version_id
                        ).into());
                    }
                }
            }
        }
        
        println!("✓ Extracted {} native library files from {} native JARs", natives_extracted, natives_attempted);
        
        if natives_attempted == 0 {
            let err_msg = format!(
                "No native libraries found for OS '{}'. Minecraft cannot start without natives.",
                current_os
            );
            Self::emit_error_log(&app_handle, instance_name, &err_msg);
            return Err(format!(
                "{}. Please reinstall Minecraft version {}",
                err_msg, base_version_id
            ).into());
        }
        
        if natives_extracted == 0 && natives_attempted > 0 {
            let err_msg = format!(
                "Found {} native JARs but failed to extract any files. Check file permissions and disk space.",
                natives_attempted
            );
            Self::emit_error_log(&app_handle, instance_name, &err_msg);
            return Err(err_msg.into());
        }

        // Build classpath
        let mut classpath = Vec::new();
        println!("Building classpath from {} libraries...", all_libraries.len());
        
        for (lib_name, _lib_url, artifact_path) in all_libraries {
            let parts: Vec<&str> = lib_name.split(':').collect();
            if parts.len() != 3 {
                continue;
            }
            
            let (group, artifact, lib_version) = (parts[0], parts[1], parts[2]);
            
            let lib_path = if let Some(path) = artifact_path {
                libraries_dir.join(path)
            } else {
                let group_path = group.replace('.', "/");
                let jar_name = format!("{}-{}.jar", artifact, lib_version);
                libraries_dir
                    .join(&group_path)
                    .join(artifact)
                    .join(lib_version)
                    .join(&jar_name)
            };
            
            if lib_path.exists() {
                classpath.push(lib_path.to_string_lossy().to_string());
            } else {
                let warning = format!("Library not found: {}", lib_path.display());
                println!("Warning: {}", warning);
                Self::emit_error_log(&app_handle, instance_name, &format!("WARNING: {}", warning));
            }
        }

        let client_jar = meta_dir
            .join("versions")
            .join(&base_version_id)
            .join(format!("{}.jar", base_version_id));

        if !client_jar.exists() {
            let err_msg = format!(
                "Minecraft {} JAR not found at: {}",
                base_version_id,
                client_jar.display()
            );
            Self::emit_error_log(&app_handle, instance_name, &err_msg);
            return Err(err_msg.into());
        }

        classpath.push(client_jar.to_string_lossy().to_string());
        
        println!("Total classpath entries: {}", classpath.len());

        let classpath_separator = if cfg!(windows) { ";" } else { ":" };
        let classpath_str = classpath.join(classpath_separator);

        println!("Main class: {}", main_class);
        println!("Assets ID: {}", assets_id);
        println!("Natives directory: {}", natives_dir.display());
        println!("Starting Minecraft process...");

        let mut cmd = Command::new(&java_path);
        cmd.arg(format!("-Xmx{}M", effective_settings.memory_mb))
            .arg(format!("-Xms{}M", effective_settings.memory_mb))
            .arg(format!("-Djava.library.path={}", natives_dir.display()))
            .arg("-cp")
            .arg(&classpath_str);

        cmd.arg(&main_class)
            .arg("--username")
            .arg(username)
            .arg("--uuid")
            .arg(uuid)
            .arg("--accessToken")
            .arg(access_token)
            .arg("--version")
            .arg(&version)
            .arg("--gameDir")
            .arg(&instance_dir)
            .arg("--assetsDir")
            .arg(meta_dir.join("assets"))
            .arg("--assetIndex")
            .arg(&assets_id);

        // Add server connection arguments if provided
        if let Some(server) = server_address {
            // Parse version to determine which argument to use
            let use_quickplay = should_use_quickplay(&base_version_id);
            
            if use_quickplay {
                println!("Adding server connection: --quickPlayMultiplayer {}", server);
                cmd.arg("--quickPlayMultiplayer").arg(server);
            } else {
                println!("Adding server connection: --server {}", server);
                cmd.arg("--server").arg(server);
            }
        }

        fn should_use_quickplay(version: &str) -> bool {
            // Extract base version from fabric versions
            let base_version = if version.contains("fabric-loader") {
                // For Fabric
                version.split('-').last().unwrap_or(version)
            } else if version.contains('-') {
                // For other loaders
                version.split('-').next().unwrap_or(version)
            } else {
                version
            };
            
            println!("Checking version for quickplay: {} (extracted from: {})", base_version, version);
            
            // Parse version
            let parts: Vec<&str> = base_version.split('.').collect();
            
            if parts.len() >= 3 {
                if let (Ok(major), Ok(minor), Ok(patch)) = 
                    (parts[0].parse::<u32>(), parts[1].parse::<u32>(), parts[2].parse::<u32>()) 
                {
                    if major == 1 && minor == 20 && patch >= 5 {
                        println!("Version {}.{}.{} >= 1.20.5, using --quickPlayMultiplayer", major, minor, patch);
                        return true;
                    }
                    if major == 1 && minor > 20 {
                        println!("Version {}.{}.{} > 1.20, using --quickPlayMultiplayer", major, minor, patch);
                        return true;
                    }
                    if major > 1 {
                        println!("Version {}.{}.{} > 1.x, using --quickPlayMultiplayer", major, minor, patch);
                        return true;
                    }
                }
            } else if parts.len() == 2 {
                if let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    if major == 1 && minor > 20 {
                        println!("Version {}.{} > 1.20, using --quickPlayMultiplayer", major, minor);
                        return true;
                    }
                    if major > 1 {
                        println!("Version {}.{} > 1.x, using --quickPlayMultiplayer", major, minor);
                        return true;
                    }
                }
            }
            
            println!("Version {} < 1.20.5, using --server", base_version);
            false
        }

        cmd.current_dir(&instance_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                let err_msg = format!("Failed to spawn Minecraft process: {}. Check if Java path is correct: {}", e, java_path);
                Self::emit_error_log(&app_handle, instance_name, &err_msg);
                return Err(err_msg.into());
            }
        };

        let child_pid = child.id();
        println!("✓ Minecraft process started (PID: {:?})", child_pid);

        // Store the PID for later termination
        {
            let mut processes = crate::commands::instances::RUNNING_PROCESSES.lock().unwrap();
            processes.insert(instance_name.to_string(), child_pid);
        }

        // Capture stdout
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let instance_name_clone = instance_name.to_string();
            let app_handle_clone = app_handle.clone();
            
            std::thread::spawn(move || {
                for line in reader.lines() {
                    if let Ok(line) = line {
                        // Filter out any lines that might contain the access token
                        if !line.contains("accessToken") && !line.contains("MINECRAFT_ACCESS_TOKEN") {
                            println!("[STDOUT] {}", line);
                            let _ = app_handle_clone.emit("console-log", serde_json::json!({
                                "instance": instance_name_clone,
                                "message": line,
                                "type": "stdout"
                            }));
                        }
                    }
                }
            });
        }

        // Capture stderr
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let instance_name_clone = instance_name.to_string();
            let app_handle_clone = app_handle.clone();
            
            std::thread::spawn(move || {
                let mut has_shown_friendly_error = false;
                
                for line in reader.lines() {
                    if let Ok(line) = line {
                        // Filter out any lines that might contain the access token
                        if line.contains("accessToken") || line.contains("MINECRAFT_ACCESS_TOKEN") {
                            continue;
                        }
                        
                        // Check for common errors and show friendly messages
                        if !has_shown_friendly_error {
                            if line.contains("UnsupportedClassVersionError") {
                                let _ = app_handle_clone.emit("console-log", serde_json::json!({
                                    "instance": instance_name_clone,
                                    "message": "ERROR: Wrong Java version! This Minecraft version requires a newer Java version. Please update Java in Settings.",
                                    "type": "stderr"
                                }));
                                has_shown_friendly_error = true;
                            } else if line.contains("class file version 65.0") {
                                let _ = app_handle_clone.emit("console-log", serde_json::json!({
                                    "instance": instance_name_clone,
                                    "message": "ERROR: Java version too old! You need Java 21 or newer. Your current Java is too old.",
                                    "type": "stderr"
                                }));
                                has_shown_friendly_error = true;
                            } else if line.contains("class file version 61.0") {
                                let _ = app_handle_clone.emit("console-log", serde_json::json!({
                                    "instance": instance_name_clone,
                                    "message": "ERROR: Java version too old! You need Java 17 or newer. Your current Java is too old.",
                                    "type": "stderr"
                                }));
                                has_shown_friendly_error = true;
                            } else if line.contains("Could not find or load main class") {
                                let _ = app_handle_clone.emit("console-log", serde_json::json!({
                                    "instance": instance_name_clone,
                                    "message": "ERROR: Game files are corrupted or missing. Try reinstalling this Minecraft version.",
                                    "type": "stderr"
                                }));
                                has_shown_friendly_error = true;
                            } else if line.contains("java.lang.OutOfMemoryError") {
                                let _ = app_handle_clone.emit("console-log", serde_json::json!({
                                    "instance": instance_name_clone,
                                    "message": "ERROR: Not enough memory allocated! Increase RAM allocation in Settings.",
                                    "type": "stderr"
                                }));
                                has_shown_friendly_error = true;
                            }
                        }
                        
                        // Always log the actual error for advanced users
                        println!("[STDERR] {}", line);
                        let _ = app_handle_clone.emit("console-log", serde_json::json!({
                            "instance": instance_name_clone,
                            "message": line,
                            "type": "stderr"
                        }));
                    }
                }
            });
        }

        // Update last played time
        let mut updated_instance = instance.clone();
        updated_instance.last_played = Some(Utc::now().to_rfc3339());

        let updated_json = serde_json::to_string_pretty(&updated_instance)?;
        fs::write(instance_json, updated_json)?;

        println!("✓ Launch command completed successfully!");

        // Monitor process exit and emit event when it closes
        let instance_name_clone = instance_name.to_string();
        let app_handle_clone = app_handle.clone();
        std::thread::spawn(move || {
            let _ = child.wait();
            println!("Instance '{}' has exited", instance_name_clone);
            
            // Remove from running processes
            {
                let mut processes = crate::commands::instances::RUNNING_PROCESSES.lock().unwrap();
                processes.remove(&instance_name_clone);
            }
            
            let _ = app_handle_clone.emit("instance-exited", serde_json::json!({
                "instance": instance_name_clone
            }));
        });

        Ok(())
    }
}