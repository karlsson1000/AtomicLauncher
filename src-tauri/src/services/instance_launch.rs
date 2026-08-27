use crate::models::{FabricProfileJson, ForgeProfileJson, Instance, LauncherSettings, NeoForgeProfileJson, Rule, VersionDetails};
use crate::services::installer::should_include_library;
use crate::utils::*;
use chrono::Utc;
use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::{fs, path::PathBuf};
use tauri::Emitter;
use zip::ZipArchive;

struct ResolvedProfile {
    main_class: String,
    base_version_id: String,
    base_version: VersionDetails,
    libraries: Vec<(String, String, Option<String>)>,
    assets_id: String,
    is_neoforge: bool,
    is_forge: bool,
    jvm_arguments: Vec<String>,
    game_arguments: Vec<String>,
}

fn process_arguments_args(args: &[serde_json::Value], current_os: &str) -> Vec<String> {
    let mut result = Vec::new();
    for arg in args {
        match arg {
            serde_json::Value::String(s) => {
                if !s.is_empty() {
                    result.push(s.clone());
                }
            }
            serde_json::Value::Object(obj) => {
                let should_include = if let Some(rules_val) = obj.get("rules") {
                    if let Ok(rules) = serde_json::from_value::<Vec<Rule>>(rules_val.clone()) {
                        should_include_library(&rules, current_os)
                    } else {
                        true
                    }
                } else {
                    true
                };

                if should_include {
                    if let Some(value) = obj.get("value") {
                        match value {
                            serde_json::Value::String(s) => result.push(s.clone()),
                            serde_json::Value::Array(arr) => {
                                for v in arr {
                                    if let Some(s) = v.as_str() {
                                        if !s.is_empty() {
                                            result.push(s.to_string());
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
    result
}

fn substitute_arg(arg: &str, subs: &[(&str, &str)]) -> String {
    let mut result = arg.to_string();
    for (from, to) in subs {
        result = result.replace(from, to);
    }
    result
}

struct LaunchGuard<'a>(&'a str);

impl Drop for LaunchGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut processes) = crate::commands::instances::RUNNING_PROCESSES.lock() {
            if processes.get(self.0) == Some(&0) {
                processes.remove(self.0);
            }
        }
    }
}

impl super::instance::InstanceManager {
    fn emit_error_log(app_handle: &tauri::AppHandle, instance_name: &str, error_msg: &str) {
        let _ = app_handle.emit("console-log", serde_json::json!({
            "instance": instance_name,
            "message": format!("ERROR: {}", error_msg),
            "type": "stderr"
        }));
    }

    fn get_java_version(java_path: &str) -> Result<u32, Box<dyn std::error::Error>> {
        let mut cmd = Command::new(java_path);
        cmd.arg("-version");

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let output = cmd.output()?;

        let version_text = String::from_utf8_lossy(&output.stderr);

        for line in version_text.lines() {
            if let Some(captures) = line.split('"').nth(1) {
                if let Some(major) = Self::parse_major_version(captures) {
                    return Ok(major);
                }
            }

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

        if parts[0] == "1" && parts.len() > 1 {
            parts[1].parse::<u32>().ok()
        } else {
            parts[0].parse::<u32>().ok()
        }
    }

    fn get_required_java_version_from_meta(version: &str) -> Option<u32> {
        let meta_dir = get_meta_dir();
        let json_path = meta_dir.join("versions").join(version).join(format!("{}.json", version));
        let content = std::fs::read_to_string(&json_path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;

        let inherits_from = json.get("inheritsFrom").and_then(|v| v.as_str());
        let base_version = inherits_from.unwrap_or(version);

        if base_version != version {
            let base_path = meta_dir.join("versions").join(base_version).join(format!("{}.json", base_version));
            if let Ok(base_content) = std::fs::read_to_string(&base_path) {
                if let Ok(base_json) = serde_json::from_str::<serde_json::Value>(&base_content) {
                    if let Some(mv) = base_json.pointer("/javaVersion/majorVersion").and_then(|v| v.as_u64()) {
                        return Some(mv as u32);
                    }
                }
            }
        } else if let Some(mv) = json.pointer("/javaVersion/majorVersion").and_then(|v| v.as_u64()) {
            return Some(mv as u32);
        }

        None
    }

    fn get_required_java_version(minecraft_version: &str) -> u32 {
        Self::get_required_java_version_from_meta(minecraft_version)
            .unwrap_or_else(|| Self::get_required_java_version_fallback(minecraft_version))
    }

    fn get_required_java_version_fallback(minecraft_version: &str) -> u32 {
        let base_version = if let Some(pos) = minecraft_version.find('-') {
            &minecraft_version[..pos]
        } else {
            minecraft_version
        };

        let parts: Vec<&str> = base_version.split('.').collect();
        if parts.len() >= 1 {
            if let Ok(major) = parts[0].parse::<u32>() {
                if major >= 26 {
                    return 25;
                }
            }
        }

        if parts.len() >= 2 {
            if let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                if major == 1 {
                    if minor == 20 && parts.len() >= 3 {
                        if let Ok(patch) = parts[2].parse::<u32>() {
                            if patch >= 5 {
                                return 21;
                            }
                        }
                    }

                    if minor > 20 { return 21; }
                    if minor == 20 { return 17; }
                    if minor >= 18 { return 17; }
                    if minor == 17 { return 16; }
                    if minor <= 16 { return 8; }
                }
            }
        }

        8
    }

    pub fn launch(
        instance_name: &str,
        username: &str,
        uuid: &str,
        access_token: &str,
        app_handle: tauri::AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::launch_internal(instance_name, username, uuid, access_token, None, None, app_handle)
    }

    pub fn launch_with_server(
        instance_name: &str,
        username: &str,
        uuid: &str,
        access_token: &str,
        server_address: &str,
        app_handle: tauri::AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::launch_internal(instance_name, username, uuid, access_token, Some(server_address), None, app_handle)
    }

    pub fn launch_with_world(
        instance_name: &str,
        username: &str,
        uuid: &str,
        access_token: &str,
        world_name: &str,
        app_handle: tauri::AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::launch_internal(instance_name, username, uuid, access_token, None, Some(world_name), app_handle)
    }

    fn launch_internal(
        instance_name: &str,
        username: &str,
        uuid: &str,
        access_token: &str,
        server_address: Option<&str>,
        world_name: Option<&str>,
        app_handle: tauri::AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut processes = crate::commands::instances::RUNNING_PROCESSES
                .lock()
                .map_err(|e| format!("Failed to acquire process lock: {}", e))?;
            if let Some(pid) = processes.get(instance_name) {
                let err_msg = if *pid == 0 {
                    format!(
                        "Instance '{}' is still launching (a first-time Java download may be in progress)",
                        instance_name
                    )
                } else {
                    format!("Instance '{}' is already running", instance_name)
                };
                Self::emit_error_log(&app_handle, instance_name, &err_msg);
                return Err(err_msg.into());
            }
            processes.insert(instance_name.to_string(), 0);
        }

        let _guard = LaunchGuard(instance_name);

        Self::perform_launch(
            instance_name, username, uuid, access_token, server_address, world_name, &app_handle,
        )
    }

    fn perform_launch(
        instance_name: &str,
        username: &str,
        uuid: &str,
        access_token: &str,
        server_address: Option<&str>,
        world_name: Option<&str>,
        app_handle: &tauri::AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let meta_dir = get_meta_dir();
        let instance_dir = get_instance_dir(instance_name);

        if !instance_dir.exists() {
            let err_msg = format!("Instance '{}' does not exist", instance_name);
            Self::emit_error_log(app_handle, instance_name, &err_msg);
            return Err(err_msg.into());
        }

        let (instance, version) = Self::step_load_instance(instance_name, &instance_dir, app_handle)?;
        let required_java = Self::get_required_java_version(&version);
        let (java_path, effective_settings) =
            Self::step_resolve_java(instance_name, &instance, required_java, app_handle)?;
        Self::step_check_java(instance_name, &version, &java_path, required_java, app_handle)?;
        let resolved = Self::step_resolve_profile(instance_name, &version, &meta_dir, app_handle)?;
        Self::step_extract_natives(instance_name, &resolved, &meta_dir, app_handle)?;
        let classpath = Self::step_build_classpath(instance_name, &resolved.libraries, &meta_dir, app_handle)?;
        Self::step_launch(
            instance_name, username, uuid, access_token, server_address, world_name,
            &instance, &version, &java_path, &resolved,
            &classpath, &instance_dir, &meta_dir, app_handle,
            &effective_settings,
        )?;
        Ok(())
    }

    fn step_load_instance(
        _instance_name: &str,
        instance_dir: &PathBuf,
        _app_handle: &tauri::AppHandle,
    ) -> Result<(Instance, String), Box<dyn std::error::Error>> {
        let instance_json = instance_dir.join("instance.json");
        let content = fs::read_to_string(&instance_json)
            .map_err(|e| format!("Failed to read instance.json: {}", e))?;
        let instance: Instance = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse instance.json: {}", e))?;
        let version = instance.version.clone();
        Ok((instance, version))
    }

    fn step_resolve_java(
        instance_name: &str,
        instance: &Instance,
        required_java: u32,
        app_handle: &tauri::AppHandle,
    ) -> Result<(String, LauncherSettings), Box<dyn std::error::Error>> {
        let mut effective_settings = crate::services::settings::SettingsManager::load()
            .unwrap_or_default();

        let mut instance_java: Option<String> = None;
        if let Some(override_settings) = &instance.settings_override {
            effective_settings.memory_mb = override_settings.memory_mb;
            if let Some(java) = override_settings.java_path.as_deref() {
                if !java.trim().is_empty() {
                    instance_java = Some(java.to_string());
                }
            }
        }

        let java_path = if let Some(custom_java) = instance_java {
            custom_java
        } else {
            let global_preference = effective_settings
                .java_path
                .clone()
                .filter(|p| !p.trim().is_empty());

            let suitable_java = global_preference
                .into_iter()
                .chain(find_java())
                .filter(|path| {
                    Self::get_java_version(path)
                        .map(|found| Self::is_java_compatible(found, required_java))
                        .unwrap_or(false)
                })
                .next();

            match suitable_java {
                Some(path) => path,
                None => {
                    match tauri::async_runtime::block_on(super::java_runtime::ensure_java_for_launch(
                        required_java,
                        instance_name,
                        app_handle,
                    )) {
                        Ok(Some(managed)) => managed,
                        Ok(None) => {
                            Self::emit_error_log(app_handle, instance_name, &format!(
                                "No managed Java {} runtime is available for this platform.",
                                required_java
                            ));
                            find_java().ok_or_else(|| -> Box<dyn std::error::Error> {
                                "Java not found. Please install Java or specify a custom Java path in settings.".into()
                            })?
                        }
                        Err(e) => {
                            Self::emit_error_log(app_handle, instance_name, &format!(
                                "Automatic Java download failed: {}. Please install Java {} or specify a custom Java path in settings.",
                                e, required_java
                            ));
                            return Err(e.into());
                        }
                    }
                }
            }
        };

        Ok((java_path, effective_settings))
    }

    fn is_java_compatible(found_major: u32, required_major: u32) -> bool {
        if required_major <= 8 {
            found_major == required_major
        } else {
            found_major >= required_major
        }
    }

    fn step_check_java(
        instance_name: &str,
        version: &str,
        java_path: &str,
        required_java: u32,
        app_handle: &tauri::AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match Self::get_java_version(java_path) {
            Ok(java_version) => {
                if java_version < required_java {
                    let err_msg = format!(
                        "Java {} detected, but Minecraft {} requires Java {} or higher. Please update Java in Settings.",
                        java_version, version, required_java
                    );
                    Self::emit_error_log(app_handle, instance_name, &err_msg);
                    return Err(err_msg.into());
                }

                if required_java <= 8 && java_version != required_java {
                    let err_msg = format!(
                        "Java {} detected, but Minecraft {} requires Java {}.",
                        java_version, version, required_java
                    );
                    Self::emit_error_log(app_handle, instance_name, &err_msg);
                    return Err(err_msg.into());
                }
            }
            Err(e) => {
                if required_java >= 17 {
                    let err_msg = format!(
                        "Could not detect Java version: {}. Minecraft {} requires Java {} or higher. Please ensure Java is correctly installed.",
                        e, version, required_java
                    );
                    Self::emit_error_log(app_handle, instance_name, &err_msg);
                    return Err(err_msg.into());
                } else {
                    let warning = format!("Could not detect Java version ({}). Proceeding with caution...", e);
                    Self::emit_error_log(app_handle, instance_name, &format!("WARNING: {}", warning));
                }
            }
        }
        Ok(())
    }

    fn step_resolve_profile(
        instance_name: &str,
        version: &str,
        meta_dir: &PathBuf,
        app_handle: &tauri::AppHandle,
    ) -> Result<ResolvedProfile, Box<dyn std::error::Error>> {
        let is_fabric = version.contains("fabric-loader");
        let is_neoforge = version.starts_with("neoforge-");
        let is_forge = version.contains("-forge-");

        let versions_dir = meta_dir.join("versions").join(version);
        let json_path = versions_dir.join(format!("{}.json", version));

        if !json_path.exists() {
            let err_msg = format!("Version {} is not installed!", version);
            Self::emit_error_log(app_handle, instance_name, &err_msg);
            return Err(err_msg.into());
        }

        let json_content = fs::read_to_string(&json_path)
            .map_err(|e| format!("Failed to read version JSON: {}", e))?;

        let current_os = get_current_os();

        if is_fabric {
            Self::resolve_fabric_profile(instance_name, version, &json_content, &current_os, meta_dir, app_handle)
        } else if is_neoforge {
            Self::resolve_neoforge_profile(instance_name, version, &json_content, &current_os, meta_dir, app_handle)
        } else if is_forge {
            Self::resolve_forge_profile(instance_name, version, &json_content, &current_os, meta_dir, app_handle)
        } else {
            Self::resolve_vanilla_profile(instance_name, &json_content, app_handle)
        }
    }

    fn resolve_fabric_profile(
        instance_name: &str,
        _version: &str,
        json_content: &str,
        current_os: &str,
        meta_dir: &PathBuf,
        app_handle: &tauri::AppHandle,
    ) -> Result<ResolvedProfile, Box<dyn std::error::Error>> {
        let fabric_profile: FabricProfileJson = serde_json::from_str(json_content)
            .map_err(|e| format!("Failed to parse Fabric profile: {}", e))?;

        let base_version_dir = meta_dir.join("versions").join(&fabric_profile.inherits_from);
        let base_json_path = base_version_dir.join(format!("{}.json", fabric_profile.inherits_from));

        if !base_json_path.exists() {
            let err_msg = format!(
                "Base Minecraft version {} not found! Please install it first.",
                fabric_profile.inherits_from
            );
            Self::emit_error_log(app_handle, instance_name, &err_msg);
            return Err(err_msg.into());
        }

        let base_json_content = fs::read_to_string(&base_json_path)
            .map_err(|e| format!("Failed to read base version JSON: {}", e))?;

        let base_version: VersionDetails = serde_json::from_str(&base_json_content)
            .map_err(|e| format!("Failed to parse base version: {}", e))?;

        let assets_id = base_version.assets.clone();

        let mut combined_libs = Vec::new();
        let mut base_lib_names = HashSet::new();

        for lib in &base_version.libraries {
            if lib.name.contains(":natives-") {
                continue;
            }
            if let Some(rules) = &lib.rules {
                if !should_include_library(rules, current_os) {
                    continue;
                }
            }
            let parts: Vec<&str> = lib.name.split(':').collect();
            if parts.len() >= 2 {
                let lib_key = format!("{}:{}", parts[0], parts[1]);
                base_lib_names.insert(lib_key);
            }
        }

        for lib in &fabric_profile.libraries {
            let parts: Vec<&str> = lib.name.split(':').collect();
            if parts.len() >= 2 {
                let lib_key = format!("{}:{}", parts[0], parts[1]);
                if base_lib_names.contains(&lib_key) {
                    continue;
                }
            }
            combined_libs.push((lib.name.clone(), lib.url.clone(), None));
        }

        for lib in &base_version.libraries {
            if lib.name.contains(":natives-") {
                continue;
            }
            if let Some(rules) = &lib.rules {
                if !should_include_library(rules, current_os) {
                    continue;
                }
            }
            if let Some(downloads) = &lib.downloads {
                if let Some(artifact) = &downloads.artifact {
                    combined_libs.push((lib.name.clone(), String::new(), Some(artifact.path.clone())));
                }
            } else {
                combined_libs.push((lib.name.clone(), String::new(), None));
            }
        }

        Ok(ResolvedProfile {
            main_class: fabric_profile.main_class,
            base_version_id: fabric_profile.inherits_from,
            base_version,
            libraries: combined_libs,
            assets_id,
            is_neoforge: false,
            is_forge: false,
            jvm_arguments: Vec::new(),
            game_arguments: Vec::new(),
        })
    }

    fn resolve_neoforge_profile(
        instance_name: &str,
        _version: &str,
        json_content: &str,
        current_os: &str,
        meta_dir: &PathBuf,
        app_handle: &tauri::AppHandle,
    ) -> Result<ResolvedProfile, Box<dyn std::error::Error>> {
        let neoforge_profile: NeoForgeProfileJson = serde_json::from_str(json_content)
            .map_err(|e| format!("Failed to parse NeoForge profile: {}", e))?;

        let base_version_dir = meta_dir.join("versions").join(&neoforge_profile.inherits_from);
        let base_json_path = base_version_dir.join(format!("{}.json", neoforge_profile.inherits_from));

        if !base_json_path.exists() {
            let err_msg = format!(
                "Base Minecraft version {} not found! Please install it first.",
                neoforge_profile.inherits_from
            );
            Self::emit_error_log(app_handle, instance_name, &err_msg);
            return Err(err_msg.into());
        }

        let base_json_content = fs::read_to_string(&base_json_path)
            .map_err(|e| format!("Failed to read base version JSON: {}", e))?;

        let base_version: VersionDetails = serde_json::from_str(&base_json_content)
            .map_err(|e| format!("Failed to parse base version: {}", e))?;

        let assets_id = base_version.assets.clone();

        let mut combined_libs = Vec::new();
        let mut overlay_lib_keys = HashSet::new();

        for lib in &neoforge_profile.libraries {
            let parts: Vec<&str> = lib.name.split(':').collect();
            if parts.len() >= 2 {
                let lib_key = format!("{}:{}", parts[0], parts[1]);
                overlay_lib_keys.insert(lib_key);
            }
            combined_libs.push((lib.name.clone(), lib.url.clone().unwrap_or_default(), None));
        }

        for lib in &base_version.libraries {
            if lib.name.contains(":natives-") {
                continue;
            }
            if let Some(rules) = &lib.rules {
                if !should_include_library(rules, current_os) {
                    continue;
                }
            }
            let parts: Vec<&str> = lib.name.split(':').collect();
            if parts.len() >= 2 {
                let lib_key = format!("{}:{}", parts[0], parts[1]);
                if overlay_lib_keys.contains(&lib_key) {
                    continue;
                }
            }
            if let Some(downloads) = &lib.downloads {
                if let Some(artifact) = &downloads.artifact {
                    combined_libs.push((lib.name.clone(), String::new(), Some(artifact.path.clone())));
                }
            } else {
                combined_libs.push((lib.name.clone(), String::new(), None));
            }
        }

        let (jvm_arguments, game_arguments) = if let Some(args) = &neoforge_profile.arguments {
            (process_arguments_args(&args.jvm, current_os),
             process_arguments_args(&args.game, current_os))
        } else {
            (Vec::new(), Vec::new())
        };

        Ok(ResolvedProfile {
            main_class: neoforge_profile.main_class,
            base_version_id: neoforge_profile.inherits_from,
            base_version,
            libraries: combined_libs,
            assets_id,
            is_neoforge: true,
            is_forge: false,
            jvm_arguments,
            game_arguments,
        })
    }

    fn resolve_forge_profile(
        instance_name: &str,
        _version: &str,
        json_content: &str,
        current_os: &str,
        meta_dir: &PathBuf,
        app_handle: &tauri::AppHandle,
    ) -> Result<ResolvedProfile, Box<dyn std::error::Error>> {
        let forge_profile: ForgeProfileJson = serde_json::from_str(json_content)
            .map_err(|e| format!("Failed to parse Forge profile: {}", e))?;

        let base_version_dir = meta_dir.join("versions").join(&forge_profile.inherits_from);
        let base_json_path = base_version_dir.join(format!("{}.json", forge_profile.inherits_from));

        if !base_json_path.exists() {
            let err_msg = format!(
                "Base Minecraft version {} not found! Please install it first.",
                forge_profile.inherits_from
            );
            Self::emit_error_log(app_handle, instance_name, &err_msg);
            return Err(err_msg.into());
        }

        let base_json_content = fs::read_to_string(&base_json_path)
            .map_err(|e| format!("Failed to read base version JSON: {}", e))?;

        let base_version: VersionDetails = serde_json::from_str(&base_json_content)
            .map_err(|e| format!("Failed to parse base version: {}", e))?;

        let assets_id = base_version.assets.clone();

        let mut combined_libs = Vec::new();
        let mut overlay_lib_keys = HashSet::new();

        for lib in &forge_profile.libraries {
            let parts: Vec<&str> = lib.name.split(':').collect();
            if parts.len() >= 2 {
                let lib_key = format!("{}:{}", parts[0], parts[1]);
                overlay_lib_keys.insert(lib_key);
            }
            combined_libs.push((lib.name.clone(), lib.url.clone().unwrap_or_default(), None));
        }

        for lib in &base_version.libraries {
            if lib.name.contains(":natives-") {
                continue;
            }
            if let Some(rules) = &lib.rules {
                if !should_include_library(rules, current_os) {
                    continue;
                }
            }
            let parts: Vec<&str> = lib.name.split(':').collect();
            if parts.len() >= 2 {
                let lib_key = format!("{}:{}", parts[0], parts[1]);
                if overlay_lib_keys.contains(&lib_key) {
                    continue;
                }
            }
            if let Some(downloads) = &lib.downloads {
                if let Some(artifact) = &downloads.artifact {
                    combined_libs.push((lib.name.clone(), String::new(), Some(artifact.path.clone())));
                }
            } else {
                combined_libs.push((lib.name.clone(), String::new(), None));
            }
        }

        let (jvm_arguments, game_arguments) = if let Some(args) = &forge_profile.arguments {
            (process_arguments_args(&args.jvm, current_os),
             process_arguments_args(&args.game, current_os))
        } else {
            (Vec::new(), Vec::new())
        };

        Ok(ResolvedProfile {
            main_class: forge_profile.main_class,
            base_version_id: forge_profile.inherits_from,
            base_version,
            libraries: combined_libs,
            assets_id,
            is_neoforge: false,
            is_forge: true,
            jvm_arguments,
            game_arguments,
        })
    }

    fn resolve_vanilla_profile(
        _instance_name: &str,
        json_content: &str,
        _app_handle: &tauri::AppHandle,
    ) -> Result<ResolvedProfile, Box<dyn std::error::Error>> {
        let version_details: VersionDetails = serde_json::from_str(json_content)
            .map_err(|e| format!("Failed to parse Minecraft profile: {}", e))?;

        let current_os = get_current_os();
        let main_class = version_details.main_class.clone();
        let base_version_id = version_details.id.clone();
        let assets_id = version_details.assets.clone();

        let mut libs = Vec::new();
        for lib in &version_details.libraries {
            if lib.name.contains(":natives-") {
                continue;
            }
            if let Some(rules) = &lib.rules {
                if !should_include_library(rules, &current_os) {
                    continue;
                }
            }
            if let Some(downloads) = &lib.downloads {
                if let Some(artifact) = &downloads.artifact {
                    libs.push((lib.name.clone(), String::new(), Some(artifact.path.clone())));
                }
            } else {
                libs.push((lib.name.clone(), String::new(), None));
            }
        }

        Ok(ResolvedProfile {
            main_class,
            base_version_id,
            base_version: version_details,
            libraries: libs,
            assets_id,
            is_neoforge: false,
            is_forge: false,
            jvm_arguments: Vec::new(),
            game_arguments: Vec::new(),
        })
    }

    fn extract_native_jar(
        archive: &mut ZipArchive<fs::File>,
        natives_dir: &PathBuf,
    ) -> std::io::Result<usize> {
        let mut extracted = 0;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.name().ends_with('/') || file.name().starts_with("META-INF") {
                continue;
            }
            let Some(safe_name) = file.enclosed_name() else {
                continue;
            };
            let outpath = natives_dir.join(safe_name);
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
            extracted += 1;
        }
        Ok(extracted)
    }

    fn step_extract_natives(
        instance_name: &str,
        resolved: &ResolvedProfile,
        meta_dir: &PathBuf,
        app_handle: &tauri::AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let instance_dir = get_instance_dir(instance_name);
        let natives_dir = instance_dir.join("natives");
        fs::create_dir_all(&natives_dir)
            .map_err(|e| format!("Failed to create natives directory: {}", e))?;

        let current_os = get_current_os();
        let libraries_dir = meta_dir.join("libraries");

        let mut natives_extracted = 0;
        let mut natives_attempted = 0;

        for library in &resolved.base_version.libraries {
            let is_native_name = library.name.contains(":natives-");

            if is_native_name {
                let platform_suffix = if library.name.contains(":natives-windows") {
                    "windows"
                } else if library.name.contains(":natives-linux") {
                    "linux"
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

                        if native_path.exists() {
                            if let Ok(file) = fs::File::open(&native_path) {
                                if let Ok(mut archive) = ZipArchive::new(file) {
                                    match Self::extract_native_jar(&mut archive, &natives_dir) {
                                        Ok(count) => natives_extracted += count,
                                        Err(e) => Self::emit_error_log(app_handle, instance_name, &format!("Native extraction failed: {}", e)),
                                    }
                                } else {
                                    Self::emit_error_log(app_handle, instance_name, &format!("Failed to open native archive for {}", library.name));
                                }
                            } else {
                                Self::emit_error_log(app_handle, instance_name, &format!("Failed to open native file for {}", library.name));
                            }
                        } else {
                            Self::emit_error_log(app_handle, instance_name, &format!(
                                "Native library not found: {}. This will cause LWJGL to fail!",
                                artifact.path
                            ));
                            return Err(format!(
                                "Native library missing: {}. Please reinstall Minecraft {}",
                                artifact.path, resolved.base_version_id
                            ).into());
                        }
                    }
                }
            }

            if let Some(downloads) = &library.downloads {
                if let Some(classifiers) = &downloads.classifiers {
                    for (key, artifact) in classifiers {
                        let platform_suffix = if key.contains("natives-windows") {
                            "windows"
                        } else if key.contains("natives-linux") {
                            "linux"
                        } else {
                            continue;
                        };

                        if platform_suffix != current_os {
                            continue;
                        }

                        if let Some(rules) = &library.rules {
                            if !should_include_library(rules, &current_os) {
                                continue;
                            }
                        }

                        natives_attempted += 1;
                        let native_path = libraries_dir.join(&artifact.path);

                        if native_path.exists() {
                            if let Ok(file) = fs::File::open(&native_path) {
                                if let Ok(mut archive) = ZipArchive::new(file) {
                                    match Self::extract_native_jar(&mut archive, &natives_dir) {
                                        Ok(count) => natives_extracted += count,
                                        Err(e) => Self::emit_error_log(app_handle, instance_name, &format!("Native extraction failed: {}", e)),
                                    }
                                } else {
                                    Self::emit_error_log(app_handle, instance_name, &format!("Failed to open native archive for classifier {}", key));
                                }
                            } else {
                                Self::emit_error_log(app_handle, instance_name, &format!("Failed to open native file for classifier {}", key));
                            }
                        } else {
                            Self::emit_error_log(app_handle, instance_name, &format!(
                                "Native library not found: {}. This will cause LWJGL to fail!",
                                artifact.path
                            ));
                            return Err(format!(
                                "Native library missing: {}. Please reinstall Minecraft {}",
                                artifact.path, resolved.base_version_id
                            ).into());
                        }
                    }
                }
            }
        }

        if natives_attempted == 0 {
            let err_msg = format!(
                "No native libraries found for OS '{}'. Minecraft cannot start without natives.",
                current_os
            );
            Self::emit_error_log(app_handle, instance_name, &err_msg);
            return Err(format!(
                "{}. Please reinstall Minecraft version {}",
                err_msg, resolved.base_version_id
            ).into());
        }

        if natives_extracted == 0 && natives_attempted > 0 {
            let err_msg = format!(
                "Found {} native JARs but failed to extract any files. Check file permissions and disk space.",
                natives_attempted
            );
            Self::emit_error_log(app_handle, instance_name, &err_msg);
            return Err(err_msg.into());
        }

        Ok(())
    }

    fn step_build_classpath(
        instance_name: &str,
        all_libraries: &[(String, String, Option<String>)],
        meta_dir: &PathBuf,
        app_handle: &tauri::AppHandle,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let libraries_dir = meta_dir.join("libraries");
        let mut classpath = Vec::new();

        for (lib_name, _lib_url, artifact_path) in all_libraries {
            let parts: Vec<&str> = lib_name.split(':').collect();
            if parts.len() < 3 || parts.len() > 4 {
                continue;
            }

            let (group, artifact, lib_version_raw) = (parts[0], parts[1], parts[2]);
            let classifier_raw = if parts.len() == 4 { parts[3] } else { "" };
            let classifier = classifier_raw.split('@').next().unwrap_or("");
            let classifier = if classifier.is_empty() { None } else { Some(classifier) };
            let lib_version = lib_version_raw.split('@').next().unwrap_or(lib_version_raw);
            if lib_version.is_empty() {
                continue;
            }

            let lib_path = if let Some(path) = artifact_path {
                libraries_dir.join(path)
            } else {
                let group_path = group.replace('.', "/");
                let jar_name = if let Some(cls) = classifier {
                    format!("{}-{}-{}.jar", artifact, lib_version, cls)
                } else {
                    format!("{}-{}.jar", artifact, lib_version)
                };
                libraries_dir
                    .join(&group_path)
                    .join(artifact)
                    .join(lib_version)
                    .join(&jar_name)
            };

            if lib_path.exists() {
                classpath.push(lib_path.to_string_lossy().to_string());
            } else {
                Self::emit_error_log(app_handle, instance_name, &format!("WARNING: Library not found: {}", lib_path.display()));
            }
        }

        Ok(classpath)
    }

    fn split_java_args(input: &str) -> Vec<String> {
        let mut args: Vec<String> = Vec::new();
        let mut current = String::new();
        let mut quote_char: Option<char> = None;

        for ch in input.chars() {
            match quote_char {
                Some(q) => {
                    if ch == q {
                        quote_char = None;
                    } else {
                        current.push(ch);
                    }
                }
                None => {
                    if ch == '"' || ch == '\'' {
                        quote_char = Some(ch);
                    } else if ch.is_whitespace() {
                        if !current.is_empty() {
                            args.push(std::mem::take(&mut current));
                        }
                    } else {
                        current.push(ch);
                    }
                }
            }
        }

        if !current.is_empty() {
            args.push(current);
        }

        args
    }

    fn step_launch(
        instance_name: &str,
        username: &str,
        uuid: &str,
        access_token: &str,
        server_address: Option<&str>,
        world_name: Option<&str>,
        instance: &Instance,
        version: &str,
        java_path: &str,
        resolved: &ResolvedProfile,
        classpath: &[String],
        instance_dir: &PathBuf,
        meta_dir: &PathBuf,
        app_handle: &tauri::AppHandle,
        effective_settings: &LauncherSettings,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client_jar = meta_dir
            .join("versions")
            .join(&resolved.base_version_id)
            .join(format!("{}.jar", resolved.base_version_id));

        if !client_jar.exists() {
            let err_msg = format!(
                "Minecraft {} JAR not found at: {}",
                resolved.base_version_id,
                client_jar.display()
            );
            Self::emit_error_log(app_handle, instance_name, &err_msg);
            return Err(err_msg.into());
        }

        let classpath_sep = if cfg!(windows) { ";" } else { ":" };

        let natives_dir = get_instance_dir(instance_name).join("natives");
        let libraries_dir = meta_dir.join("libraries");

        let natives_dir_str = natives_dir.to_string_lossy().into_owned();
        let libraries_dir_str = libraries_dir.to_string_lossy().into_owned();
        let instance_dir_str = instance_dir.to_string_lossy().into_owned();
        let assets_root = meta_dir.join("assets");
        let assets_root_str = assets_root.to_string_lossy().into_owned();
        let subs: &[(&str, &str)] = &[
            ("${natives_directory}", &natives_dir_str),
            ("${library_directory}", &libraries_dir_str),
            ("${classpath_separator}", classpath_sep),
            ("${launcher_name}", "octane-launcher"),
            ("${launcher_version}", "0.1.0"),
            ("${version_name}", &resolved.base_version_id),
            ("${game_directory}", &instance_dir_str),
            ("${assets_root}", &assets_root_str),
            ("${assets_index_name}", &resolved.assets_id),
            ("${auth_player_name}", username),
            ("${auth_uuid}", uuid),
            ("${auth_access_token}", access_token),
            ("${user_properties}", "{}"),
            ("${user_type}", "msa"),
            ("${version_type}", "release"),
        ];

        let mut full_classpath = classpath.to_vec();
        if !(resolved.is_neoforge || resolved.is_forge) {
            full_classpath.push(client_jar.to_string_lossy().to_string());
        }

        if resolved.is_neoforge || resolved.is_forge {
            let substituted_jvm: Vec<String> = resolved
                .jvm_arguments
                .iter()
                .map(|a| substitute_arg(a, subs))
                .collect();

            let mut module_path_files: HashSet<String> = HashSet::new();
            let mut expect_paths = false;
            for arg in &substituted_jvm {
                if expect_paths {
                    expect_paths = false;
                    for part in arg.split(classpath_sep) {
                        if let Some(name) = std::path::Path::new(part)
                            .file_name()
                            .and_then(|n| n.to_str())
                        {
                            module_path_files.insert(name.to_lowercase());
                        }
                    }
                    continue;
                }
                if arg == "-p" || arg == "--module-path" {
                    expect_paths = true;
                    continue;
                }
                if let Some(rest) = arg.strip_prefix("--module-path=") {
                    for part in rest.split(classpath_sep) {
                        if let Some(name) = std::path::Path::new(part)
                            .file_name()
                            .and_then(|n| n.to_str())
                        {
                            module_path_files.insert(name.to_lowercase());
                        }
                    }
                }
            }

            if !module_path_files.is_empty() {
                full_classpath.retain(|cp| {
                    let file_name = std::path::Path::new(cp)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_lowercase());
                    match file_name {
                        Some(n) => !module_path_files.contains(&n),
                        None => true,
                    }
                });
            }
        }

        let classpath_str = full_classpath.join(classpath_sep);

        let xms = (effective_settings.memory_mb * 80 / 100).max(512);

        let mut cmd = Command::new(java_path);
        cmd.arg(format!("-Xms{}M", xms))
            .arg(format!("-Xmx{}M", effective_settings.memory_mb))
            .arg("-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump")
            .arg("-Dminecraft.launcher.brand=octane-launcher")
            .arg(format!("-Dminecraft.launcher.version={}", env!("CARGO_PKG_VERSION")));

        if !resolved.is_forge && !resolved.is_neoforge {
            cmd.arg("-DFabricMcEmu=net.minecraft.client.main.Main");
        }

        if resolved.is_neoforge || resolved.is_forge {
            for arg in &resolved.jvm_arguments {
                cmd.arg(substitute_arg(arg, subs));
            }
            cmd.arg("--add-opens").arg("java.base/java.lang=ALL-UNNAMED")
                .arg("--add-opens").arg("java.base/java.lang.invoke=ALL-UNNAMED")
                .arg("--add-opens").arg("java.base/java.lang.reflect=ALL-UNNAMED")
                .arg("--add-opens").arg("java.base/java.io=ALL-UNNAMED")
                .arg("--add-opens").arg("java.base/java.nio=ALL-UNNAMED")
                .arg("--add-opens").arg("java.base/java.nio.file=ALL-UNNAMED")
                .arg("--add-opens").arg("java.base/java.util=ALL-UNNAMED")
                .arg("--add-opens").arg("java.base/java.util.jar=ALL-UNNAMED")
                .arg("--add-opens").arg("java.base/java.util.zip=ALL-UNNAMED")
                .arg("--add-opens").arg("java.base/sun.nio.ch=ALL-UNNAMED")
                .arg("--add-opens").arg("jdk.zipfs/jdk.nio.zipfs=ALL-UNNAMED")
                .arg("--add-opens").arg("java.base/sun.security.util=ALL-UNNAMED")
                .arg("--add-exports").arg("java.base/sun.security.util=ALL-UNNAMED")
                .arg("--add-exports").arg("jdk.naming.dns/com.sun.jndi.dns=ALL-UNNAMED,java.naming")
                .arg("--enable-native-access=ALL-UNNAMED");
            cmd.arg(format!("-Djava.library.path={}", natives_dir.display()))
                .arg(format!("-DlibraryDirectory={}", libraries_dir.display()))
                .arg(format!("-Dminecraft.client.jar={}", client_jar.display()))
                .arg("-Dfml.earlyprogresswindow=false");

            if resolved.is_neoforge {
                let nf_version_dir = libraries_dir
                    .join("net")
                    .join("neoforged")
                    .join("neoforge")
                    .join(version);
                let mut plugin_layer: Vec<String> = Vec::new();
                let mut game_layer: Vec<String> = Vec::new();
                if nf_version_dir.exists() {
                    if let Ok(entries) = fs::read_dir(&nf_version_dir) {
                        for entry in entries.flatten() {
                            let name = entry.file_name().to_string_lossy().to_lowercase();
                            if name.ends_with("-universal.jar") {
                                plugin_layer.push(entry.path().to_string_lossy().into_owned());
                            } else if name.ends_with("-client.jar") {
                                game_layer.push(entry.path().to_string_lossy().into_owned());
                            }
                        }
                    }
                }
                if !plugin_layer.is_empty() {
                    cmd.arg(format!(
                        "-Dfml.pluginLayerLibraries={}",
                        plugin_layer.join(classpath_sep)
                    ));
                }
                if !game_layer.is_empty() {
                    cmd.arg(format!(
                        "-Dfml.gameLayerLibraries={}",
                        game_layer.join(classpath_sep)
                    ));
                }
            }
        } else {
            cmd.arg(format!("-Djava.library.path={}", natives_dir.display()));
        }

        for arg in Self::split_java_args(effective_settings.java_args.as_deref().unwrap_or("")) {
            cmd.arg(arg);
        }

        cmd.arg("-cp").arg(&classpath_str)
            .arg(&resolved.main_class)
            .arg("--username").arg(username)
            .arg("--uuid").arg(uuid)
            .arg("--accessToken").arg(access_token)
            .arg("--version").arg(version)
            .arg("--gameDir").arg(instance_dir)
            .arg("--assetsDir").arg(meta_dir.join("assets"))
            .arg("--assetIndex").arg(&resolved.assets_id);

        if resolved.is_neoforge || resolved.is_forge {
            for arg in &resolved.game_arguments {
                cmd.arg(substitute_arg(arg, subs));
            }
        }

        if let Some(server) = server_address {
            let use_quickplay = Self::should_use_quickplay(&resolved.base_version_id);
            if use_quickplay {
                cmd.arg("--quickPlayMultiplayer").arg(server);
            } else {
                cmd.arg("--server").arg(server);
            }
        }

        if let Some(world) = world_name {
            let use_quickplay = Self::should_use_quickplay(&resolved.base_version_id);
            if use_quickplay {
                cmd.arg("--quickPlaySingleplayer").arg(world);
            }
        }

        cmd.current_dir(instance_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                let err_msg = format!("Failed to spawn Minecraft process: {}. Check if Java path is correct: {}", e, java_path);
                Self::emit_error_log(app_handle, instance_name, &err_msg);
                return Err(err_msg.into());
            }
        };

        let child_pid = child.id();

        {
            let mut processes = crate::commands::instances::RUNNING_PROCESSES.lock().map_err(|e| e.to_string())?;
            processes.insert(instance_name.to_string(), child_pid);
        }

        let instance_name_for_status = instance_name.to_string();
        let launching_uuid = uuid.to_string();
        let app_handle_for_status = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            let _ = crate::services::friends::set_status_for_account(
                &app_handle_for_status,
                &launching_uuid,
                crate::models::FriendStatus::InGame,
                Some(instance_name_for_status),
            )
            .await;
        });

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let instance_name_clone = instance_name.to_string();
            let app_handle_clone = app_handle.clone();
            std::thread::spawn(move || {
                for line in reader.lines() {
                    if let Ok(line) = line {
                        if !line.contains("accessToken") && !line.contains("MINECRAFT_ACCESS_TOKEN") {
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

        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let instance_name_clone = instance_name.to_string();
            let app_handle_clone = app_handle.clone();
            std::thread::spawn(move || {
                let mut has_shown_friendly_error = false;
                for line in reader.lines() {
                    if let Ok(line) = line {
                        if line.contains("accessToken") || line.contains("MINECRAFT_ACCESS_TOKEN") {
                            continue;
                        }
                        if !has_shown_friendly_error {
                            let error_message = if line.contains("UnsupportedClassVersionError") {
                                Some("ERROR: Wrong Java version! This Minecraft version requires a newer Java version. Please update Java in Settings.")
                            } else if line.contains("class file version 65.0") {
                                Some("ERROR: Java version too old! You need Java 21 or newer. Your current Java is too old.")
                            } else if line.contains("class file version 61.0") {
                                Some("ERROR: Java version too old! You need Java 17 or newer. Your current Java is too old.")
                            } else if line.contains("Could not find or load main class") {
                                Some("ERROR: Game files are corrupted or missing. Try reinstalling this Minecraft version.")
                            } else if line.contains("java.lang.OutOfMemoryError") {
                                Some("ERROR: Not enough memory allocated! Increase RAM allocation in Settings.")
                            } else {
                                None
                            };
                            if let Some(msg) = error_message {
                                let _ = app_handle_clone.emit("console-log", serde_json::json!({
                                    "instance": instance_name_clone,
                                    "message": msg,
                                    "type": "stderr"
                                }));
                                has_shown_friendly_error = true;
                            }
                        }
                        let _ = app_handle_clone.emit("console-log", serde_json::json!({
                            "instance": instance_name_clone,
                            "message": line,
                            "type": "stderr"
                        }));
                    }
                }
            });
        }

        let instance_json = instance_dir.join("instance.json");
        let mut updated_instance = instance.clone();
        updated_instance.last_played = Some(Utc::now().to_rfc3339());
        let updated_json = serde_json::to_string_pretty(&updated_instance)?;
        fs::write(instance_json, updated_json)?;

        let instance_name_clone = instance_name.to_string();
        let app_handle_clone = app_handle.clone();
        let launching_uuid = uuid.to_string();
        let launch_time = std::time::Instant::now();

        std::thread::spawn(move || {
            Self::step_post_launch_process(
                child,
                &instance_name_clone,
                &launching_uuid,
                &app_handle_clone,
                launch_time,
            );
        });

        Ok(())
    }

    fn should_use_quickplay(version: &str) -> bool {
        let base_version = if version.contains("fabric-loader") {
            version.split('-').last().unwrap_or(version)
        } else if version.contains("-forge-") {
            version.split("-forge-").next().unwrap_or(version)
        } else if version.contains('-') {
            version.split('-').next().unwrap_or(version)
        } else {
            version
        };

        let parts: Vec<&str> = base_version.split('.').collect();

        if parts.len() >= 3 {
            if let (Ok(major), Ok(minor), Ok(patch)) =
                (parts[0].parse::<u32>(), parts[1].parse::<u32>(), parts[2].parse::<u32>())
            {
                if major == 1 && minor == 20 && patch >= 5 {
                    return true;
                }
                if major == 1 && minor > 20 {
                    return true;
                }
                if major > 1 {
                    return true;
                }
            }
        } else if parts.len() == 2 {
            if let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                if major == 1 && minor > 20 {
                    return true;
                }
                if major > 1 {
                    return true;
                }
            }
        }

        false
    }

    fn step_post_launch_process(
        mut child: Child,
        instance_name: &str,
        uuid: &str,
        app_handle: &tauri::AppHandle,
        launch_time: std::time::Instant,
    ) {
        let _ = child.wait();
        let play_duration = launch_time.elapsed().as_secs();

        let instance_dir = get_instance_dir(instance_name);
        let instance_json_path = instance_dir.join("instance.json");

        if let Ok(content) = fs::read_to_string(&instance_json_path) {
            if let Ok(mut instance) = serde_json::from_str::<Instance>(&content) {
                instance.total_playtime_seconds += play_duration;
                if let Ok(updated_json) = serde_json::to_string_pretty(&instance) {
                    let _ = fs::write(&instance_json_path, updated_json);
                }
            }
        }

        {
            if let Ok(mut processes) = crate::commands::instances::RUNNING_PROCESSES.lock() {
                processes.remove(instance_name);
            }
        }

        let uuid_owned = uuid.to_string();
        let app_handle_for_status = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            let _ = crate::services::friends::set_status_for_account(
                &app_handle_for_status,
                &uuid_owned,
                crate::models::FriendStatus::Online,
                None,
            )
            .await;
        });

        let _ = app_handle.emit("instance-exited", serde_json::json!({
            "instance": instance_name
        }));
    }
}
