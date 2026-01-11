use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use std::collections::HashMap;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct SendFriendRequestPayload {
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserStatusUpdate {
    pub uuid: String,
    pub status: FriendStatus,
    pub current_server: Option<String>,
}

// ===== TEMPLATE MODELS =====

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceTemplate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub launcher_settings: Option<LauncherSettings>,
    pub minecraft_options: Option<MinecraftOptions>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinecraftOptions {
    pub fov: Option<f32>,
    pub render_distance: Option<i32>,
    pub simulation_distance: Option<i32>,
    pub max_fps: Option<i32>,
    pub fullscreen: Option<bool>,
    pub vsync: Option<bool>,
    pub gui_scale: Option<i32>,
    pub brightness: Option<f32>,
    pub entity_shadows: Option<bool>,
    pub particles: Option<String>,
    pub graphics: Option<String>,
    pub graphics_preset: Option<String>,
    pub smooth_lighting: Option<bool>,
    pub biome_blend: Option<i32>,
    pub mipmap_levels: Option<i32>,
    pub chunk_updates_mode: Option<i32>,
    pub cloud_rendering: Option<String>,
    pub vignette: Option<bool>,
    pub master_volume: Option<f32>,
    pub music_volume: Option<f32>,
    pub record_volume: Option<f32>,
    pub weather_volume: Option<f32>,
    pub blocks_volume: Option<f32>,
    pub hostile_volume: Option<f32>,
    pub neutral_volume: Option<f32>,
    pub players_volume: Option<f32>,
    pub ambient_volume: Option<f32>,
    pub voice_volume: Option<f32>,
    pub mouse_sensitivity: Option<f32>,
    pub invert_mouse: Option<bool>,
    pub raw_input: Option<bool>,
    pub discrete_mouse_scroll: Option<bool>,
    pub touchscreen: Option<bool>,
    pub auto_jump: Option<bool>,
    pub sneak_toggles: Option<bool>,
    pub sprint_toggles: Option<bool>,
    pub narrator: Option<i32>,
    pub chat_opacity: Option<f32>,
    pub chat_line_spacing: Option<f32>,
    pub text_background_opacity: Option<f32>,
    pub chat_height_focused: Option<f32>,
    pub chat_height_unfocused: Option<f32>,
    pub chat_width: Option<f32>,
    pub damage_tilt_strength: Option<f32>,
    pub keybinds: Option<HashMap<String, String>>,
}

// ===== SETTINGS MODELS =====

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LauncherSettings {
    pub java_path: Option<String>,
    #[serde(default = "default_memory")]
    pub memory_mb: u32,
    #[serde(default = "default_discord_rpc_enabled")]
    pub discord_rpc_enabled: bool,
}

fn default_memory() -> u32 {
    2048
}

fn default_discord_rpc_enabled() -> bool {
    true
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            java_path: None,
            memory_mb: 2048,
            discord_rpc_enabled: true,
        }
    }
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
    pub xui: Vec<std::collections::HashMap<String, String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftLoginRequest<'a> {
    pub identity_token: &'a str,
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
    pub classifiers: Option<std::collections::HashMap<String, Artifact>>,
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
    pub objects: std::collections::HashMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    #[allow(dead_code)]
    pub size: u64,
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