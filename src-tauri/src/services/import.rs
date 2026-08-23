use crate::models::ImportableInstance;
use crate::services::fabric::FabricInstaller;
use crate::services::forge::ForgeInstaller;
use crate::services::instance::InstanceManager;
use crate::services::installer::MinecraftInstaller;
use crate::services::neoforge::NeoForgeInstaller;
use crate::utils::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Emitter;
use zip::ZipArchive;

#[derive(Clone)]
struct TheseusDbEntry {
    name: Option<String>,
    game_version: String,
    loader: Option<String>,
    loader_version: Option<String>,
    icon_path: Option<String>,
}

pub struct InstanceImporter;

impl InstanceImporter {
    pub fn detect() -> Vec<ImportableInstance> {
        let mut found = Vec::new();

        for (source, roots) in Self::source_roots() {
            let theseus_db = if source == "modrinth" {
                Self::load_theseus_db_entries()
            } else {
                HashMap::new()
            };

            for root in roots {
                if !root.is_dir() {
                    continue;
                }
                if let Ok(entries) = fs::read_dir(&root) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) else {
                            continue;
                        };
                        if dir_name.starts_with('.') || !path.is_dir() {
                            continue;
                        }
                        if let Some(instance) =
                            Self::inspect_instance(&source, &path, dir_name, &theseus_db)
                        {
                            found.push(instance);
                        }
                    }
                }
            }
        }

        let source_rank = |s: &str| match s {
            "modrinth" => 0,
            "prism" => 1,
            _ => 2,
        };
        found.sort_by(|a, b| {
            source_rank(&a.source)
                .cmp(&source_rank(&b.source))
                .then(a.name.cmp(&b.name))
        });
        found
    }

    fn source_roots() -> Vec<(&'static str, Vec<PathBuf>)> {
        let mut roots: Vec<(&'static str, Vec<PathBuf>)> = Vec::new();

        #[cfg(target_os = "windows")]
        {
            if let Some(data_dir) = dirs::data_dir() {
                roots.push((
                    "prism",
                    vec![data_dir.join("PrismLauncher").join("instances")],
                ));
                roots.push((
                    "modrinth",
                    vec![
                        data_dir.join("ModrinthApp").join("profiles"),
                        data_dir.join("com.modrinth.ModrinthApp").join("profiles"),
                    ],
                ));
                let mut curseforge_roots = Vec::new();
                if let Some(home) = dirs::home_dir() {
                    // Default install location of the CurseForge (Overwolf) app
                    curseforge_roots.push(home.join("curseforge").join("minecraft").join("Instances"));
                }
                curseforge_roots.push(data_dir.join("curseforge").join("Instances"));
                roots.push(("curseforge", curseforge_roots));
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(data_dir) = dirs::data_dir() {
                roots.push((
                    "prism",
                    vec![
                        data_dir.join("PrismLauncher").join("instances"),
                        dirs::home_dir()
                            .map(|h| h.join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances"))
                            .unwrap_or_default(),
                    ],
                ));
                roots.push((
                    "modrinth",
                    vec![
                        data_dir.join("ModrinthApp").join("profiles"),
                        data_dir.join("com.modrinth.ModrinthApp").join("profiles"),
                    ],
                ));
            }
        }

        roots
    }

    fn inspect_instance(
        source: &str,
        dir: &Path,
        name: &str,
        theseus_db: &HashMap<String, TheseusDbEntry>,
    ) -> Option<ImportableInstance> {
        match source {
            "prism" => {
                let has_meta = dir.join("instance.cfg").exists() || dir.join("mmc-pack.json").exists();
                if !has_meta {
                    return None;
                }
            }
            "curseforge" => {
                let has_content = dir.join("manifest.json").exists()
                    || dir.join("mods").is_dir()
                    || dir.join("saves").is_dir();
                if !has_content {
                    return None;
                }
            }
            "modrinth" => {
                let has_content = Self::read_theseus_metadata(dir).is_some()
                    || dir.join("mods").is_dir()
                    || dir.join("saves").is_dir()
                    || dir.join("options.txt").exists();
                if !has_content {
                    return None;
                }
            }
            _ => return None,
        }

        let content_root = Self::content_root(source, dir);
        let (mut display_name, mut mc_version, mut loader, mut loader_version, mut icon_path) =
            match source {
                "prism" => {
                    let cfg_name = Self::parse_prism_instance_cfg(&dir.join("instance.cfg"));
                    let (mc, l, lv) = Self::parse_mmc_pack(&dir.join("mmc-pack.json"));
                    let icon = ["instance.gif", "folder.jpg", "icon.png"]
                        .iter()
                        .map(|f| dir.join(f))
                        .find(|p| p.is_file())
                        .map(|p| p.to_string_lossy().to_string());
                    (cfg_name, mc, l, lv, icon)
                }
                "curseforge" => {
                    let (n, mc, l, lv) = Self::parse_curseforge_manifest(&dir.join("manifest.json"));
                    (n, mc, l, lv, None)
                }
                _ => {
                    let (n, mc, l, lv, icon) = Self::parse_theseus_metadata(dir);
                    (n, mc, l, lv, icon)
                }
            };

        if source == "modrinth" {
            if let Some(entry) = Self::lookup_theseus_db(dir, theseus_db) {
                if display_name.is_none() {
                    display_name = entry.name.clone();
                }
                if icon_path.is_none() {
                    icon_path = entry.icon_path.clone();
                }
                if mc_version.is_none() {
                    mc_version = Some(entry.game_version);
                }
                if loader.is_none() {
                    loader = entry.loader.filter(|l| !l.is_empty() && l != "vanilla");
                }
                loader_version = entry.loader_version;
            }
        }

        let trimmed = display_name.take().map(|n| n.trim().to_string());
        let display_name = trimmed.filter(|n| !n.is_empty());

        let icon_path = icon_path.filter(|p| Path::new(p).is_file());

        if loader.is_none() {
            loader = Self::sniff_loader_from_mods(&content_root);
        }
        if source != "prism" && mc_version.is_none() {
            mc_version = Self::sniff_mc_version(&content_root);
        }

        Some(ImportableInstance {
            source: source.to_string(),
            name: display_name.unwrap_or_else(|| name.to_string()),
            path: dir.to_string_lossy().to_string(),
            mc_version,
            loader,
            loader_version,
            size_bytes: Self::dir_size(dir),
            icon_path,
        })
    }

    fn content_root(source: &str, dir: &Path) -> PathBuf {
        let minecraft_subdir = dir.join(".minecraft");
        if (source == "prism" || source == "modrinth") && minecraft_subdir.exists() {
            return minecraft_subdir;
        }
        dir.to_path_buf()
    }

    fn parse_mmc_pack(path: &Path) -> (Option<String>, Option<String>, Option<String>) {
        let Ok(content) = fs::read_to_string(path) else {
            return (None, None, None);
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
            return (None, None, None);
        };

        let mut mc_version = None;
        let mut loader = None;
        let mut loader_version = None;

        if let Some(components) = value.get("components").and_then(|c| c.as_array()) {
            for component in components {
                let Some(uid) = component.get("uid").and_then(|u| u.as_str()) else {
                    continue;
                };
                let version = component
                    .get("version")
                    .or_else(|| component.get("cachedVersion"))
                    .and_then(|v| v.as_str())
                    .map(String::from);

                match uid {
                    "net.minecraft" => mc_version = version,
                    "net.fabricmc.fabric-loader" => {
                        loader = Some("fabric".to_string());
                        loader_version = version;
                    }
                    "net.minecraftforge" => {
                        loader = Some("forge".to_string());
                        loader_version = version;
                    }
                    u if u.starts_with("net.neoforged") => {
                        loader = Some("neoforge".to_string());
                        loader_version = version;
                    }
                    _ => {}
                }
            }
        }

        (mc_version, loader, loader_version)
    }

    fn parse_curseforge_manifest(
        path: &Path,
    ) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
        let Ok(content) = fs::read_to_string(path) else {
            return (None, None, None, None);
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
            return (None, None, None, None);
        };

        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .map(String::from);

        let Some(minecraft) = value.get("minecraft") else {
            return (name, None, None, None);
        };

        let mc_version = minecraft
            .get("version")
            .and_then(|v| v.as_str())
            .map(String::from);

        let mut loader = None;
        let mut loader_version = None;

        if let Some(mod_loaders) = minecraft.get("modLoaders").and_then(|m| m.as_array()) {
            for mod_loader in mod_loaders {
                let Some(id) = mod_loader.get("id").and_then(|i| i.as_str()) else {
                    continue;
                };
                if let Some((kind, version)) = id.split_once('-') {
                    loader = Some(kind.to_lowercase());
                    loader_version = Some(version.to_string());
                    break;
                }
            }
        }

        (name, mc_version, loader, loader_version)
    }

    fn parse_theseus_metadata(
        dir: &Path,
    ) -> (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) {
        match Self::read_theseus_metadata(dir) {
            Some(value) => {
                let metadata = value.get("metadata").unwrap_or(&value);
                let name = metadata
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let mc_version = metadata
                    .get("game_version")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let loader = metadata
                    .get("loader")
                    .and_then(|v| v.as_str())
                    .filter(|l| !l.is_empty() && *l != "vanilla" && *l != "unknown")
                    .map(String::from);
                let loader_version = metadata
                    .get("loader_version")
                    .and_then(|v| v.as_str())
                    .filter(|l| !l.is_empty())
                    .map(String::from);
                let raw_icon = metadata
                    .get("icon")
                    .and_then(|v| v.as_str())
                    .filter(|i| !i.is_empty());
                let icon = raw_icon.map(|i| {
                    let p = PathBuf::from(i);
                    if p.is_absolute() {
                        p
                    } else {
                        dir.join(p)
                    }
                });
                let icon = icon
                    .filter(|p| p.is_file())
                    .map(|p| p.to_string_lossy().to_string());
                (name, mc_version, loader, loader_version, icon)
            }
            None => (None, None, None, None, None),
        }
    }

    fn parse_prism_instance_cfg(path: &Path) -> Option<String> {
        let content = fs::read_to_string(path).ok()?;
        for line in content.lines() {
            if let Some(value) = line.strip_prefix("name=") {
                return Some(value.trim().to_string());
            }
        }
        None
    }

    fn read_theseus_metadata(dir: &Path) -> Option<serde_json::Value> {
        let entries = fs::read_dir(dir).ok()?;
        for entry in entries.flatten().take(10) {
            let path = entry.path();
            let is_json = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("json"))
                .unwrap_or(false);
            if !is_json {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                    let metadata = value.get("metadata").unwrap_or(&value);
                    if metadata.get("game_version").is_some() {
                        return Some(value);
                    }
                }
            }
        }
        None
    }

    fn load_theseus_db_entries() -> HashMap<String, TheseusDbEntry> {
        let mut map = HashMap::new();

        let Some(data_dir) = dirs::data_dir() else {
            return map;
        };

        let bases = [
            data_dir.join("ModrinthApp"),
            data_dir.join("com.modrinth.ModrinthApp"),
            data_dir.join("com.modrinth.theseus"),
        ];

        let mut temp_dirs: Vec<PathBuf> = Vec::new();

        for base in bases {
            for db_path in Self::find_sqlite_files(&base) {
                let Some(snapshot_path) = Self::snapshot_sqlite_db(&db_path, &mut temp_dirs)
                else {
                    continue;
                };

                let Ok(conn) = rusqlite::Connection::open_with_flags(
                    &snapshot_path,
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                ) else {
                    continue;
                };

                let mut load = |sql: &str| -> rusqlite::Result<()> {
                    let mut stmt = conn.prepare(sql)?;
                    let rows = stmt.query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<String>>(5)?,
                        ))
                    })?;
                    for row in rows.flatten() {
                        let (path, name, game_version, loader, loader_version, icon_path) = row;
                        map.insert(
                            path.replace('\\', "/"),
                            TheseusDbEntry {
                                name,
                                game_version,
                                loader,
                                loader_version,
                                icon_path,
                            },
                        );
                    }
                    Ok(())
                };

                let _ = load(
                    "SELECT i.path, i.name, cs.game_version, cs.loader, cs.loader_version, i.icon_path \
                     FROM instances i \
                     JOIN instance_content_sets cs ON cs.instance_id = i.id",
                );

                let _ = load(
                    "SELECT path, name, game_version, mod_loader, mod_loader_version, icon_path FROM profiles",
                );
            }
        }

        for dir in temp_dirs {
            let _ = fs::remove_dir_all(dir);
        }

        map
    }

    fn snapshot_sqlite_db(db_path: &Path, temp_dirs: &mut Vec<PathBuf>) -> Option<PathBuf> {
        let dir = std::env::temp_dir().join(format!(
            "octane_import_db_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        fs::create_dir_all(&dir).ok()?;

        let file_name = db_path.file_name()?.to_string_lossy().to_string();
        fs::copy(db_path, dir.join(&file_name)).ok()?;

        for sidecar_ext in ["-wal", "-shm"] {
            let mut sidecar = db_path.as_os_str().to_owned();
            sidecar.push(sidecar_ext);
            let sidecar_path = PathBuf::from(sidecar);
            if sidecar_path.exists() {
                let mut sidecar_copy = file_name.clone();
                sidecar_copy.push_str(sidecar_ext);
                let _ = fs::copy(&sidecar_path, dir.join(sidecar_copy));
            }
        }

        temp_dirs.push(dir.clone());
        Some(dir.join(file_name))
    }

    fn find_sqlite_files(base: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();

        for candidate in [base.to_path_buf(), base.join("settings"), base.join("data")] {
            let Ok(entries) = fs::read_dir(&candidate) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let is_db_file = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == "db" || e == "sqlite" || e == "sqlite3")
                    .unwrap_or(false);
                if is_db_file {
                    found.push(path);
                }
            }
        }

        found
    }

    fn lookup_theseus_db(
        dir: &Path,
        db: &HashMap<String, TheseusDbEntry>,
    ) -> Option<TheseusDbEntry> {
        if db.is_empty() {
            return None;
        }

        let folder = dir.file_name()?.to_str()?.to_string();
        let full_forward = dir.to_string_lossy().replace('\\', "/");
        let trimmed = full_forward.trim_end_matches('/').to_string();

        for key in [full_forward, trimmed, folder] {
            if let Some(entry) = db.get(&key) {
                return Some(entry.clone());
            }
        }

        None
    }

    fn sniff_loader_from_mods(content_root: &Path) -> Option<String> {
        let mods_dir = content_root.join("mods");
        let entries = fs::read_dir(&mods_dir).ok()?;

        let mut jars = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jar") {
                continue;
            }

            jars += 1;
            if jars > 25 {
                break;
            }

            if let Some(loader) = Self::sniff_jar_loader(&path) {
                return Some(loader);
            }
        }

        None
    }

    fn sniff_jar_loader(jar_path: &Path) -> Option<String> {
        let file = fs::File::open(jar_path).ok()?;
        let mut archive = ZipArchive::new(file).ok()?;

        let marker_for = |name: &str| -> Option<&'static str> {
            if name == "fabric.mod.json" {
                Some("fabric")
            } else if name == "META-INF/neoforge.mods.toml" {
                Some("neoforge")
            } else if name == "META-INF/mods.toml" || name == "mcmod.info" {
                Some("forge")
            } else {
                None
            }
        };

        for i in 0..archive.len() {
            let Ok(file) = archive.by_index(i) else { continue };
            if let Some(loader) = marker_for(file.name()) {
                return Some(loader.to_string());
            }
        }

        None
    }

    fn sniff_mc_version(content_root: &Path) -> Option<String> {
        let versions_dir = content_root.join("versions");
        if let Ok(entries) = fs::read_dir(&versions_dir) {
            for entry in entries.flatten().take(5) {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let version_json = path.join("version.json");
                if let Ok(content) = fs::read_to_string(&version_json) {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(id) = value.get("id").and_then(|v| v.as_str()) {
                            return Some(id.to_string());
                        }
                    }
                }
            }
        }

        Self::sniff_mc_version_from_log(content_root)
    }

    fn sniff_mc_version_from_log(content_root: &Path) -> Option<String> {
        let log_path = content_root.join("logs").join("latest.log");
        let Ok(content) = fs::read_to_string(&log_path) else {
            return None;
        };

        for line in content.lines().take(500) {
            if let Some(idx) = line.find("Loading Minecraft ") {
                let rest = &line[idx + "Loading Minecraft ".len()..];
                let version: String =
                    rest.chars().take_while(|c| !c.is_whitespace()).collect();
                if !version.is_empty() && version.chars().next().is_some_and(|c| c.is_ascii_digit())
                {
                    return Some(version);
                }
            }
        }

        None
    }

    fn dir_size(path: &Path) -> u64 {
        let mut size = 0;

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    size += Self::dir_size(&entry_path);
                } else if let Ok(metadata) = entry.metadata() {
                    size += metadata.len();
                }
            }
        }

        size
    }

    pub async fn import(
        detected: &ImportableInstance,
        target_name: &str,
        app_handle: &tauri::AppHandle,
    ) -> Result<(), String> {
        let mc_version = detected
            .mc_version
            .clone()
            .ok_or_else(|| format!("Could not determine the Minecraft version of '{}'", detected.name))?;

        let source_dir = PathBuf::from(&detected.path);
        if !source_dir.is_dir() {
            return Err(format!("Source folder no longer exists: {}", detected.path));
        }

        let meta_dir = get_meta_dir();

        let emit = |progress: u32, stage: &str| {
            let _ = app_handle.emit(
                "creation-progress",
                serde_json::json!({
                    "instance": target_name,
                    "progress": progress,
                    "stage": stage
                }),
            );
        };

        emit(0, &format!("Importing '{}'...", detected.name));
        emit(5, &format!("Installing Minecraft {}...", mc_version));

        let installer = MinecraftInstaller::new(meta_dir.clone()).map_err(|e| e.to_string())?;
        installer
            .install_version(&mc_version)
            .await
            .map_err(|e| e.to_string())?;

        let loader = detected.loader.clone().unwrap_or_else(|| "vanilla".to_string());
        let final_version = match loader.as_str() {
            "fabric" => {
                emit(20, "Installing Fabric loader...");
                let fabric_installer =
                    FabricInstaller::new(meta_dir.clone()).map_err(|e| e.to_string())?;
                let fabric_version = match &detected.loader_version {
                    Some(v) => v.clone(),
                    None => {
                        let versions = fabric_installer
                            .get_loader_versions()
                            .await
                            .map_err(|e| e.to_string())?;
                        versions
                            .iter()
                            .find(|v| v.stable)
                            .or_else(|| versions.first())
                            .ok_or("No Fabric versions found")?
                            .version
                            .clone()
                    }
                };
                fabric_installer
                    .install_fabric(&mc_version, &fabric_version)
                    .await
                    .map_err(|e| e.to_string())?
            }
            "forge" => {
                emit(20, "Installing Forge loader...");
                let forge_version = detected
                    .loader_version
                    .clone()
                    .ok_or("Forge version unknown for this instance")?;
                let full_version = format!("{}-{}", mc_version, forge_version);
                let forge_installer =
                    ForgeInstaller::new(meta_dir.clone()).map_err(|e| e.to_string())?;
                forge_installer
                    .install_forge(&full_version)
                    .await
                    .map_err(|e| e.to_string())?
            }
            "neoforge" => {
                emit(20, "Installing NeoForge loader...");
                let neoforge_version = detected
                    .loader_version
                    .clone()
                    .ok_or("NeoForge version unknown for this instance")?;
                let neoforge_installer =
                    NeoForgeInstaller::new(meta_dir.clone()).map_err(|e| e.to_string())?;
                neoforge_installer
                    .install_neoforge(&neoforge_version)
                    .await
                    .map_err(|e| e.to_string())?
            }
            "vanilla" => {
                emit(20, "Preparing vanilla installation...");
                mc_version.clone()
            }
            other => {
                return Err(format!(
                    "Loader '{}' is not supported for import yet",
                    other
                ));
            }
        };

        emit(55, "Creating instance...");

        InstanceManager::create(
            target_name,
            &final_version,
            if loader == "vanilla" { None } else { Some(loader.clone()) },
            detected.loader_version.clone(),
        )
        .map_err(|e| e.to_string())?;

        emit(60, "Copying your files...");

        let content_root = Self::content_root(&detected.source, &source_dir);
        let instance_dir = get_instance_dir(target_name);

        const SKIPPED_ENTRIES: &[&str] = &[
            "instance.json",
            "instance.cfg",
            "mmc-pack.json",
            "profile.json",
            "modrinth.app.json",
            "manifest.json",
            ".ftbapp",
            ".mixin.out",
            ".cache",
            ".backup",
            "natives",
            "versions",
            "logs",
            "crash-reports",
        ];

        let total_bytes = Self::dir_size(&content_root);
        let copied_bytes = std::sync::atomic::AtomicU64::new(0);
        let last_pct = std::sync::atomic::AtomicU32::new(60);

        Self::copy_dir_filtered(
            &content_root,
            &instance_dir,
            SKIPPED_ENTRIES,
            total_bytes,
            &copied_bytes,
            &last_pct,
            app_handle,
            target_name,
        )
        .map_err(|e| e.to_string())?;

        emit(97, "Setting icon...");

        if let Some(icon_source) = &detected.icon_path {
            let _ = Self::apply_imported_icon(target_name, icon_source).await;
        }

        emit(100, "Import complete!");

        Ok(())
    }

    async fn apply_imported_icon(target_name: &str, icon_source: &str) -> Result<(), String> {
        const MAX_ICON_BYTES: usize = 8 * 1024 * 1024;

        let metadata = fs::metadata(icon_source).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() as usize > MAX_ICON_BYTES {
            return Err("Icon file missing or too large".to_string());
        }

        use base64::{Engine as _, engine::general_purpose};
        let bytes = fs::read(icon_source).map_err(|e| e.to_string())?;
        let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;

        let mut png_buffer = std::io::Cursor::new(Vec::new());
        img.write_to(&mut png_buffer, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;

        crate::commands::set_instance_icon(
            target_name.to_string(),
            general_purpose::STANDARD.encode(png_buffer.into_inner()),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    fn copy_dir_filtered(
        src: &Path,
        dst: &Path,
        skipped: &[&str],
        total_bytes: u64,
        copied_bytes: &std::sync::atomic::AtomicU64,
        last_pct: &std::sync::atomic::AtomicU32,
        app_handle: &tauri::AppHandle,
        target_name: &str,
    ) -> std::io::Result<()> {
        use std::sync::atomic::Ordering;

        fs::create_dir_all(dst)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let entry_name = entry.file_name();

            if skipped
                .iter()
                .any(|s| s.eq_ignore_ascii_case(&entry_name.to_string_lossy()))
            {
                continue;
            }

            let src_path = entry.path();
            let dst_path = dst.join(entry_name);

            if file_type.is_dir() {
                Self::copy_dir_filtered(
                    &src_path,
                    &dst_path,
                    skipped,
                    total_bytes,
                    copied_bytes,
                    last_pct,
                    app_handle,
                    target_name,
                )?;
            } else if file_type.is_file() {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                fs::copy(&src_path, &dst_path)?;
                let copied = copied_bytes.fetch_add(size, Ordering::Relaxed) + size;

                if total_bytes > 0 {
                    let pct = 60 + (copied.min(total_bytes) * 15 / total_bytes) as u32;
                    if last_pct.swap(pct, Ordering::Relaxed) != pct {
                        let _ = app_handle.emit(
                            "creation-progress",
                            serde_json::json!({
                                "instance": target_name,
                                "progress": pct,
                                "stage": format!("Copying files... {:.1} MB", copied as f64 / 1024.0 / 1024.0)
                            }),
                        );
                    }
                }
            }
        }

        Ok(())
    }
}