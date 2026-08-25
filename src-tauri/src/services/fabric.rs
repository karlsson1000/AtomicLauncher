use crate::models::*;
use std::{fs, path::PathBuf};

const FABRIC_META_URL: &str = "https://meta.fabricmc.net/v2";

pub struct FabricInstaller {
    http_client: reqwest::Client,
    launcher_dir: PathBuf,
}

impl FabricInstaller {
    pub fn new(launcher_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            http_client: crate::utils::http::get_client(),
            launcher_dir,
        })
    }

    pub async fn get_loader_versions(&self) -> Result<Vec<FabricLoaderVersion>, Box<dyn std::error::Error>> {
        let url = format!("{}/versions/loader", FABRIC_META_URL);
        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()).into());
        }

        let versions: Vec<FabricLoaderVersion> = response.json().await?;
        Ok(versions)
    }

    pub async fn get_supported_game_versions(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let url = format!("{}/versions/game", FABRIC_META_URL);
        
        let response = self.http_client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()).into());
        }
        
        #[derive(serde::Deserialize)]
        struct GameVersion {
            version: String,
        }
        
        let versions: Vec<GameVersion> = response.json().await?;
        
        Ok(versions.into_iter().map(|v| v.version).collect())
    }

    pub async fn get_compatible_loader_for_minecraft(
        &self,
        minecraft_version: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/versions/loader/{}", FABRIC_META_URL, minecraft_version);
        
        let response = self.http_client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()).into());
        }
        
        let loaders: Vec<serde_json::Value> = response.json().await?;
        
        if loaders.is_empty() {
            return Err("No Fabric loaders available".into());
        }

        for loader in &loaders {
            if let Some(loader_obj) = loader.get("loader") {
                if let (Some(version), Some(stable)) = (
                    loader_obj.get("version").and_then(|v| v.as_str()),
                    loader_obj.get("stable").and_then(|s| s.as_bool()),
                ) {
                    if stable {
                        return Ok(version.to_string());
                    }
                }
            }
        }

        if let Some(first) = loaders.first() {
            if let Some(version) = first.get("loader")
                .and_then(|l| l.get("version"))
                .and_then(|v| v.as_str()) {
                return Ok(version.to_string());
            }
        }
        
        Err("No compatible Fabric loader found".into())
    }

    pub async fn get_fabric_profile(
        &self,
        minecraft_version: &str,
        loader_version: &str,
    ) -> Result<FabricProfileJson, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/versions/loader/{}/{}/profile/json",
            FABRIC_META_URL, minecraft_version, loader_version
        );

        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {} - {}", status, error_text).into());
        }

        let text = response.text().await?;
        
        let profile: FabricProfileJson = serde_json::from_str(&text)
            .map_err(|e| format!("{}", e))?;
        
        Ok(profile)
    }

    pub async fn install_fabric(
        &self,
        minecraft_version: &str,
        loader_version: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let profile = self.get_fabric_profile(minecraft_version, loader_version).await?;
        
        let fabric_id = profile.id.clone();
        let versions_dir = self.launcher_dir.join("versions").join(&fabric_id);
        let libraries_dir = self.launcher_dir.join("libraries");

        fs::create_dir_all(&versions_dir)?;
        fs::create_dir_all(&libraries_dir)?;

        for lib in &profile.libraries {
            let parts: Vec<&str> = lib.name.split(':').collect();
            if parts.len() != 3 {
                continue;
            }

            let (group, artifact, version) = (parts[0], parts[1], parts[2]);
            let group_path = group.replace('.', "/");
            let jar_name = format!("{}-{}.jar", artifact, version);
            let lib_path = libraries_dir.join(&group_path).join(artifact).join(version).join(&jar_name);

            let base_url = if lib.url.ends_with('/') {
                lib.url.trim_end_matches('/')
            } else {
                &lib.url
            };
            let url = format!("{}/{}/{}/{}/{}", base_url, group_path, artifact, version, jar_name);

            if !lib_path.exists() {
                if let Some(parent) = lib_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                let response = self
                    .http_client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to download library {}: {}", lib.name, e))?;

                if !response.status().is_success() {
                    return Err(format!(
                        "Failed to download library {}: HTTP {}",
                        lib.name,
                        response.status()
                    )
                    .into());
                }

                let bytes = response
                    .bytes()
                    .await
                    .map_err(|e| format!("Failed to read library {}: {}", lib.name, e))?;

                fs::write(&lib_path, bytes)
                    .map_err(|e| format!("Failed to write library {}: {}", lib.name, e))?;
            }
        }

        let profile_path = versions_dir.join(format!("{}.json", fabric_id));
        let profile_json = serde_json::to_string_pretty(&profile)?;
        fs::write(&profile_path, profile_json)?;

        Ok(fabric_id)
    }

}