use serde::{Deserialize, Serialize};

const MODRINTH_API_BASE: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthSearchResult {
    pub hits: Vec<ModrinthProject>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthProject {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    pub client_side: String,
    pub server_side: String,
    pub project_type: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub color: Option<u32>,
    pub project_id: String,
    pub author: String,
    pub display_categories: Option<Vec<String>>,
    pub versions: Vec<String>,
    pub follows: u32,
    pub date_created: String,
    pub date_modified: String,
    pub latest_version: Option<String>,
    pub license: String,
    pub gallery: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthProjectDetails {
    pub body: String,
    pub description: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub id: String,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthVersion {
    pub id: String,
    pub project_id: String,
    pub author_id: String,
    pub featured: bool,
    pub name: String,
    pub version_number: String,
    pub changelog: Option<String>,
    pub date_published: String,
    pub downloads: u32,
    pub version_type: String,
    pub files: Vec<VersionFile>,
    pub dependencies: Vec<Dependency>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionFile {
    pub hashes: FileHashes,
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    pub file_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileHashes {
    pub sha1: String,
    pub sha512: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub file_name: Option<String>,
    pub dependency_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionFileResponse {
    pub id: String,
    pub project_id: String,
    pub files: Vec<VersionFile>,
}

pub struct ModrinthClient {
    http_client: reqwest::Client,
}

impl ModrinthClient {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self { http_client: crate::utils::http::get_client() })
    }

    pub async fn get_version_files_by_hashes(
        &self,
        hashes: &[String],
    ) -> Result<std::collections::HashMap<String, VersionFileResponse>, Box<dyn std::error::Error>> {
        let url = format!("{}/version_files", MODRINTH_API_BASE);
        #[derive(Serialize)]
        struct HashRequest<'a> {
            hashes: &'a [String],
            algorithm: &'a str,
        }
        let body = HashRequest { hashes, algorithm: "sha1" };
        let response = self.http_client.post(&url).json(&body).send().await?;
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Modrinth API error: {}", error_text).into());
        }
        let result: std::collections::HashMap<String, VersionFileResponse> = response.json().await?;
        Ok(result)
    }

    pub async fn get_latest_version_files_by_hashes(
        &self,
        hashes: &[String],
        game_versions: Option<Vec<String>>,
        loaders: Option<Vec<String>>,
    ) -> Result<std::collections::HashMap<String, VersionFileResponse>, Box<dyn std::error::Error>> {
        let url = format!("{}/version_files/update", MODRINTH_API_BASE);
        #[derive(Serialize)]
        struct HashRequest<'a> {
            hashes: &'a [String],
            algorithm: &'a str,
            game_versions: Option<Vec<String>>,
            loaders: Option<Vec<String>>,
        }
        let body = HashRequest { hashes, algorithm: "sha1", game_versions, loaders };
        let response = self.http_client.post(&url).json(&body).send().await?;
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Modrinth API error: {}", error_text).into());
        }
        let result: std::collections::HashMap<String, VersionFileResponse> = response.json().await?;
        Ok(result)
    }

    pub async fn get_projects_batch(
        &self,
        project_ids: &[String],
    ) -> Result<Vec<ModrinthProjectDetails>, Box<dyn std::error::Error>> {
        let url = format!("{}/projects", MODRINTH_API_BASE);
        let ids_json = serde_json::to_string(project_ids)?;
        let response = self.http_client.get(&url).query(&[("ids", &ids_json)]).send().await?;
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Modrinth API error: {}", error_text).into());
        }
        let result: Vec<ModrinthProjectDetails> = response.json().await?;
        Ok(result)
    }

    pub async fn search_projects(
        &self,
        query: &str,
        facets: Option<&str>,
        index: Option<&str>,
        offset: Option<u32>,
        limit: Option<u32>,
    ) -> Result<ModrinthSearchResult, Box<dyn std::error::Error>> {
        let url = format!("{}/search", MODRINTH_API_BASE);
        let mut params = vec![("query", query.to_string())];

        if let Some(facets) = facets {
            params.push(("facets", facets.to_string()));
        }

        if let Some(index) = index {
            params.push(("index", index.to_string()));
        }

        if let Some(offset) = offset {
            params.push(("offset", offset.to_string()));
        }

        if let Some(limit) = limit {
            params.push(("limit", limit.to_string()));
        }

        let response = self
            .http_client
            .get(&url)
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Modrinth API error: {}", error_text).into());
        }

        let result: ModrinthSearchResult = response.json().await?;
        Ok(result)
    }

    pub async fn get_project(
        &self,
        id_or_slug: &str,
    ) -> Result<ModrinthProjectDetails, Box<dyn std::error::Error>> {
        let url = format!("{}/project/{}", MODRINTH_API_BASE, id_or_slug);

        let response = self.http_client.get(&url).send().await
            .map_err(|e| format!("Modrinth request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Modrinth API error ({}): {}", status, body).into());
        }

        let body = response.text().await
            .map_err(|e| format!("Failed to read Modrinth response body: {}", e))?;

        let project: ModrinthProjectDetails = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse Modrinth response: {} | Body preview: {}",
                e, &body[..body.len().min(200)]))?;
        Ok(project)
    }

    pub async fn get_project_versions(
        &self,
        id_or_slug: &str,
        loaders: Option<Vec<String>>,
        game_versions: Option<Vec<String>>,
    ) -> Result<Vec<ModrinthVersion>, Box<dyn std::error::Error>> {
        let url = format!("{}/project/{}/version", MODRINTH_API_BASE, id_or_slug);

        let mut params: Vec<(&str, String)> = Vec::new();
        params.push(("include_changelog", "false".to_string()));

        if let Some(loaders) = loaders {
            params.push(("loaders", format!("[\"{}\"]", loaders.join("\",\""))));
        }

        if let Some(game_versions) = game_versions {
            params.push((
                "game_versions",
                format!("[\"{}\"]", game_versions.join("\",\"")),
            ));
        }

        let response = self.http_client.get(&url).query(&params).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Modrinth API error: {}", error_text).into());
        }

        let versions: Vec<ModrinthVersion> = response.json().await?;
        Ok(versions)
    }

    pub async fn get_version_by_id(
        &self,
        id_or_slug: &str,
        version_id: &str,
    ) -> Result<ModrinthVersion, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/project/{}/version/{}",
            MODRINTH_API_BASE, id_or_slug, version_id
        );

        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Modrinth API error: {}", error_text).into());
        }

        let version: ModrinthVersion = response.json().await?;
        Ok(version)
    }

    pub async fn download_mod_file(
        &self,
        url: &str,
        destination: &std::path::Path,
        expected_sha1: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        crate::utils::download::download_file_verified(url, destination, expected_sha1)
            .await
            .map_err(|e| e.into())
    }
}