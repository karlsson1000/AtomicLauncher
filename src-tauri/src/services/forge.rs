use crate::models::ForgeVersion;
use crate::services::loader_common;
use std::path::PathBuf;
use serde::Deserialize;

const FORGE_API_URL: &str = "https://maven.minecraftforge.net/api/maven/versions/releases/net/minecraftforge/forge";
const FORGE_MAVEN_URL: &str = "https://maven.minecraftforge.net/releases";

type ForgeError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Deserialize)]
struct ForgeMavenResponse {
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

    pub async fn install_forge(
        &self,
        forge_version: &str,
    ) -> Result<String, ForgeError> {
        loader_common::ensure_launcher_profile(&self.meta_dir)
            .map_err(|e| -> ForgeError { e.into() })?;

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
        let installer_path = loader_common::unique_installer_jar("forge", &full_version);
        std::fs::write(&installer_path, &installer_bytes)?;

        let meta_dir = self.meta_dir.clone();
        let (success, stdout, stderr) =
            loader_common::run_installer_jvm(installer_path.clone(), meta_dir).await?;

        let _ = std::fs::remove_file(&installer_path);
        loader_common::cleanup_install_logs(&self.meta_dir, "forge", &full_version);

        if !success {
            return Err(format!(
                "Forge installer failed!\nStdout: {}\nStderr: {}",
                stdout, stderr
            ).into());
        }

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
