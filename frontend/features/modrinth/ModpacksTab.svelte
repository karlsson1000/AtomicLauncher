<script lang="ts">
  import { Search, Download, Loader2, Package } from "lucide-svelte"
  import { invoke } from "@tauri-apps/api/core"
  import { untrack } from "svelte"
  import type { Snippet } from "svelte"
  import type { Instance, ModrinthSearchResult, ModrinthProject, ModrinthVersion, ModrinthProjectDetails } from "../../types"
  import { formatDownloads } from "../../lib/format"

  let {
    instances = [],
    onRefreshInstances = undefined as (() => void) | undefined,
    onShowCreationToast = undefined as ((instanceName: string) => void) | undefined,
    sourceSelector,
    modsSelector,
    hideToolbar = false,
    searchQuery = undefined as string | undefined,
    onSearchQueryChange = undefined as ((query: string) => void) | undefined,
    onViewProjectDetail = undefined as ((source: "modrinth" | "curseforge", projectId: string, slug: string, projectType: string, author?: string) => void) | undefined,
  }: {
    instances: Instance[]
    onRefreshInstances?: () => void
    onShowCreationToast?: (instanceName: string) => void
    sourceSelector?: Snippet
    modsSelector?: Snippet
    hideToolbar?: boolean
    searchQuery?: string
    onSearchQueryChange?: (query: string) => void
    onViewProjectDetail?: (source: "modrinth" | "curseforge", projectId: string, slug: string, projectType: string, author?: string) => void
  } = $props()

  const CUSTOM_MODPACK_SLUG = "stellarmc-enhanced"
  const CUSTOM_MODPACK_AUTHOR = "StellarMC"
  const ITEMS_PER_PAGE = 20

  let internalSearchQuery = $state("")
  let debounceSearchQuery = $derived(searchQuery ?? internalSearchQuery)
  let hits = $state<ModrinthProject[]>([])
  let isSearching = $state(false)
  let isLoadingMore = $state(false)
  let sentinelEl: HTMLDivElement | undefined = $state()
  let searchTimeout: ReturnType<typeof setTimeout> | undefined
  let offset = $state(0)
  let hasMore = true

  let selectedModpack = $state<ModrinthProject | null>(null)
  let modpackVersions = $state<Record<string, ModrinthVersion[]>>({})
  let loadingVersions = $state<Set<string>>(new Set())
  let installingVersions = $state<Set<string>>(new Set())
  let installationStatus = $state<Record<string, "success" | "error">>({})
  let customModpack = $state<ModrinthProject | null>(null)
  let customModpackLoaded = false
  $effect(() => {
    loadCustomModpack()
  })

  $effect(() => {
    debounceSearchQuery;
    if (searchTimeout) clearTimeout(searchTimeout)
    offset = 0
    hasMore = true
    searchTimeout = setTimeout(() => {
      fetchModpacks(0, true)
    }, untrack(() => hits.length === 0 ? 0 : 300))
    return () => clearTimeout(searchTimeout)
  })

  $effect(() => {
    if (!sentinelEl) return
    const el = sentinelEl
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && !isLoadingMore && !isSearching && hasMore) {
          loadMore()
        }
      },
      { threshold: 0.1 }
    )
    observer.observe(el)
    return () => observer.disconnect()
  })

  let displayedHits = $derived.by<ModrinthProject[]>(() => {
    const modpack = customModpack
    if (!modpack || debounceSearchQuery.trim() || (offset === 0 && hits.length === 0)) {
      return hits
    }
    const withoutCustom = hits.filter(m => m.project_id !== modpack.project_id)
    if (!debounceSearchQuery.trim()) {
      return [modpack, ...withoutCustom]
    }
    return hits
  })

  async function fetchModpacks(offsetVal: number, replace: boolean) {
    const query = debounceSearchQuery.trim()
    if (replace) isSearching = true
    else isLoadingMore = true

    try {
      const facets: string[][] = [["project_type:modpack"]]

      const result = await invoke<ModrinthSearchResult>("search_mods", {
        query: query || "",
        facets: JSON.stringify(facets),
        index: query ? "relevance" : "downloads",
        offset: offsetVal,
        limit: ITEMS_PER_PAGE,
      })

      offset = offsetVal + result.hits.length
      hasMore = offsetVal + result.hits.length < result.total_hits
      if (replace) {
        hits = result.hits
      } else {
        const ids = new Set(hits.map(h => h.project_id))
        hits = [...hits, ...result.hits.filter(h => !ids.has(h.project_id))]
      }
    } catch (error) {
      console.error("Search error:", error)
    } finally {
      if (replace) isSearching = false
      else isLoadingMore = false
    }
  }

  function loadMore() {
    if (!hasMore || isLoadingMore || isSearching) return
    fetchModpacks(offset, false)
  }

  async function loadCustomModpack() {
    if (customModpackLoaded) return
    customModpackLoaded = true
    try {
      const projectDetails = await invoke<ModrinthProjectDetails>("get_project_details", {
        idOrSlug: CUSTOM_MODPACK_SLUG,
      })
      const modpackData: ModrinthProject = {
        project_id: projectDetails.id,
        slug: projectDetails.slug,
        title: projectDetails.title,
        description: projectDetails.description,
        author: CUSTOM_MODPACK_AUTHOR,
        icon_url: projectDetails.icon_url,
        downloads: projectDetails.downloads || 0,
        follows: 0,
        date_created: "",
        date_modified: "",
        latest_version: "",
        license: "",
        client_side: "required",
        server_side: "optional",
        project_type: "modpack",
        categories: [],
        versions: [],
        gallery: [],
        display_categories: [],
      }
      customModpack = modpackData
    } catch (error) {
      console.error("Failed to load custom modpack:", error)
    }
  }

  async function handleModpackSelect(modpack: ModrinthProject) {
    selectedModpack = modpack
    const projectId = modpack.project_id
    if (!modpackVersions[projectId]) {
      loadingVersions = new Set(loadingVersions).add(projectId)
      try {
        const versions = await invoke<ModrinthVersion[]>("get_modpack_versions", {
          idOrSlug: modpack.slug,
          gameVersion: null,
        })
        modpackVersions = { ...modpackVersions, [projectId]: versions }
      } catch (error) {
        console.error("Failed to load versions:", error)
      } finally {
        const s = new Set(loadingVersions)
        s.delete(projectId)
        loadingVersions = s
      }
    }
  }

  async function handleInstallModpack(version: ModrinthVersion, modpack: ModrinthProject) {
    const instanceName = modpack.title
    const existingInstance = instances.find(i => i.name === instanceName)
    const finalName = existingInstance ? `${instanceName}-${Date.now()}` : instanceName

    installingVersions = new Set(installingVersions).add(version.id)
    if (onShowCreationToast) onShowCreationToast(finalName)

    try {
      await invoke("install_modpack", {
        modpackSlug: modpack.slug,
        instanceName: finalName,
        versionId: version.id,
        preferredGameVersion: null,
      })
      installationStatus = { ...installationStatus, [modpack.project_id]: "success" }
      if (onRefreshInstances) setTimeout(() => onRefreshInstances(), 500)
      setTimeout(() => {
        const s = new Set(installingVersions)
        s.delete(version.id)
        installingVersions = s
        const st = { ...installationStatus }
        delete st[modpack.project_id]
        installationStatus = st
      }, 3000)
    } catch (error) {
      console.error("Failed to install modpack:", error)
      installationStatus = { ...installationStatus, [modpack.project_id]: "error" }
      const s = new Set(installingVersions)
      s.delete(version.id)
      installingVersions = s
      setTimeout(() => {
        const st = { ...installationStatus }
        delete st[modpack.project_id]
        installationStatus = st
      }, 5000)
    }
  }

</script>

<div class="max-w-7xl mx-auto h-full flex flex-col">
  {#if !hideToolbar}
    <div class="sticky top-0 z-10 bg-[var(--content-bg)] pb-4 flex-shrink-0">
      <div class="flex gap-2 items-stretch">
        {#if sourceSelector}
          {@render sourceSelector()}
        {/if}
        <div class="relative flex-1 rounded-md bg-[var(--bg-tertiary)]">
          <Search size={16} class="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--text-muted)] z-20 pointer-events-none" strokeWidth={2} />
          <input
            type="text"
            placeholder="Search modpacks..."
            value={searchQuery ?? internalSearchQuery}
            oninput={(e) => onSearchQueryChange ? onSearchQueryChange((e.target as HTMLInputElement).value) : (internalSearchQuery = (e.target as HTMLInputElement).value)}
            class="w-full bg-transparent rounded-md pl-10 pr-4 py-2.5 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none transition-all relative z-10"
          />
          {#if isSearching}
            <div class="absolute right-3 top-1/2 -translate-y-1/2 z-20">
              <Loader2 size={16} class="animate-spin text-[#3b82f6]" />
            </div>
          {/if}
        </div>
        {#if modsSelector}
          {@render modsSelector()}
        {/if}
      </div>
    </div>
  {/if}

  <div class="flex-1 min-h-0 {hideToolbar ? '' : 'grid grid-cols-1 lg:grid-cols-3 gap-2'}">
    <div class="{hideToolbar ? '' : 'lg:col-span-2'} space-y-3 overflow-y-auto pr-2">
      {#each displayedHits as modpack (modpack.project_id)}
        <div
          class="rounded-md overflow-hidden cursor-pointer transition-all {selectedModpack?.project_id === modpack.project_id ? 'bg-[var(--bg-elevated)]' : 'bg-[var(--bg-tertiary)]'}"
          role="button"
          tabindex="0"
          onkeydown={(e) => { if (e.key === 'Enter') handleModpackSelect(modpack); }}
          onclick={() => { if (onViewProjectDetail) onViewProjectDetail("modrinth", modpack.project_id, modpack.slug, "modpack", modpack.author); handleModpackSelect(modpack); }}
        >
          <div class="flex min-h-0 relative z-0">
            {#if modpack.icon_url}
              <div class="w-24 h-24 flex items-center justify-center flex-shrink-0 rounded m-2">
                <img src={modpack.icon_url} alt={modpack.title} class="w-full h-full object-contain rounded" />
              </div>
            {:else}
              <div class="w-24 h-24 bg-gradient-to-br from-[#3b82f6]/10 to-[#60a5fa]/10 flex items-center justify-center flex-shrink-0 rounded m-2">
                <Package size={48} class="text-[#3b82f6]" />
              </div>
            {/if}
            <div class="flex-1 min-w-0 py-2 px-3 flex items-center gap-3">
              <div class="flex-1 min-w-0">
                <div class="flex items-start justify-between gap-2 mb-0">
                  <h3 class="font-semibold text-base text-[var(--text-primary)] truncate">{modpack.title}</h3>
                  <span class="text-xs text-[var(--text-muted)] whitespace-nowrap">by {modpack.author}</span>
                </div>
                <p class="text-sm text-[var(--text-muted)] line-clamp-2 mb-2">{modpack.description}</p>
                <div class="flex items-center gap-2 text-xs flex-wrap">
                  <span class="flex items-center gap-1 bg-[var(--bg-secondary)] px-2 py-1 rounded text-[var(--text-muted)]">
                    <Download size={12} />
                    {formatDownloads(modpack.downloads)}
                  </span>
                  {#each modpack.categories.slice(0, 2) as category}
                    <span class="bg-[var(--bg-secondary)] px-2 py-1 rounded text-[var(--text-muted)]">{category}</span>
                  {/each}
                </div>
              </div>
            </div>
          </div>
        </div>
      {/each}

      <div bind:this={sentinelEl} class="flex items-center justify-center py-4">
        {#if isLoadingMore}
          <Loader2 size={20} class="animate-spin text-[#3b82f6]" />
        {/if}
      </div>
    </div>

    {#if !hideToolbar && selectedModpack}
      {@const modpack = selectedModpack}
      <div class="bg-[var(--bg-tertiary)] rounded-md p-3 sticky top-0 self-start">
        <div class="flex gap-3 mb-4">
          {#if selectedModpack.icon_url}
            <img src={selectedModpack.icon_url} alt={selectedModpack.title} class="w-16 h-16 rounded" />
          {/if}
          <div class="flex-1 min-w-0">
            <h2 class="text-xl font-semibold text-[var(--text-primary)] truncate">{selectedModpack.title}</h2>
            <p class="text-sm text-[var(--text-muted)]">by {selectedModpack.author}</p>
          </div>
        </div>

        <p class="text-sm text-[var(--text-muted)] mb-4 leading-relaxed">{selectedModpack.description}</p>
        <div class="flex gap-2 mb-5 text-xs flex-wrap">
          <span class="flex items-center gap-1 bg-[var(--bg-secondary)] px-2 py-1 rounded text-[var(--text-muted)]">
            <Download size={12} />
            {formatDownloads(selectedModpack.downloads)}
          </span>
          <span class="bg-[var(--bg-secondary)] px-2 py-1 rounded text-[var(--text-muted)]">{selectedModpack.follows.toLocaleString()} followers</span>
        </div>

        <div class="pt-1">
          <h3 class="font-semibold text-sm text-[var(--text-primary)] mb-3">Versions</h3>
          {#if loadingVersions.has(selectedModpack.project_id)}
            <div class="text-center py-6"><Loader2 size={20} class="animate-spin text-[#3b82f6] mx-auto" /></div>
          {:else if modpackVersions[selectedModpack.project_id]?.length > 0}
            <div class="space-y-2 max-h-[40vh] overflow-y-auto pr-1.5">
              {#each modpackVersions[selectedModpack.project_id] as version (version.id)}
                {@const installing = installingVersions.has(version.id)}
                {@const projectStatus = installationStatus[selectedModpack.project_id]}
                <div class="bg-[var(--bg-secondary)] rounded p-3 flex items-center justify-between gap-2">
                  <div class="flex-1 min-w-0">
                    <div class="text-sm font-medium text-[var(--text-primary)] truncate">{version.name}</div>
                    <div class="text-xs text-[#3a3f4b] truncate mt-0.5">{version.game_versions?.join(", ")}</div>
                  </div>
                  <button
                    onclick={() => handleInstallModpack(version, modpack)}
                    disabled={installing}
                    class="px-3 py-2 bg-[#3b82f6] hover:bg-[#60a5fa] disabled:opacity-50 disabled:cursor-not-allowed text-white rounded text-xs font-medium whitespace-nowrap transition-all cursor-pointer flex items-center gap-1"
                  >
                    {#if installing}
                      <Loader2 size={14} class="animate-spin" />
                    {:else if projectStatus === "success"}
                      Installed
                    {:else if projectStatus === "error"}
                      Error
                    {:else}
                      <Download size={14} />Install
                    {/if}
                  </button>
                </div>
              {/each}
            </div>
          {:else}
            <p class="text-sm text-[#3a3f4b] text-center py-3">No versions available</p>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>
