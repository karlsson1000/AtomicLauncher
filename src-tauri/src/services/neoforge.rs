use crate::models::NeoForgeVersion;
use std::path::PathBuf;
use serde::Deserialize;
use std::process::{Command, Stdio};


const NEOFORGE_META_URL: &str = "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";
const NEOFORGE_MAVEN_URL: &str = "https://maven.neoforged.net/releases";

type NeoForgeError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Deserialize)]
struct NeoForgeMavenResponse {
    versions: Vec<String>,
}

pub struct NeoForgeInstaller {
    http_client: reqwest::Client,
    meta_dir: PathBuf,
}

impl NeoForgeInstaller {
    pub fn new(meta_dir: PathBuf) -> Result<Self, NeoForgeError> {
        Ok(Self {
            http_client: crate::utils::http::get_client(),
            meta_dir,
        })
    }

    pub fn parse_minecraft_version_from_neoforge(neoforge_version: &str) -> Option<String> {
        let version_clean = neoforge_version
            .replace("-beta", "")
            .replace("-alpha", "");
        
        let parts: Vec<&str> = version_clean.split('.').collect();
        
        if parts.len() >= 2 {
            if let Ok(major) = parts[0].parse::<u32>() {
                if let Ok(minor) = parts[1].parse::<u32>() {
                    if major >= 22 {
                        if parts.len() >= 3 {
                            if let Ok(patch) = parts[2].parse::<u32>() {
                                if patch == 0 {
                                    return Some(format!("{}.{}", major, minor));
                                } else {
                                    return Some(format!("{}.{}.{}", major, minor, patch));
                                }
                            }
                        }
                        if minor == 0 {
                            return Some(format!("{}", major));
                        } else {
                            return Some(format!("{}.{}", major, minor));
                        }
                    } else if major >= 20 {
                        if minor == 0 {
                            return Some(format!("1.{}", major));
                        } else {
                            return Some(format!("1.{}.{}", major, minor));
                        }
                    }
                }
            }
        }
        
        None
    }

    pub async fn get_neoforge_versions(&self) -> Result<Vec<NeoForgeVersion>, NeoForgeError> {
        let response = self.http_client
            .get(NEOFORGE_META_URL)
            .send()
            .await?;

        let text = response.text().await?;

        let maven_response: NeoForgeMavenResponse = serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse NeoForge versions: {}", e))?;
        
        let mut neoforge_versions = Vec::new();
        
        for version in maven_response.versions {
            if version.contains("snapshot") || version.contains("alpha") {
                continue;
            }
            
            if let Some((mc_version, neoforge_version)) = version.split_once('-') {
                if mc_version.starts_with("1.") && mc_version.contains('.') {
                    neoforge_versions.push(NeoForgeVersion {
                        minecraft_version: mc_version.to_string(),
                        neoforge_version: neoforge_version.to_string(),
                        full_version: version.clone(),
                    });
                    continue;
                }
            }
            
            if let Some(mc_version) = Self::parse_minecraft_version_from_neoforge(&version) {
                neoforge_versions.push(NeoForgeVersion {
                    minecraft_version: mc_version,
                    neoforge_version: version.clone(),
                    full_version: version.clone(),
                });
            }
        }

        neoforge_versions.reverse();
        Ok(neoforge_versions)
    }

    pub async fn get_supported_game_versions(&self) -> Result<Vec<String>, NeoForgeError> {
        let versions = self.get_neoforge_versions().await?;
        let mut mc_versions: Vec<String> = versions
            .into_iter()
            .map(|v| v.minecraft_version)
            .collect();
        
        mc_versions.sort();
        mc_versions.dedup();
        mc_versions.reverse();
        
        Ok(mc_versions)
    }

    pub async fn get_compatible_loader_for_minecraft(
        &self,
        minecraft_version: &str,
    ) -> Result<String, NeoForgeError> {
        let versions = self.get_neoforge_versions().await?;
        
        let compatible = versions
            .iter()
            .find(|v| v.minecraft_version == minecraft_version)
            .ok_or_else(|| format!("No NeoForge version found for Minecraft {}", minecraft_version))?;

        Ok(compatible.neoforge_version.clone())
    }

    fn ensure_launcher_profile(&self) -> Result<(), NeoForgeError> {
        let launcher_profiles_path = self.meta_dir.join("launcher_profiles.json");
        
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
            
            std::fs::write(
                &launcher_profiles_path,
                serde_json::to_string_pretty(&minimal_profile)?
            )?;
        }
        
        Ok(())
    }

    fn cleanup_install_logs(&self, full_version: &str) {
        // Clean up common installer log files
        let log_patterns: Vec<String> = vec![
            "installer.log".to_string(),
            "install.log".to_string(),
            "neoforge_installer.log".to_string(),
            format!("neoforge-{}-installer.jar.log", full_version),
        ];

        // Directories to check for log files
        let search_dirs = vec![
            self.meta_dir.clone(),
            std::env::temp_dir(),
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        ];

        // Check all patterns in all directories
        for dir in &search_dirs {
            for pattern in &log_patterns {
                let log_path = dir.join(pattern);
                if log_path.exists() {
                    let _ = std::fs::remove_file(&log_path);
                }
            }
        }

        // Clean up any neoforge-specific log files in all directories
        for dir in &search_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        if file_name.contains("neoforge") && file_name.ends_with(".log") {
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }

    pub async fn install_neoforge(
        &self,
        neoforge_version: &str,
    ) -> Result<String, NeoForgeError> {
        self.ensure_launcher_profile()?;
        
        let full_version = neoforge_version.to_string();
        
        let version_id = format!("neoforge-{}", full_version);
        
        // Check if already installed
        let version_dir = self.meta_dir.join("versions").join(&version_id);
        let json_path = version_dir.join(format!("{}.json", version_id));
        
        if json_path.exists() {
            return Ok(version_id);
        }
        
        let installer_url = format!(
            "{}/net/neoforged/neoforge/{}/neoforge-{}-installer.jar",
            NEOFORGE_MAVEN_URL, full_version, full_version
        );

        let installer_response = self.http_client.get(&installer_url).send().await?;
        
        if !installer_response.status().is_success() {
            return Err(format!("Failed to download NeoForge installer: HTTP {}", installer_response.status()).into());
        }
        
        let installer_bytes = installer_response.bytes().await?;

        let temp_dir = std::env::temp_dir();
        let installer_path = temp_dir.join(format!("neoforge-{}-installer.jar", full_version));
        std::fs::write(&installer_path, installer_bytes)?;

        let mut cmd = Command::new("java");
        cmd.arg("-jar")
            .arg(&installer_path)
            .arg("--installClient")
            .arg(&self.meta_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let child = cmd.spawn()?;
        
        let output = child.wait_with_output()?;

        let _ = std::fs::remove_file(&installer_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            // Clean up logs even on failure
            self.cleanup_install_logs(&full_version);
            
            return Err(format!(
                "NeoForge installer failed!\nStdout: {}\nStderr: {}",
                stdout, stderr
            ).into());
        }

        // Clean up installer logs after successful installation
        self.cleanup_install_logs(&full_version);

        if !json_path.exists() {
            return Err(format!(
                "NeoForge installer did not create the expected version JSON at: {:?}",
                json_path
            ).into());
        }

        Ok(version_id)
    }

    pub async fn get_loader_versions(&self) -> Result<Vec<NeoForgeVersion>, NeoForgeError> {
        self.get_neoforge_versions().await
    }
}