use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use std::collections::HashMap;

// ===== RUNTIME CONFIG =====

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub microsoft_client_id: String,
    pub database_url: String,
    pub database_key: String,
}

// ===== PUBLIC API MODELS =====

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_expiry: DateTime<Utc>,
    pub username: String,
    pub uuid: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Instance {
    pub name: String,
    pub version: String,
    pub created_at: String,
    pub last_played: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub settings_override: Option<LauncherSettings>,
    #[serde(default)]
    pub icon_path: Option<String>,
    #[serde(default)]
    pub total_playtime_seconds: u64,
}

// ===== INSTANCE IMPORT MODELS =====

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportableInstance {
    pub source: String,
    pub name: String,
    pub path: String,
    pub mc_version: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub size_bytes: u64,
    #[serde(default)]
    pub icon_path: Option<String>,
}

// ===== FRIENDS SYSTEM MODELS =====

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Friend {
    pub uuid: String,
    pub username: String,
    pub status: FriendStatus,
    pub last_seen: DateTime<Utc>,
    pub current_instance: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FriendStatus {
    Online,
    Offline,
    InGame,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FriendRequest {
    pub id: String,
    pub from_uuid: String,
    pub from_username: String,
    pub to_uuid: String,
    pub status: RequestStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RequestStatus {
    Pending,
    Accepted,
    Rejected,
}

// ===== SETTINGS MODELS =====

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LauncherSettings {
    pub java_path: Option<String>,
    #[serde(default = "default_memory")]
    pub memory_mb: u32,
    #[serde(default = "default_auto_navigate_to_console")]
    pub auto_navigate_to_console: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_tab")]
    pub default_tab: String,
    #[serde(default)]
    pub cat_mode: bool,
}

fn default_memory() -> u32 { 4096 }
fn default_auto_navigate_to_console() -> bool { true }
fn default_theme() -> String { "octane".to_string() }
fn default_tab() -> String { "home".to_string() }

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            java_path: None,
            memory_mb: 4096,
            auto_navigate_to_console: true,
            theme: default_theme(),
            default_tab: default_tab(),
            cat_mode: false,
        }
    }
}

// ===== TRASH MODELS =====

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrashItem {
    pub id: String,
    pub original_name: String,
    pub original_type: String,
    pub folder_name: String,
    pub moved_at: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TrashIndex {
    pub items: Vec<TrashItem>,
}

// ===== AUTHENTICATION MODELS =====

#[derive(Serialize, Deserialize)]
pub struct TokenWithExpiry {
    pub token: Arc<str>,
    pub expiry: DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct XboxLiveAuthRequest<'a> {
    pub properties: XboxLiveAuthProperties<'a>,
    pub relying_party: &'a str,
    pub token_type: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct XboxLiveAuthProperties<'a> {
    pub auth_method: &'a str,
    pub site_name: &'a str,
    pub rps_ticket: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct XboxLiveAuthResponse {
    pub issue_instant: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub token: Arc<str>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct XstsAuthRequest<'a> {
    pub properties: XstsAuthProperties<'a>,
    pub relying_party: &'a str,
    pub token_type: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct XstsAuthProperties<'a> {
    pub sandbox_id: &'a str,
    pub user_tokens: &'a [&'a str],
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct XstsAuthResponse {
    pub issue_instant: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub token: Arc<str>,
    pub display_claims: DisplayClaims,
}

#[derive(Deserialize)]
pub struct DisplayClaims {
    pub xui: Vec<HashMap<String, String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftLoginRequest<'a> {
    pub xtoken: &'a str,
    pub platform: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub struct MinecraftLoginResponse {
    pub username: Arc<str>,
    pub access_token: Arc<str>,
    pub expires_in: usize,
}

#[derive(Deserialize)]
pub struct MinecraftProfile {
    pub id: Uuid,
    pub name: Arc<str>,
}

// ===== MULTI-ACCOUNT MODELS =====

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoredAccount {
    pub uuid: String,
    pub username: String,
    pub access_token: String,
    pub refresh_token: String,
    pub token_expiry: DateTime<Utc>,
    pub added_at: String,
    pub last_used: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountsData {
    pub accounts: HashMap<String, StoredAccount>,
    pub active_account_uuid: Option<String>,
}

impl Default for AccountsData {
    fn default() -> Self {
        Self {
            accounts: HashMap::new(),
            active_account_uuid: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountInfo {
    pub uuid: String,
    pub username: String,
    pub is_active: bool,
    pub added_at: String,
    pub last_used: Option<String>,
}

// ===== MINECRAFT VERSION MODELS =====

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinecraftVersion {
    pub id: String,
    pub r#type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: Latest,
    pub versions: Vec<MinecraftVersion>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionDetails {
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndex,
    pub assets: String,
    pub downloads: Downloads,
    pub id: String,
    pub libraries: Vec<Library>,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<Arguments>,
    #[serde(rename = "javaVersion", default)]
    pub java_version: Option<JavaVersion>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedJava {
    pub major_version: u32,
    pub full_version: String,
    pub architecture: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    #[serde(rename = "totalSize")]
    pub total_size: u64,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Downloads {
    pub client: DownloadInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    pub downloads: Option<LibraryDownloads>,
    pub name: String,
    pub rules: Option<Vec<Rule>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<Artifact>,
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Artifact {
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Arguments {
    pub game: Vec<serde_json::Value>,
    pub jvm: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AssetIndexData {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
pub struct AssetObject {
    pub hash: String,
}

// ===== FABRIC LOADER MODELS =====

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FabricProfileJson {
    pub id: String,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub time: String,
    #[serde(rename = "type")]
    pub profile_type: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub arguments: Option<FabricArguments>,
    pub libraries: Vec<FabricProfileLibrary>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FabricArguments {
    pub game: Vec<serde_json::Value>,
    #[serde(default)]
    pub jvm: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FabricProfileLibrary {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FabricLoaderVersion {
    pub separator: String,
    pub build: u32,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

// ===== FORGE LOADER MODELS =====

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ForgeVersion {
    pub minecraft_version: String,
    pub forge_version: String,
    pub full_version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ForgeProfileJson {
    pub id: String,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub time: String,
    #[serde(rename = "type")]
    pub profile_type: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub arguments: Option<ForgeArguments>,
    pub libraries: Vec<ForgeProfileLibrary>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ForgeArguments {
    pub game: Vec<serde_json::Value>,
    #[serde(default)]
    pub jvm: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ForgeProfileLibrary {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub downloads: Option<serde_json::Value>,
}

// ===== NEOFORGE LOADER MODELS =====

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NeoForgeVersion {
    pub minecraft_version: String,
    pub neoforge_version: String,
    pub full_version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NeoForgeProfileJson {
    pub id: String,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub time: String,
    #[serde(rename = "type")]
    pub profile_type: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub arguments: Option<NeoForgeArguments>,
    pub libraries: Vec<NeoForgeProfileLibrary>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NeoForgeArguments {
    pub game: Vec<serde_json::Value>,
    #[serde(default)]
    pub jvm: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NeoForgeProfileLibrary {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub downloads: Option<serde_json::Value>,
}

