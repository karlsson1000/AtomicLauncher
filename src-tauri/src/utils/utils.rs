use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

pub const LIBRARY_BASE_URL: &str = "https://libraries.minecraft.net/";

pub fn library_maven_path(libraries_dir: &Path, name: &str) -> PathBuf {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return libraries_dir.join("unknown.jar");
    }
    let (group, artifact, version) = (parts[0], parts[1], parts[2]);
    let classifier = if parts.len() >= 4 { Some(parts[3]) } else { None };
    let group_path = group.replace('.', "/");
    let jar_name = if let Some(cls) = classifier {
        format!("{}-{}-{}.jar", artifact, version, cls)
    } else {
        format!("{}-{}.jar", artifact, version)
    };
    libraries_dir.join(&group_path).join(artifact).join(version).join(&jar_name)
}

pub fn library_maven_url(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return String::new();
    }
    let (group, artifact, version) = (parts[0], parts[1], parts[2]);
    let classifier = if parts.len() >= 4 { Some(parts[3]) } else { None };
    let group_path = group.replace('.', "/");
    let jar_name = if let Some(cls) = classifier {
        format!("{}-{}-{}.jar", artifact, version, cls)
    } else {
        format!("{}-{}.jar", artifact, version)
    };
    format!("{}{}/{}/{}/{}", LIBRARY_BASE_URL, group_path, artifact, version, jar_name)
}

pub fn get_current_os() -> String {
    #[cfg(target_os = "windows")]
    return "windows".to_string();

    #[cfg(target_os = "linux")]
    return "linux".to_string();
}

pub fn get_launcher_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");

    #[cfg(target_os = "windows")]
    let launcher_dir = home.join("AppData").join("Roaming").join("Octane Launcher");

    #[cfg(target_os = "linux")]
    let launcher_dir = home.join(".octane-launcher");

    launcher_dir
}

pub fn get_meta_dir() -> PathBuf {
    get_launcher_dir().join("meta")
}

pub fn get_instances_dir() -> PathBuf {
    get_launcher_dir().join("instances")
}

pub fn get_instance_dir(instance_name: &str) -> PathBuf {
    get_instances_dir().join(instance_name)
}

pub fn get_trash_dir() -> PathBuf {
    get_launcher_dir().join("trash")
}

pub fn get_trash_index_path() -> PathBuf {
    get_trash_dir().join("trash_index.json")
}

pub fn get_default_options_path() -> PathBuf {
    get_launcher_dir().join("default-options.txt")
}

pub fn find_java() -> Option<String> {
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        #[cfg(target_os = "windows")]
        {
            let javaw_path = PathBuf::from(&java_home).join("bin").join("javaw.exe");
            if javaw_path.exists() {
                return Some(javaw_path.to_string_lossy().to_string());
            }
        }
        let java_bin = if cfg!(windows) { "java.exe" } else { "java" };
        let java_path = PathBuf::from(&java_home).join("bin").join(java_bin);
        if java_path.exists() {
            return Some(java_path.to_string_lossy().to_string());
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(output) = Command::new("which").arg("java").output() {
            if output.status.success() {
                if let Ok(path) = String::from_utf8(output.stdout) {
                    let trimmed = path.trim().to_string();
                    if !trimmed.is_empty() {
                        return Some(trimmed);
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = Command::new("where");
        cmd.arg("java");
        cmd.creation_flags(0x08000000);
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                if let Ok(paths) = String::from_utf8(output.stdout) {
                    if let Some(first) = paths.lines().next() {
                        let trimmed = first.trim().to_string();
                        if !trimmed.is_empty() {
                            let candidate = PathBuf::from(&trimmed);
                            if candidate.file_name().and_then(|f| f.to_str()) == Some("java.exe") {
                                if let Some(parent) = candidate.parent() {
                                    let javaw = parent.join("javaw.exe");
                                    if javaw.exists() {
                                        return Some(javaw.to_string_lossy().to_string());
                                    }
                                }
                            }
                            return Some(trimmed);
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let search_roots = [
            r"C:\Program Files\Java",
            r"C:\Program Files (x86)\Java",
            r"C:\Program Files\Eclipse Adoptium",
            r"C:\Program Files (x86)\Eclipse Adoptium",
            r"C:\Program Files\Microsoft",
            r"C:\Program Files\Zulu",
            r"C:\Program Files\GraalVM",
            r"C:\Program Files\BellSoft",
            r"C:\Program Files\Amazon Corretto",
        ];
        if let Some(path) = scan_jvm_dirs(&search_roots, "bin\\javaw.exe") {
            return Some(path);
        }
        if let Some(path) = scan_jvm_dirs(&search_roots, "bin\\java.exe") {
            return Some(path);
        }
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let local_programs = PathBuf::from(local_app_data).join("Programs");
            if local_programs.exists() {
                if let Ok(entries) = fs::read_dir(&local_programs) {
                    for entry in entries.flatten() {
                        let javaw_path = entry.path().join("bin").join("javaw.exe");
                        if javaw_path.exists() {
                            return Some(javaw_path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let search_roots = [
            "/usr/lib/jvm",
            "/usr/local/lib/jvm",
        ];
        if let Some(path) = scan_jvm_dirs(&search_roots, "bin/java") {
            return Some(path);
        }
        let snap_path = PathBuf::from("/snap/bin/java");
        if snap_path.exists() {
            return Some(snap_path.to_string_lossy().to_string());
        }
    }

    None
}

fn parse_java_major_from_dirname(dirname: &str) -> u32 {
    dirname.split(|c: char| !c.is_ascii_digit() && c != '.')
        .filter(|s| !s.is_empty())
        .filter_map(|s| {
            let parts: Vec<&str> = s.split('.').collect();
            if parts.len() >= 2 && parts[0] == "1" {
                parts[1].parse::<u32>().ok()
            } else {
                parts[0].parse::<u32>().ok()
            }
        })
        .max()
        .unwrap_or(0)
}

fn scan_jvm_dirs(roots: &[&str], binary_relative: &str) -> Option<String> {
    let mut candidates: Vec<(PathBuf, u32)> = Vec::new();

    for root in roots {
        let root_path = PathBuf::from(root);
        if !root_path.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&root_path) {
            for entry in entries.flatten() {
                let java_bin = entry.path().join(binary_relative);
                if java_bin.exists() {
                    let version = entry.path()
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(parse_java_major_from_dirname)
                        .unwrap_or(0);
                    candidates.push((java_bin, version));
                }
            }
        }
    }

    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    candidates.into_iter().next().map(|(p, _)| p.to_string_lossy().to_string())
}

pub fn open_folder(path: PathBuf) -> Result<(), std::io::Error> {
    #[cfg(target_os = "windows")]
    Command::new("explorer").arg(path).spawn()?;

    #[cfg(target_os = "linux")]
    Command::new("xdg-open").arg(path).spawn()?;

    Ok(())
}