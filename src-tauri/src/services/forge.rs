use crate::models::ForgeVersion;
use std::path::PathBuf;
use serde::Deserialize;
use std::process::{Command, Stdio};

const FORGE_API_URL: &str = "https://maven.minecraftforge.net/api/maven/versions/releases/net/minecraftforge/forge";
const FORGE_MAVEN_URL: &str = "https://maven.minecraftforge.net/releases";

type ForgeError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ForgeMavenResponse {
    #[serde(default, rename = "isSnapshot")]
    is_snapshot: bool,
    versions: Vec<String>,
}

pub struct ForgeInstaller {
    http_client: reqwest::Client,
    meta_dir: PathBuf,
}

impl ForgeInstaller {
    pub fn new(meta_dir: PathBuf) -> Result<Self, ForgeError> {
        Ok(Self {
            http_client: crate::utils::http::get_client(),
            meta_dir,
        })
    }

    pub async fn get_forge_versions(&self) -> Result<Vec<ForgeVersion>, ForgeError> {
        let response = self.http_client
            .get(FORGE_API_URL)
            .send()
            .await?;

        let text = response.text().await?;

        let maven_response: ForgeMavenResponse = serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse Forge versions: {}", e))?;

        let mut forge_versions = Vec::new();

        for version in maven_response.versions {
            if version.contains("snapshot") {
                continue;
            }

            if let Some((mc_version, forge_version)) = version.split_once('-') {
                if !mc_version.is_empty() && !forge_version.is_empty() {
                    forge_versions.push(ForgeVersion {
                        minecraft_version: mc_version.to_string(),
                        forge_version: forge_version.to_string(),
                        full_version: version.clone(),
                    });
                }
            }
        }

        forge_versions.reverse();
        Ok(forge_versions)
    }

    pub async fn get_supported_game_versions(&self) -> Result<Vec<String>, ForgeError> {
        let versions = self.get_forge_versions().await?;
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
    ) -> Result<String, ForgeError> {
        let versions = self.get_forge_versions().await?;

        let compatible = versions
            .iter()
            .find(|v| v.minecraft_version == minecraft_version)
            .ok_or_else(|| format!("No Forge version found for Minecraft {}", minecraft_version))?;

        Ok(compatible.forge_version.clone())
    }

    fn ensure_launcher_profile(&self) -> Result<(), ForgeError> {
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
        let log_patterns: Vec<String> = vec![
            "installer.log".to_string(),
            "install.log".to_string(),
            "forge_installer.log".to_string(),
            format!("forge-{}-installer.jar.log", full_version),
        ];

        let search_dirs = vec![
            self.meta_dir.clone(),
            std::env::temp_dir(),
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        ];

        for dir in &search_dirs {
            for pattern in &log_patterns {
                let log_path = dir.join(pattern);
                if log_path.exists() {
                    let _ = std::fs::remove_file(&log_path);
                }
            }
        }

        for dir in &search_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        if file_name.contains("forge") && file_name.ends_with(".log") {
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }

    pub async fn install_forge(
        &self,
        forge_version: &str,
    ) -> Result<String, ForgeError> {
        self.ensure_launcher_profile()?;

        let full_version = forge_version.to_string();

        let (mc_ver, forge_ver) = full_version.split_once('-')
            .ok_or_else(|| format!("Invalid Forge version format: {}", full_version))?;
        let version_id = format!("{}-forge-{}", mc_ver, forge_ver);

        let version_dir = self.meta_dir.join("versions").join(&version_id);
        let json_path = version_dir.join(format!("{}.json", version_id));

        if json_path.exists() {
            return Ok(version_id);
        }

        let installer_url = format!(
            "{}/net/minecraftforge/forge/{}/forge-{}-installer.jar",
            FORGE_MAVEN_URL, full_version, full_version
        );

        let installer_response = self.http_client.get(&installer_url).send().await?;

        if !installer_response.status().is_success() {
            return Err(format!("Failed to download Forge installer: HTTP {}", installer_response.status()).into());
        }

        let installer_bytes = installer_response.bytes().await?;

        let temp_dir = std::env::temp_dir();
        let installer_path = temp_dir.join(format!("forge-{}-installer.jar", full_version));
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

            self.cleanup_install_logs(&full_version);

            return Err(format!(
                "Forge installer failed!\nStdout: {}\nStderr: {}",
                stdout, stderr
            ).into());
        }

        self.cleanup_install_logs(&full_version);

        if !json_path.exists() {
            return Err(format!(
                "Forge installer did not create the expected version JSON at: {:?}",
                json_path
            ).into());
        }

        Ok(version_id)
    }

    pub async fn get_loader_versions(&self) -> Result<Vec<ForgeVersion>, ForgeError> {
        self.get_forge_versions().await
    }
}
