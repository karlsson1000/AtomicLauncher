use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const INSTALLER_TIMEOUT_SECS: u64 = 600;

pub fn ensure_launcher_profile(meta_dir: &Path) -> Result<(), String> {
    let launcher_profiles_path = meta_dir.join("launcher_profiles.json");

    if !launcher_profiles_path.exists() {
        let minimal_profile = serde_json::json!({
            "profiles": {},
            "settings": {
                "enableSnapshots": false,
                "enableAdvanced": false,
                "crashAssistance": true,
                "enableHistorical": false,
                "enableReleases": true,
                "keepLauncherOpen": false,
                "showGameLog": false,
                "showMenu": false,
                "soundOn": false
            },
            "version": 3
        });

        std::fs::create_dir_all(meta_dir).map_err(|e| e.to_string())?;
        std::fs::write(
            &launcher_profiles_path,
            serde_json::to_string_pretty(&minimal_profile).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn cleanup_install_logs(meta_dir: &Path, keyword: &str, full_version: &str) {
    let exact_names = [
        "installer.log".to_string(),
        "install.log".to_string(),
        format!("{keyword}_installer.log"),
        format!("{keyword}-{full_version}-installer.jar.log"),
    ];

    for name in &exact_names {
        let log_path = meta_dir.join(name);
        if log_path.exists() {
            let _ = std::fs::remove_file(&log_path);
        }
    }
}

pub fn unique_installer_jar(keyword: &str, full_version: &str) -> PathBuf {
    let unique = uuid::Uuid::new_v4().simple();
    std::env::temp_dir().join(format!(
        "{keyword}-{full_version}-{unique}-installer.jar"
    ))
}

fn kill_process_tree(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = Command::new("taskkill");
        cmd.arg("/F").arg("/T").arg("/PID").arg(pid.to_string());
        cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = cmd.output();
    }

    #[cfg(not(target_os = "windows"))]
    {
        unsafe {
            libc::kill(pid as i32, libc::SIGKILL);
        }
    }
}

struct InstallerOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

fn run_installer_blocking(java: &str, jar: &Path, meta_dir: &Path) -> Result<InstallerOutput, String> {
    let mut cmd = Command::new(java);
    cmd.arg("-jar")
        .arg(jar)
        .arg("--installClient")
        .arg(meta_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd.spawn().map_err(|e| {
        format!(
            "Failed to launch installer JVM ({}): {}",
            java, e
        )
    })?;
    let pid = child.id();

    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let stdout_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(pipe) = stdout_pipe.as_mut() {
            let _ = pipe.read_to_string(&mut buf);
        }
        buf
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(pipe) = stderr_pipe.as_mut() {
            let _ = pipe.read_to_string(&mut buf);
        }
        buf
    });

    let deadline = Instant::now() + Duration::from_secs(INSTALLER_TIMEOUT_SECS);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = stdout_reader.join().unwrap_or_default();
                let stderr = stderr_reader.join().unwrap_or_default();
                return Ok(InstallerOutput {
                    success: status.success(),
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    kill_process_tree(pid);
                    let _ = child.wait();
                    return Err(format!(
                        "Installer timed out after {} seconds and was terminated",
                        INSTALLER_TIMEOUT_SECS
                    ));
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(e) => return Err(format!("Failed to wait for installer: {}", e)),
        }
    }
}

pub async fn run_installer_jvm(
    jar: PathBuf,
    meta_dir: PathBuf,
) -> Result<(bool, String, String), String> {
    let java =
        crate::utils::find_java().ok_or_else(|| "No Java installation found".to_string())?;

    tauri::async_runtime::spawn_blocking(move || {
        run_installer_blocking(&java, &jar, &meta_dir).map(|out| (out.success, out.stdout, out.stderr))
    })
    .await
    .map_err(|e| format!("Installer task failed: {}", e))?
}
