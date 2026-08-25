use std::time::Duration;
use serde::{Deserialize, Serialize};

const CURSEFORGE_API_BASE: &str = "https://api.curseforge.com/v1";
const MINECRAFT_GAME_ID: u32 = 432;

fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("OctaneLauncher/1.0")
        .http1_only()
        .build()
        .expect("Failed to build CurseForge HTTP client")
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseforgeSearchResult {
    pub data: Vec<CurseforgeHit>,
    pub pagination: CurseforgePagination,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseforgePagination {
    pub index: u32,
    pub page_size: u32,
    pub result_count: u32,
    pub total_count: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseforgeHit {
    pub id: u32,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub download_count: u64,
    pub class_id: Option<u32>,
    pub logo: Option<CurseforgeModAsset>,
    pub authors: Vec<CurseforgeModAuthor>,
    pub categories: Vec<CurseforgeCategory>,
    pub latest_files_indexes: Vec<FileIndex>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileIndex {
    pub game_version: String,
    pub file_id: u32,
    pub mod_loader: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseforgeModAsset {
    pub thumbnail_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurseforgeModAuthor {
    pub id: u32,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseforgeCategory {
    pub name: String,
    pub is_class: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseforgeModDetail {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub logo: Option<CurseforgeModAsset>,
    #[serde(default)]
    pub authors: Vec<CurseforgeModAuthor>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurseforgeGetModFilesResult {
    pub data: Vec<CurseforgeFile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseforgeFile {
    pub id: u32,
    pub mod_id: u32,
    pub file_name: String,
    pub release_type: u32,
    pub file_length: u64,
    pub hashes: Vec<CurseforgeFileHash>,
    pub download_url: Option<String>,
    pub dependencies: Vec<CurseforgeFileDependency>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseforgeFileDependency {
    pub mod_id: u32,
    pub relation_type: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseforgeFileHash {
    pub value: String,
    pub algo: u32,
}

#[derive(Clone)]
pub struct CurseforgeClient {
    http_client: reqwest::Client,
    api_key: String,
}

pub fn extract_sha1(hashes: &[CurseforgeFileHash]) -> Option<&str> {
    hashes
        .iter()
        .find(|h| h.algo == 1)
        .map(|h| h.value.as_str())
}

impl CurseforgeClient {
    pub fn new(api_key: String) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            http_client: build_client(),
            api_key,
        })
    }

    pub async fn search_mods(
        &self,
        query: &str,
        class_id: u32,
        category_ids: Option<&str>,
        game_version: Option<&str>,
        mod_loader_types: Option<&str>,
        sort_field: u32,
        sort_order: Option<&str>,
        index: u32,
        page_size: u32,
    ) -> Result<CurseforgeSearchResult, Box<dyn std::error::Error>> {
        let url = format!("{}/mods/search", CURSEFORGE_API_BASE);
        let mut params: Vec<(String, String)> = vec![
            ("gameId".to_string(), MINECRAFT_GAME_ID.to_string()),
            ("classId".to_string(), class_id.to_string()),
            ("sortField".to_string(), sort_field.to_string()),
            ("index".to_string(), index.to_string()),
            ("pageSize".to_string(), page_size.to_string()),
        ];

        if let Some(order) = sort_order {
            params.push(("sortOrder".to_string(), order.to_string()));
        }

        if !query.is_empty() {
            params.push(("searchFilter".to_string(), query.to_string()));
        }

        if let Some(cats) = category_ids {
            params.push(("categoryIds".to_string(), cats.to_string()));
        }

        if let Some(version) = game_version {
            params.push(("gameVersion".to_string(), version.to_string()));
        }

        if let Some(loaders) = mod_loader_types {
            params.push(("modLoaderTypes".to_string(), loaders.to_string()));
        }

        let response = self
            .http_client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .header("Accept", "application/json")
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(format!("CurseForge API error ({}): {}", status, error_text).into());
        }

        let result: CurseforgeSearchResult = response.json().await?;
        Ok(result)
    }

    pub async fn get_mod_files(
        &self,
        mod_id: u32,
        game_version: Option<&str>,
        mod_loader_type: Option<u32>,
        page_size: Option<u32>,
    ) -> Result<CurseforgeGetModFilesResult, Box<dyn std::error::Error>> {
        let url = format!("{}/mods/{}/files", CURSEFORGE_API_BASE, mod_id);
        let mut params: Vec<(String, String)> = Vec::new();

        if let Some(version) = game_version {
            params.push(("gameVersion".to_string(), version.to_string()));
        }

        if let Some(loader) = mod_loader_type {
            params.push(("modLoaderType".to_string(), loader.to_string()));
        }

        if let Some(ps) = page_size {
            params.push(("pageSize".to_string(), ps.to_string()));
        }

        let response = self
            .http_client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("CurseForge API error: {}", error_text).into());
        }

        let result: CurseforgeGetModFilesResult = response.json().await?;
        Ok(result)
    }

    pub async fn get_files_by_ids(
        &self,
        file_ids: &[u32],
    ) -> Result<CurseforgeGetModFilesResult, Box<dyn std::error::Error>> {
        let url = format!("{}/mods/files", CURSEFORGE_API_BASE);
        let body = serde_json::json!({ "fileIds": file_ids });

        let response = self
            .http_client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("CurseForge API error: {}", error_text).into());
        }

        let result: CurseforgeGetModFilesResult = response.json().await?;
        Ok(result)
    }

    async fn get_mod_description(
        &self,
        mod_id: u32,
    ) -> Result<String, String> {
        let url = format!("{}/mods/{}/description", CURSEFORGE_API_BASE, mod_id);

        let response = self
            .http_client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("CurseForge description request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("CurseForge description API error ({}): {}", status, body));
        }

        let body = response.text().await.map_err(|e| format!("Failed to read description response body: {}", e))?;

        #[derive(Deserialize)]
        struct DescriptionResponse {
            data: serde_json::Value,
        }

        let result: DescriptionResponse = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse CurseForge description response: {} | Body preview: {}",
                e, &body[..body.len().min(200)]))?;

        let desc = match &result.data {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Object(obj) => {
                obj.get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            }
            _ => String::new(),
        };
        Ok(desc)
    }

    pub async fn get_mod(
        &self,
        mod_id: u32,
    ) -> Result<CurseforgeModDetail, String> {
        let url = format!("{}/mods/{}", CURSEFORGE_API_BASE, mod_id);

        let response = self
            .http_client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("CurseForge request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("CurseForge API error ({}): {}", status, body));
        }

        let body = response.text().await.map_err(|e| format!("Failed to read response body: {}", e))?;

        #[derive(Deserialize)]
        struct ModDetailResponse {
            data: CurseforgeModDetail,
        }

        let mut result: CurseforgeModDetail = serde_json::from_str::<ModDetailResponse>(&body)
            .map_err(|e| format!("Failed to parse CurseForge response: {} | Body preview: {}",
                e, &body[..body.len().min(200)]))?
            .data;

        if result.description.is_empty() {
            if let Ok(desc) = self.get_mod_description(mod_id).await {
                result.description = desc;
            }
        }

        Ok(result)
    }
}


