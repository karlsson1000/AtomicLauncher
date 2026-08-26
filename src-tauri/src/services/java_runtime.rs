use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::Emitter;

const AZUL_PACKAGES_URL: &str = "https://api.azul.com/metadata/v1/zulu/packages";
const COMPLETION_MARKER: &str = ".octane-java-ok";

pub fn get_runtimes_dir() -> PathBuf {
    crate::utils::get_launcher_dir().join("runtimes")
}

fn runtime_dir(required_major: u32) -> PathBuf {
    get_runtimes_dir().join(format!("java-{}", required_major))
}

fn log(app_handle: &tauri::AppHandle, instance_name: &str, message: &str) {
    let _ = app_handle.emit(
        "console-log",
        json!({
            "instance": instance_name,
            "message": format!("[Java] {}", message),
            "type": "stdout"
        }),
    );
}

pub async fn ensure_java_for_launch(
    required_major: u32,
    instance_name: &str,
    app_handle: &tauri::AppHandle,
) -> Result<Option<String>, String> {
    let target_dir = runtime_dir(required_major);

    if let Some(java) = cached_java_executable(&target_dir) {
        return Ok(Some(java.to_string_lossy().to_string()));
    }

    log(
        app_handle,
        instance_name,
        &format!("Downloading Java {}...", required_major),
    );

    let client = crate::utils::http::get_client();
    let metadata_url = format!(
        "{}?arch={}&java_version={}&os={}&archive_type=zip&javafx_bundled=false&java_package_type=jre&page_size=1",
        AZUL_PACKAGES_URL,
        std::env::consts::ARCH,
        required_major,
        std::env::consts::OS
    );

    let packages: Vec<Value> = client
        .get(&metadata_url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let package = packages.first().ok_or_else(|| {
        format!(
            "No Java {} runtime is available for {} / {}",
            required_major,
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;

    let download_url = package
        .get("download_url")
        .and_then(Value::as_str)
        .ok_or("Java runtime metadata is missing a download URL")?;

    let staging_dir = target_dir.with_extension("tmp");
    if staging_dir.exists() {
        tokio::fs::remove_dir_all(&staging_dir)
            .await
            .map_err(|e| e.to_string())?;
    }
    tokio::fs::create_dir_all(&staging_dir)
        .await
        .map_err(|e| e.to_string())?;

    let archive_path = staging_dir.join("runtime.zip");
    let extraction_target = staging_dir.clone();
    crate::utils::download::download_file_verified(download_url, &archive_path, None).await?;

    tokio::task::spawn_blocking(move || {
        let result = extract_zip(&archive_path, &extraction_target);
        if result.is_ok() {
            let _ = std::fs::remove_file(&archive_path);
        }
        result
    })
    .await
    .map_err(|e| e.to_string())??;

    if target_dir.exists() {
        tokio::fs::remove_dir_all(&target_dir)
            .await
            .map_err(|e| e.to_string())?;
    }
    tokio::fs::rename(&staging_dir, &target_dir)
        .await
        .map_err(|e| e.to_string())?;
    tokio::fs::write(target_dir.join(COMPLETION_MARKER), "ok")
        .await
        .map_err(|e| e.to_string())?;

    let java = find_java_executable(&target_dir)
        .ok_or("Runtime downloaded, but no Java executable was found")?;

    log(
        app_handle,
        instance_name,
        &format!("Finished downloading Java {}.", required_major),
    );

    Ok(Some(java.to_string_lossy().to_string()))
}

fn cached_java_executable(runtime_dir: &Path) -> Option<PathBuf> {
    if !runtime_dir.join(COMPLETION_MARKER).exists() {
        return None;
    }
    find_java_executable(runtime_dir)
}

fn extract_zip(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;

        let Some(relative) = sanitized_relative_path(entry.name()) else {
            continue;
        };
        let out_path = destination.join(relative);

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out_file = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut out_file).map_err(|e| e.to_string())?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = entry.unix_mode() {
                    let _ =
                        std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(mode));
                }
            }
        }
    }

    Ok(())
}

fn sanitized_relative_path(relative: &str) -> Option<PathBuf> {
    if relative.starts_with('/') || relative.starts_with('\\') || relative.contains(':') {
        return None;
    }

    let mut path = PathBuf::new();
    for segment in relative.split(['/', '\\']) {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return None;
        }
        path.push(segment);
    }

    if path.as_os_str().is_empty() {
        None
    } else {
        Some(path)
    }
}

pub fn find_java_executable(root: &Path) -> Option<PathBuf> {
    const PREFERRED_NAMES: &[&str] = if cfg!(windows) {
        &["javaw.exe", "java.exe"]
    } else {
        &["java"]
    };

    let mut candidates: Vec<(usize, PathBuf)> = Vec::new();
    collect_java_binaries(root, 0, PREFERRED_NAMES, &mut candidates);

    for name in PREFERRED_NAMES {
        if let Some((_depth, path)) = candidates
            .iter()
            .filter(|(_, p)| {
                p.file_name()
                    .and_then(|f| f.to_str())
                    .map(|f| f.eq_ignore_ascii_case(name))
                    .unwrap_or(false)
            })
            .min_by_key(|(depth, _)| *depth)
        {
            return Some(path.clone());
        }
    }

    None
}

fn collect_java_binaries(dir: &Path, depth: usize, names: &[&str], out: &mut Vec<(usize, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_java_binaries(&path, depth + 1, names, out);
        } else if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
            if names.iter().any(|n| file_name.eq_ignore_ascii_case(n)) {
                out.push((depth, path));
            }
        }
    }
}
