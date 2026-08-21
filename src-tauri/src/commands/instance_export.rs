use crate::commands::validation::sanitize_instance_name;
use crate::models::Instance;
use crate::utils::*;
use std::io::Write;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

#[tauri::command]
pub async fn export_instance(
    instance_name: String,
    output_path: String,
    export_format: String,
    include_worlds: bool,
    include_resource_packs: bool,
    include_shader_packs: bool,
    include_mods: bool,
    include_config: bool,
) -> Result<(), String> {
    let safe_name = sanitize_instance_name(&instance_name)?;
    let instance_dir = get_instance_dir(&safe_name);

    if !instance_dir.exists() {
        return Err(format!("Instance '{}' does not exist", safe_name));
    }

    let output_path_obj = std::path::Path::new(&output_path);
    if let Some(parent) = output_path_obj.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create output directory: {}", e))?;
        }
    }

    let file = std::fs::File::create(&output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;

    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o755);

    if export_format == "mrpack" {
        export_as_mrpack(
            &mut zip,
            &safe_name,
            &instance_dir,
            options,
            include_worlds,
            include_resource_packs,
            include_shader_packs,
            include_mods,
            include_config,
        )
        .await?;
    } else {
        export_as_zip(
            &mut zip,
            &instance_dir,
            options,
            include_worlds,
            include_resource_packs,
            include_shader_packs,
            include_mods,
            include_config,
        )?;
    }

    zip.finish()
        .map_err(|e| format!("Failed to finalize zip: {}", e))?;

    Ok(())
}

fn export_as_zip(
    zip: &mut ZipWriter<std::fs::File>,
    instance_dir: &std::path::Path,
    options: SimpleFileOptions,
    include_worlds: bool,
    include_resource_packs: bool,
    include_shader_packs: bool,
    include_mods: bool,
    include_config: bool,
) -> Result<(), String> {
    let instance_json = instance_dir.join("instance.json");
    if instance_json.exists() {
        add_file_to_zip(zip, &instance_json, "instance.json", options)?;
    }

    let icon_path = instance_dir.join("icon.png");
    if icon_path.exists() {
        add_file_to_zip(zip, &icon_path, "icon.png", options)?;
    }

    if include_worlds {
        let saves_dir = instance_dir.join("saves");
        if saves_dir.exists() {
            add_dir_to_zip(zip, &saves_dir, "saves", options)?;
        }
    }

    if include_resource_packs {
        let resourcepacks_dir = instance_dir.join("resourcepacks");
        if resourcepacks_dir.exists() {
            add_dir_to_zip(zip, &resourcepacks_dir, "resourcepacks", options)?;
        }
    }

    if include_shader_packs {
        let shaderpacks_dir = instance_dir.join("shaderpacks");
        if shaderpacks_dir.exists() {
            add_dir_to_zip(zip, &shaderpacks_dir, "shaderpacks", options)?;
        }
    }

    if include_mods {
        let mods_dir = instance_dir.join("mods");
        if mods_dir.exists() {
            add_dir_to_zip(zip, &mods_dir, "mods", options)?;
        }
    }

    if include_config {
        let config_dir = instance_dir.join("config");
        if config_dir.exists() {
            add_dir_to_zip(zip, &config_dir, "config", options)?;
        }

        let options_txt = instance_dir.join("options.txt");
        if options_txt.exists() {
            add_file_to_zip(zip, &options_txt, "options.txt", options)?;
        }

        let optionsof_txt = instance_dir.join("optionsof.txt");
        if optionsof_txt.exists() {
            add_file_to_zip(zip, &optionsof_txt, "optionsof.txt", options)?;
        }

        let optionsshaders_txt = instance_dir.join("optionsshaders.txt");
        if optionsshaders_txt.exists() {
            add_file_to_zip(zip, &optionsshaders_txt, "optionsshaders.txt", options)?;
        }

        let servers_dat = instance_dir.join("servers.dat");
        if servers_dat.exists() {
            add_file_to_zip(zip, &servers_dat, "servers.dat", options)?;
        }

        let servers_dat_old = instance_dir.join("servers.dat_old");
        if servers_dat_old.exists() {
            add_file_to_zip(zip, &servers_dat_old, "servers.dat_old", options)?;
        }
    }

    Ok(())
}

async fn export_as_mrpack(
    zip: &mut ZipWriter<std::fs::File>,
    instance_name: &str,
    instance_dir: &std::path::Path,
    options: SimpleFileOptions,
    include_worlds: bool,
    include_resource_packs: bool,
    include_shader_packs: bool,
    include_mods: bool,
    include_config: bool,
) -> Result<(), String> {
    let instance_json_path = instance_dir.join("instance.json");
    let instance_content = std::fs::read_to_string(&instance_json_path)
        .map_err(|e| e.to_string())?;
    let instance: Instance = serde_json::from_str(&instance_content)
        .map_err(|e| e.to_string())?;

    let loader = instance.loader.clone().unwrap_or_else(|| "vanilla".to_string());
    let minecraft_version = extract_minecraft_version(&instance.version, &loader);
    let loader_version = instance.loader_version.clone();

    let mut manifest = serde_json::json!({
        "formatVersion": 1,
        "game": "minecraft",
        "versionId": format!("{}-{}", instance_name, chrono::Utc::now().timestamp()),
        "name": instance_name,
        "summary": format!("Exported from launcher - Minecraft {}", minecraft_version),
        "files": [],
        "dependencies": {
            "minecraft": minecraft_version
        }
    });

    match loader.as_str() {
        "fabric" => {
            if let Some(ver) = loader_version {
                manifest["dependencies"]["fabric-loader"] = serde_json::Value::String(ver);
            }
        }
        "forge" => {
            if let Some(ver) = loader_version {
                manifest["dependencies"]["forge"] = serde_json::Value::String(ver);
            }
        }
        "neoforge" => {
            if let Some(ver) = loader_version {
                manifest["dependencies"]["neoforge"] = serde_json::Value::String(ver);
            }
        }
        _ => {}
    }

    if include_mods {
        let mods_dir = instance_dir.join("mods");
        if mods_dir.exists() {
            let mut resolved_files: Vec<serde_json::Value> = Vec::new();
            let mut override_candidates: Vec<std::path::PathBuf> = Vec::new();

            let mut candidates: Vec<std::path::PathBuf> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&mods_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    let is_mod_archive = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| {
                            e.eq_ignore_ascii_case("jar")
                                || e.eq_ignore_ascii_case("zip")
                                || e.eq_ignore_ascii_case("litemod")
                        })
                        .unwrap_or(false);
                    if is_mod_archive {
                        candidates.push(path);
                    } else {
                        override_candidates.push(path);
                    }
                }
            }

            let client = crate::utils::modrinth::ModrinthClient::new().ok();
            let mut hashes: Vec<String> = Vec::new();
            let mut hashed: Vec<(std::path::PathBuf, String)> = Vec::new();

            for path in &candidates {
                let Ok(bytes) = std::fs::read(path) else {
                    override_candidates.push(path.clone());
                    continue;
                };

                use sha1::{Digest, Sha1};
                let mut sha1_hasher = Sha1::new();
                sha1_hasher.update(&bytes);
                let sha1_hex = format!("{:x}", sha1_hasher.finalize());

                hashes.push(sha1_hex.clone());
                hashed.push((path.clone(), sha1_hex));
            }

            let mut resolved_map = std::collections::HashMap::new();
            if !hashes.is_empty() {
                if let Some(client) = &client {
                    if let Ok(map) = client.get_version_files_by_hashes(&hashes).await {
                        resolved_map = map;
                    }
                }
            }

            for (path, sha1_hex) in hashed {
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                let version_file = resolved_map
                    .get(&sha1_hex)
                    .and_then(|response| response.files.iter().find(|f| f.hashes.sha1 == sha1_hex));

                if let Some(vf) = version_file {
                    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(vf.size);
                    resolved_files.push(serde_json::json!({
                        "path": format!("mods/{}", file_name),
                        "hashes": {
                            "sha1": vf.hashes.sha1,
                            "sha512": vf.hashes.sha512,
                        },
                        "env": {
                            "client": "required",
                            "server": "required"
                        },
                        "downloads": [vf.url],
                        "fileSize": size
                    }));
                } else {
                    override_candidates.push(path);
                }
            }

            manifest["files"] = serde_json::Value::Array(resolved_files);

            for path in override_candidates {
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .ok_or("Invalid mod file name")?;
                add_file_to_zip(zip, &path, &format!("overrides/mods/{}", file_name), options)?;
            }
        }
    }

    if include_worlds {
        let saves_dir = instance_dir.join("saves");
        if saves_dir.exists() {
            add_dir_to_zip(zip, &saves_dir, "overrides/saves", options)?;
        }
    }

    if include_resource_packs {
        let resourcepacks_dir = instance_dir.join("resourcepacks");
        if resourcepacks_dir.exists() {
            add_dir_to_zip(zip, &resourcepacks_dir, "overrides/resourcepacks", options)?;
        }
    }

    if include_shader_packs {
        let shaderpacks_dir = instance_dir.join("shaderpacks");
        if shaderpacks_dir.exists() {
            add_dir_to_zip(zip, &shaderpacks_dir, "overrides/shaderpacks", options)?;
        }
    }

    if include_config {
        let config_dir = instance_dir.join("config");
        if config_dir.exists() {
            add_dir_to_zip(zip, &config_dir, "overrides/config", options)?;
        }

        let options_txt = instance_dir.join("options.txt");
        if options_txt.exists() {
            add_file_to_zip(zip, &options_txt, "overrides/options.txt", options)?;
        }

        let optionsof_txt = instance_dir.join("optionsof.txt");
        if optionsof_txt.exists() {
            add_file_to_zip(zip, &optionsof_txt, "overrides/optionsof.txt", options)?;
        }

        let optionsshaders_txt = instance_dir.join("optionsshaders.txt");
        if optionsshaders_txt.exists() {
            add_file_to_zip(zip, &optionsshaders_txt, "overrides/optionsshaders.txt", options)?;
        }
    }

    let icon_path = instance_dir.join("icon.png");
    if icon_path.exists() {
        add_file_to_zip(zip, &icon_path, "icon.png", options)?;
    }

    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

    zip.start_file("modrinth.index.json", options)
        .map_err(|e| format!("Failed to create manifest file: {}", e))?;
    zip.write_all(manifest_json.as_bytes())
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    Ok(())
}

fn extract_minecraft_version(version_string: &str, loader: &str) -> String {
    match loader {
        "fabric" => {
            if let Some(mc_version) = version_string.rsplit('-').next() {
                return mc_version.to_string();
            }
            version_string.to_string()
        }
        "forge" => {
            if let Some(pos) = version_string.find("-forge-") {
                return version_string[..pos].to_string();
            }
            version_string.to_string()
        }
        "neoforge" => {
            if let Some(ver) = version_string.strip_prefix("neoforge-") {
                if let Some((mc_ver, _)) = ver.split_once('-') {
                    if mc_ver.starts_with("1.") {
                        return mc_ver.to_string();
                    }
                }
                if let Some(mc_ver) =
                    crate::services::neoforge::NeoForgeInstaller::parse_minecraft_version_from_neoforge(ver)
                {
                    return mc_ver;
                }
            }
            version_string.to_string()
        }
        _ => version_string.to_string(),
    }
}

fn add_file_to_zip(
    zip: &mut ZipWriter<std::fs::File>,
    file_path: &std::path::Path,
    zip_path: &str,
    options: SimpleFileOptions,
) -> Result<(), String> {
    let mut file = std::fs::File::open(file_path)
        .map_err(|e| format!("Failed to open file: {}", e))?;

    zip.start_file(zip_path, options)
        .map_err(|e| format!("Failed to start file in zip: {}", e))?;

    std::io::copy(&mut file, zip)
        .map_err(|e| format!("Failed to write file to zip: {}", e))?;

    Ok(())
}

fn add_dir_to_zip(
    zip: &mut ZipWriter<std::fs::File>,
    dir_path: &std::path::Path,
    zip_prefix: &str,
    options: SimpleFileOptions,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir_path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        let zip_path = format!("{}/{}", zip_prefix, name_str);

        if path.is_file() {
            add_file_to_zip(zip, &path, &zip_path, options)?;
        } else if path.is_dir() {
            zip.add_directory(&format!("{}/", zip_path), options)
                .map_err(|e| format!("Failed to add directory to zip: {}", e))?;

            add_dir_to_zip(zip, &path, &zip_path, options)?;
        }
    }

    Ok(())
}

