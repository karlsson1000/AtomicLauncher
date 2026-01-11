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

export interface LauncherSettings {
  memory_mb: number
  java_path: string | null
  discord_rpc_enabled: boolean
}

export interface ConsoleLog {
  instance: string
  type: "stdout" | "stderr"
  message: string
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

export interface ModrinthProjectDetails {
  id: string
  slug: string
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