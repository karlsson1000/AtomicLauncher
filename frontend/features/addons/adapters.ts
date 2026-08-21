import { invoke } from "@tauri-apps/api/core"
import { Image, Package } from "lucide-svelte"
import type {
  CurseforgeHit,
  CurseforgeSearchResult,
  ModrinthProject,
  ModrinthProjectDetails,
  ModrinthSearchResult,
} from "../../types"

export type ContentSource = "modrinth" | "curseforge"
export type AddonKind = "mods" | "modpacks" | "resourcepacks" | "shaderpacks"

export interface AddonHit {
  id: string
  slug: string
  title: string
  author: string
  summary: string
  imageUrl: string | null
  downloads: number
  categories: string[]
}

export interface AddonSourceAdapter {
  search(query: string, offset: number, limit: number): Promise<{ hits: AddonHit[]; total: number }>
  loadPinned?: () => Promise<AddonHit | null>
}

export interface AddonCategoryConfig {
  kind: AddonKind
  projectType: string
  placeholder: string
  accentMain: string
  accentHover: string
  fallbackIcon: typeof Package
  requiresInstance: boolean
  requiresModdedLoader: boolean
  noInstanceHint: string
  sources: Record<ContentSource, AddonSourceAdapter>
}

export function isModdedLoader(loader: string | null): boolean {
  return loader === "fabric" || loader === "neoforge" || loader === "forge"
}

function toModrinthHit(p: ModrinthProject): AddonHit {
  return {
    id: p.project_id,
    slug: p.slug,
    title: p.title,
    author: p.author,
    summary: p.description,
    imageUrl: p.icon_url,
    downloads: p.downloads,
    categories: p.categories,
  }
}

function toCurseforgeHit(h: CurseforgeHit): AddonHit {
  return {
    id: String(h.id),
    slug: h.slug,
    title: h.name,
    author: h.authors?.[0]?.name || "Unknown",
    summary: h.summary,
    imageUrl: h.logo?.thumbnailUrl ?? null,
    downloads: h.downloadCount,
    categories: h.categories?.map(c => c.name) ?? [],
  }
}

async function searchModrinth(projectType: string, query: string, offset: number, limit: number) {
  const result = await invoke<ModrinthSearchResult>("search_mods", {
    query: query || "",
    facets: JSON.stringify([[`project_type:${projectType}`]]),
    index: query ? "relevance" : "downloads",
    offset,
    limit,
  })
  return { hits: result.hits.map(toModrinthHit), total: result.total_hits }
}

async function searchCurseforge(classId: number, query: string, offset: number, limit: number) {
  const result = await invoke<CurseforgeSearchResult>("search_curseforge_mods", {
    query: query || "",
    classId,
    categoryIds: null,
    gameVersion: null,
    modLoaderTypes: null,
    sortField: query ? 4 : 6,
    sortOrder: query ? null : "desc",
    index: offset,
    pageSize: limit,
  })
  return { hits: result.data.map(toCurseforgeHit), total: result.pagination.totalCount }
}

const CUSTOM_MODPACK_SLUG = "stellarmc-enhanced"
const CUSTOM_MODPACK_AUTHOR = "StellarMC"

async function loadPinnedModpack(): Promise<AddonHit | null> {
  try {
    const details = await invoke<ModrinthProjectDetails>("get_project_details", {
      idOrSlug: CUSTOM_MODPACK_SLUG,
    })
    return {
      id: details.id,
      slug: CUSTOM_MODPACK_SLUG,
      title: details.title,
      author: CUSTOM_MODPACK_AUTHOR,
      summary: details.description,
      imageUrl: details.icon_url ?? null,
      downloads: details.downloads || 0,
      categories: [],
    }
  } catch (error) {
    console.error("Failed to load custom modpack:", error)
    return null
  }
}

export const ADDON_CATEGORIES: Record<AddonKind, AddonCategoryConfig> = {
  mods: {
    kind: "mods",
    projectType: "mod",
    placeholder: "Search mods...",
    accentMain: "#16a34a",
    accentHover: "#22c55e",
    fallbackIcon: Package,
    requiresInstance: true,
    requiresModdedLoader: true,
    noInstanceHint: "Select an instance to manage mods",
    sources: {
      modrinth: { search: (q, o, l) => searchModrinth("mod", q, o, l) },
      curseforge: { search: (q, o, l) => searchCurseforge(6, q, o, l) },
    },
  },
  modpacks: {
    kind: "modpacks",
    projectType: "modpack",
    placeholder: "Search modpacks...",
    accentMain: "#3b82f6",
    accentHover: "#60a5fa",
    fallbackIcon: Package,
    requiresInstance: false,
    requiresModdedLoader: false,
    noInstanceHint: "",
    sources: {
      modrinth: { search: (q, o, l) => searchModrinth("modpack", q, o, l), loadPinned: loadPinnedModpack },
      curseforge: { search: (q, o, l) => searchCurseforge(4471, q, o, l) },
    },
  },
  resourcepacks: {
    kind: "resourcepacks",
    projectType: "resourcepack",
    placeholder: "Search resource packs...",
    accentMain: "#8b5cf6",
    accentHover: "#a78bfa",
    fallbackIcon: Image,
    requiresInstance: true,
    requiresModdedLoader: false,
    noInstanceHint: "Select an instance to manage resource packs",
    sources: {
      modrinth: { search: (q, o, l) => searchModrinth("resourcepack", q, o, l) },
      curseforge: { search: (q, o, l) => searchCurseforge(12, q, o, l) },
    },
  },
  shaderpacks: {
    kind: "shaderpacks",
    projectType: "shaderpack",
    placeholder: "Search shader packs...",
    accentMain: "#f59e0b",
    accentHover: "#fbbf24",
    fallbackIcon: Package,
    requiresInstance: true,
    requiresModdedLoader: false,
    noInstanceHint: "Select an instance to manage shader packs",
    sources: {
      modrinth: { search: (q, o, l) => searchModrinth("shader", q, o, l) },
      curseforge: { search: (q, o, l) => searchCurseforge(6552, q, o, l) },
    },
  },
}
