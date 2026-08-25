use crate::models::*;
use crate::utils::{get_current_os, library_maven_path, library_maven_url};
use sha1::{Digest, Sha1};
use std::{fs, path::PathBuf, sync::Arc};
use tokio::sync::Semaphore;

const VERSION_MANIFEST_URL: &str = "https://launchermeta.mojang.com/mc/game/version_manifest.json";
const MAX_CONCURRENT_DOWNLOADS: usize = 32;

type DownloadError = Box<dyn std::error::Error + Send + Sync>;

pub struct MinecraftInstaller {
    http_client: reqwest::Client,
    launcher_dir: PathBuf,
}

impl MinecraftInstaller {
    pub fn new(launcher_dir: PathBuf) -> Result<Self, DownloadError> {
        Ok(Self {
            http_client: crate::utils::http::get_client(),
            launcher_dir,
        })
    }

    // Ensure launcher_profiles.json exists
    fn ensure_launcher_profile(&self) -> Result<(), DownloadError> {
        crate::services::loader_common::ensure_launcher_profile(&self.launcher_dir)
            .map_err(|e| -> DownloadError { e.into() })
    }

    async fn download_file(
        &self,
        url: &str,
        path: &PathBuf,
        expected_sha1: Option<&str>,
    ) -> Result<(), DownloadError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let response = self.http_client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(format!("HTTP {} downloading {}", response.status(), url).into());
        }

        let bytes = response.bytes().await?;
        Self::verify_sha1(&bytes, expected_sha1)
            .map_err(|e| format!("{}: {}", url, e))?;
        fs::write(path, bytes)?;

        Ok(())
    }

    fn sha1_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha1::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }

    fn verify_sha1(bytes: &[u8], expected_sha1: Option<&str>) -> Result<(), DownloadError> {
        let Some(expected) = expected_sha1.filter(|s| !s.is_empty()) else {
            return Ok(());
        };

        if Self::sha1_hex(bytes) != expected {
            return Err("SHA-1 checksum mismatch".into());
        }

        Ok(())
    }

    fn file_needs_download(path: &PathBuf, expected_sha1: Option<&str>) -> bool {
        if !path.exists() {
            return true;
        }

        let Some(expected_sha1) = expected_sha1.filter(|s| !s.is_empty()) else {
            return false;
        };

        if let Ok(contents) = fs::read(path) {
            Self::sha1_hex(&contents) != expected_sha1
        } else {
            true
        }
    }

    async fn download_file_with_sha1(
        &self,
        url: &str,
        path: &PathBuf,
        expected_sha1: &str,
    ) -> Result<bool, DownloadError> {
        if !Self::file_needs_download(path, Some(expected_sha1)) {
            return Ok(false);
        }

        self.download_file(url, path, Some(expected_sha1)).await?;
        Ok(true)
    }

    pub async fn get_versions(&self) -> Result<Vec<String>, DownloadError> {
        let response = self.http_client.get(VERSION_MANIFEST_URL).send().await?;
        let manifest: VersionManifest = response.json().await?;

        let versions: Vec<String> = manifest
            .versions
            .iter()
            .filter(|v| v.r#type == "release" || v.r#type == "snapshot")
            .take(500)
            .map(|v| v.id.clone())
            .collect();

        Ok(versions)
    }

    pub async fn get_versions_with_metadata(&self) -> Result<Vec<MinecraftVersion>, DownloadError> {
        let response = self.http_client.get(VERSION_MANIFEST_URL).send().await?;
        let manifest: VersionManifest = response.json().await?;

        let versions: Vec<MinecraftVersion> = manifest
            .versions
            .into_iter()
            .filter(|v| v.r#type == "release" || v.r#type == "snapshot")
            .take(500)
            .collect();

        Ok(versions)
    }

    pub async fn get_versions_by_type(&self, version_type: &str) -> Result<Vec<String>, DownloadError> {
        let response = self.http_client.get(VERSION_MANIFEST_URL).send().await?;
        let manifest: VersionManifest = response.json().await?;

        let versions: Vec<String> = manifest
            .versions
            .iter()
            .filter(|v| v.r#type == version_type)
            .take(500)
            .map(|v| v.id.clone())
            .collect();

        Ok(versions)
    }

    pub async fn install_version(&self, version_id: &str) -> Result<(), DownloadError> {
        // Ensure launcher profile exists
        self.ensure_launcher_profile()?;

        let manifest_response = self.http_client.get(VERSION_MANIFEST_URL).send().await?;
        let manifest: VersionManifest = manifest_response.json().await?;

        let version_info = manifest
            .versions
            .iter()
            .find(|v| v.id == version_id)
            .ok_or_else(|| format!("Version {} not found", version_id))?;

        let version_details: VersionDetails = self
            .http_client
            .get(&version_info.url)
            .send()
            .await?
            .json()
            .await?;

        let versions_dir = self.launcher_dir.join("versions").join(version_id);
        let libraries_dir = self.launcher_dir.join("libraries");
        let assets_dir = self.launcher_dir.join("assets");
        let objects_dir = assets_dir.join("objects");

        fs::create_dir_all(&versions_dir)?;
        fs::create_dir_all(&libraries_dir)?;
        fs::create_dir_all(&objects_dir)?;

        let jar_path = versions_dir.join(format!("{}.jar", version_id));
        self.download_file_with_sha1(
            &version_details.downloads.client.url,
            &jar_path,
            &version_details.downloads.client.sha1,
        )
        .await?;

        let json_path = versions_dir.join(format!("{}.json", version_id));
        let json_content = serde_json::to_string_pretty(&version_details)?;
        fs::write(json_path, json_content)?;

        let current_os = get_current_os();
        let mut library_tasks = Vec::new();
        let mut native_count = 0;
        
        for library in &version_details.libraries {
            let is_native_name = library.name.contains(":natives-");
            let should_include = if let Some(rules) = &library.rules {
                should_include_library(rules, &current_os)
            } else {
                true
            };
            
            if !should_include {
                continue;
            }

            if is_native_name {
                let platform_suffix = if library.name.contains(":natives-windows") {
                    "windows"
                } else if library.name.contains(":natives-linux") {
                    "linux"
                } else {
                    ""
                };
                
                if platform_suffix == current_os {
                    let mut queued_from_artifact = false;
                    if let Some(downloads) = &library.downloads {
                        if let Some(artifact) = &downloads.artifact {
                            queued_from_artifact = true;
                            native_count += 1;
                            library_tasks.push((
                                artifact.url.clone(),
                                libraries_dir.join(&artifact.path),
                                artifact.sha1.clone(),
                            ));
                        }
                    }
                    if !queued_from_artifact {
                        let path = library_maven_path(&libraries_dir, &library.name);
                        if Self::file_needs_download(&path, None) {
                            let url = library_maven_url(&library.name);
                            if !url.is_empty() {
                                native_count += 1;
                                library_tasks.push((url, path, String::new()));
                            }
                        }
                    }
                }
                continue;
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

                        native_count += 1;
                        library_tasks.push((
                            artifact.url.clone(),
                            libraries_dir.join(&artifact.path),
                            artifact.sha1.clone(),
                        ));
                    }
                }

                if let Some(artifact) = &downloads.artifact {
                    library_tasks.push((
                        artifact.url.clone(),
                        libraries_dir.join(&artifact.path),
                        artifact.sha1.clone(),
                    ));
                } else {
                    let path = library_maven_path(&libraries_dir, &library.name);
                    if Self::file_needs_download(&path, None) {
                        let url = library_maven_url(&library.name);
                        if !url.is_empty() {
                            library_tasks.push((url, path, String::new()));
                        }
                    }
                }
            } else {
                let path = library_maven_path(&libraries_dir, &library.name);
                if Self::file_needs_download(&path, None) {
                    let url = library_maven_url(&library.name);
                    if !url.is_empty() {
                        library_tasks.push((url, path, String::new()));
                    }
                }
            }
        }

        if native_count == 0 {
            return Err(format!("No native libraries found for {}", current_os).into());
        }

        self.download_parallel(library_tasks).await?;

        let asset_index_path = assets_dir
            .join("indexes")
            .join(format!("{}.json", version_details.asset_index.id));
        fs::create_dir_all(
            asset_index_path.parent()
                .ok_or_else(|| format!("Invalid asset index path: {}", asset_index_path.display()))?
        )?;

        self.download_file_with_sha1(
            &version_details.asset_index.url,
            &asset_index_path,
            &version_details.asset_index.sha1,
        )
        .await?;

        let asset_index_data: AssetIndexData =
            serde_json::from_str(&fs::read_to_string(&asset_index_path)?)?;

        let mut asset_tasks = Vec::new();
        for (_, asset) in asset_index_data.objects {
            let hash_prefix = asset
                .hash
                .get(0..2)
                .ok_or_else(|| format!("Invalid asset hash: {}", asset.hash))?
                .to_string();
            let asset_path = objects_dir.join(&hash_prefix).join(&asset.hash);
            let asset_url = format!(
                "https://resources.download.minecraft.net/{}/{}",
                hash_prefix, asset.hash
            );

            asset_tasks.push((asset_url, asset_path, asset.hash));
        }

        self.download_parallel(asset_tasks).await?;

        Ok(())
    }

    async fn download_parallel(
        &self,
        tasks: Vec<(String, PathBuf, String)>,
    ) -> Result<usize, DownloadError> {
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS));
        let client = Arc::new(self.http_client.clone());
        let downloaded_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut handles = Vec::new();

        for (url, path, sha1) in tasks {
            let permit = semaphore.clone().acquire_owned().await?;
            let client = client.clone();
            let downloaded_count = downloaded_count.clone();

            let handle = tokio::spawn(async move {
                let result = Self::download_with_client(&client, &url, &path, &sha1).await;
                drop(permit);
                
                if let Ok(true) = result {
                    downloaded_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                
                result
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await??;
        }

        Ok(downloaded_count.load(std::sync::atomic::Ordering::Relaxed))
    }

    async fn download_with_client(
        client: &reqwest::Client,
        url: &str,
        path: &PathBuf,
        expected_sha1: &str,
    ) -> Result<bool, DownloadError> {
        if !Self::file_needs_download(path, Some(expected_sha1)) {
            return Ok(false);
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let response = client.get(url).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()).into());
        }
        
        let bytes = response.bytes().await?;
        Self::verify_sha1(&bytes, Some(expected_sha1))
            .map_err(|e| format!("{}: {}", url, e))?;
        fs::write(path, bytes)?;

        Ok(true)
    }
}

pub fn should_include_library(rules: &[Rule], current_os: &str) -> bool {
    let mut allowed = false;

    for rule in rules {
        let matches = match &rule.os {
            Some(os) => os.name.as_deref() == Some(current_os),
            None => true,
        };

        if matches {
            allowed = rule.action == "allow";
        }
    }

    allowed
}