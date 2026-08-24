export interface AuthData {
  username: string
  uuid: string
  access_token: string
}

export interface AuthResponse {
  username: string
  uuid: string
  access_token: string
}

export interface Instance {
  name: string
  version: string
  loader: string | null
  loader_version: string | null
  created_at: string
  last_played: string | null
  settings_override: LauncherSettings | null
  icon_path: string | null
  total_playtime_seconds?: number
}

export interface FabricVersion {
  version: string
  stable: boolean
}

export interface NeoForgeVersion {
  minecraft_version: string
  neoforge_version: string
  full_version: string
}

export interface ForgeVersion {
  minecraft_version: string
  forge_version: string
  full_version: string
}

export interface LauncherSettings {
  memory_mb: number
  java_path: string | null
  auto_navigate_to_console?: boolean
  theme?: string
  default_tab?: string
  cat_mode?: boolean
}

export interface MinecraftOptions {
  fov: number | null
  render_distance: number | null
  max_fps: number | null
  fullscreen: boolean | null
  vsync: boolean | null
  gui_scale: number | null
  brightness: number | null
  entity_shadows: boolean | null
  particles: string | null
  graphics: string | null
  smooth_lighting: boolean | null
  biome_blend: number | null
  master_volume: number | null
  music_volume: number | null
  mouse_sensitivity: number | null
  auto_jump: boolean | null
  keybinds: Record<string, string> | null
}

export interface ModrinthSearchResult {
  hits: ModrinthProject[]
  offset: number
  limit: number
  total_hits: number
}

export interface ModrinthProject {
  project_id: string
  project_type: string
  slug: string
  author: string
  title: string
  description: string
  categories: string[]
  display_categories: string[]
  versions: string[]
  downloads: number
  follows: number
  icon_url: string | null
  date_created: string
  date_modified: string
  latest_version: string
  license: string
  client_side: string
  server_side: string
  gallery: string[]
}

export interface ConsoleLog {
  instance: string
  type: "stdout" | "stderr"
  message: string
}

export interface ModrinthProjectDetails {
  id: string
  slug?: string
  project_type: string
  team: string
  title: string
  description: string
  body: string
  body_url: string | null
  published: string
  updated: string
  approved: string | null
  status: string
  moderator_message: string | null
  license: {
    id: string
    name: string
    url: string | null
  }
  client_side: string
  server_side: string
  downloads: number
  followers: number
  categories: string[]
  additional_categories: string[]
  game_versions: string[]
  loaders: string[]
  versions: string[]
  icon_url: string | null
  issues_url: string | null
  source_url: string | null
  wiki_url: string | null
  discord_url: string | null
  donation_urls: Array<{
    id: string
    platform: string
    url: string
  }>
  gallery: Array<{
    url: string
    featured: boolean
    title: string | null
    description: string | null
    created: string
    ordering: number
  }>
}

export interface ModrinthVersion {
  id: string
  project_id: string
  author_id: string
  featured: boolean
  name: string
  version_number: string
  changelog: string | null
  date_published: string
  downloads: number
  version_type: string
  files: ModrinthFile[]
  dependencies: ModrinthDependency[]
  game_versions: string[]
  loaders: string[]
}

export interface ModrinthFile {
  hashes: {
    sha512: string
    sha1: string
  }
  url: string
  filename: string
  primary: boolean
  size: number
  file_type: string | null
}

export interface ModrinthDependency {
  version_id: string | null
  project_id: string | null
  file_name: string | null
  dependency_type: string
}

export interface ModFile {
  filename: string
  size: number
}

export interface ModHash {
  filename: string
  size: number
  disabled: boolean
  sha1_hash: string
}

export interface ModFileWithMetadata {
  filename: string
  size: number
  project_id: string | null
  name: string | null
  description: string | null
  icon_url: string | null
  author: string | null
  downloads: number | null
  disabled: boolean
  current_version_id: string | null
}

export interface AccountInfo {
  uuid: string
  username: string
  is_active: boolean
  added_at: string
  last_used: string | null
}

export interface UpdateInfo {
  current_version: string
  new_version: string
}

export interface PlayerInfo {
  name: string
  uuid: string
}

export interface ServerInfo {
  name: string
  address: string
  port: number
  status: "online" | "offline" | "unknown"
  players_online?: number
  players_max?: number
  players_list?: PlayerInfo[]
  version?: string
  motd?: string
  motd_html?: string[]
  favicon?: string
  last_checked?: number
  ping?: number
}

export interface McSrvStatResponse {
  online: boolean
  ip?: string
  port?: number
  hostname?: string
  version?: string
  protocol?: {
    version: number
    name?: string
  }
  icon?: string
  motd?: {
    clean?: string[]
    html?: string[]
  }
  players?: {
    online: number
    max: number
    list?: PlayerInfo[]
  }
}

export interface Screenshot {
  path: string
  filename: string
  instance_name: string
  timestamp: number
  size: number
}

export interface CachedSkin {
  url: string
  variant: "classic" | "slim"
  timestamp: number
}

export interface RecentSkin {
  url: string
  variant: "classic" | "slim"
  timestamp: number
}

export interface Cape {
  id: string
  state: string
  url: string
  alias: string
  icon?: string
}

export interface Snapshot {
  id: string
  title: string
  version: string
  type: string
  date?: string
  image?: {
    title: string
    url: string
  }
  body?: string
  contentPath?: string
  shortText?: string
}

export interface SnapshotsResponse {
  version: number
  entries: Snapshot[]
}

export type FriendStatus = "online" | "offline" | "ingame"

export interface Friend {
  uuid: string
  username: string
  status: FriendStatus
  last_seen: string
  current_instance: string | null
}

export type RequestStatus = "pending" | "accepted" | "rejected"

export interface FriendRequest {
  id: string
  from_uuid: string
  from_username: string
  to_uuid: string
  status: RequestStatus
  created_at: string
}

export interface CurseforgeSearchResult {
  data: CurseforgeHit[]
  pagination: CurseforgePagination
}

export interface CurseforgePagination {
  index: number
  pageSize: number
  resultCount: number
  totalCount: number
}

export interface CurseforgeHit {
  id: number
  name: string
  slug: string
  summary: string
  downloadCount: number
  classId: number | null
  logo: CurseforgeModAsset | null
  authors: CurseforgeModAuthor[]
  categories: CurseforgeCategory[]
  latestFilesIndexes: CurseforgeFileIndex[]
}

export interface CurseforgeFileIndex {
  gameVersion: string
  fileId: number
  modLoader: number | null
}

export interface CurseforgeModAsset {
  thumbnailUrl: string
}

export interface CurseforgeModAuthor {
  id: number
  name: string
  url: string
}

export interface CurseforgeCategory {
  name: string
  isClass: boolean
}

export interface CurseforgeModDetail {
  id: number
  name: string
  description: string
  logo: CurseforgeModAsset | null
  authors: CurseforgeModAuthor[]
}

export interface CurseforgeGetModFilesResult {
  data: CurseforgeFile[]
}

export interface CurseforgeFile {
  id: number
  modId: number
  fileName: string
  releaseType: number
  fileLength: number
  hashes: CurseforgeFileHash[]
  downloadUrl: string | null
  dependencies: CurseforgeFileDependency[]
}

export interface CurseforgeFileDependency {
  modId: number
  relationType: number
}



export interface CurseforgeFileHash {
  value: string
  algo: number
}

export interface ImportableInstance {
  source: "prism" | "modrinth" | "curseforge"
  name: string
  path: string
  mc_version: string | null
  loader: string | null
  loader_version: string | null
  size_bytes: number
  icon_path: string | null
}