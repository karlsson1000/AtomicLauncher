<script lang="ts">
  import { Search, Download, Loader2, Package } from "lucide-svelte"
  import { invoke } from "@tauri-apps/api/core"
  import { untrack } from "svelte"
  import type { Snippet } from "svelte"
  import type { Instance, ModrinthSearchResult, ModrinthProject, ModrinthVersion } from "../../types"
  import { formatDownloads } from "../../lib/format"

  let {
    selectedInstance = null,
    sourceSelector,
    modsSelector,
    hideToolbar = false,
    searchQuery = undefined as string | undefined,
    onSearchQueryChange = undefined as ((query: string) => void) | undefined,
    onViewProjectDetail = undefined as ((source: "modrinth" | "curseforge", projectId: string, slug: string, projectType: string, author?: string) => void) | undefined,
  }: {
    selectedInstance: Instance | null
    sourceSelector?: Snippet
    modsSelector?: Snippet
    hideToolbar?: boolean
    searchQuery?: string
    onSearchQueryChange?: (query: string) => void
    onViewProjectDetail?: (source: "modrinth" | "curseforge", projectId: string, slug: string, projectType: string, author?: string) => void
  } = $props()

  const ITEMS_PER_PAGE = 20

  let internalSearchQuery = $state("")
  let debounceSearchQuery = $derived(searchQuery ?? internalSearchQuery)
  let hits = $state<ModrinthProject[]>([])
  let isSearching = $state(false)
  let isLoadingMore = $state(false)
  let sentinelEl: HTMLDivElement | undefined = $state()
  let searchTimeout: ReturnType<typeof setTimeout> | undefined
  let offset = 0
  let hasMore = true

  let selectedShader = $state<ModrinthProject | null>(null)
  let shaderVersions = $state<ModrinthVersion[]>([])
  let isLoadingVersions = $state(false)
  let downloadingShaders = $state<Set<string>>(new Set())
  let installedShaderFiles = $state<Set<string>>(new Set())

  $effect(() => {
    if (selectedInstance) loadInstalledShaders()
  })

  $effect(() => {
    debounceSearchQuery;
    if (searchTimeout) clearTimeout(searchTimeout)
    offset = 0
    hasMore = true
    searchTimeout = setTimeout(() => {
      fetchShaders(0, true)
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

  async function fetchShaders(offsetVal: number, replace: boolean) {
    const query = debounceSearchQuery.trim()
    if (replace) isSearching = true
    else isLoadingMore = true
    try {
      const result = await invoke<ModrinthSearchResult>("search_mods", {
        query: query || "",
        facets: JSON.stringify([["project_type:shader"]]),
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
    fetchShaders(offset, false)
  }

  async function loadInstalledShaders() {
    if (!selectedInstance) return
    try {
      const shaders = await invoke<string[]>("get_installed_shaderpacks", { instanceName: selectedInstance.name })
      installedShaderFiles = new Set(shaders)
    } catch (error) {
      console.error("Failed to load installed shader packs:", error)
    }
  }

  async function handleShaderSelect(shader: ModrinthProject) {
    selectedShader = shader
    isLoadingVersions = true
    try {
      const versions = await invoke<ModrinthVersion[]>("get_mod_versions", {
        idOrSlug: shader.project_id,
        loaders: undefined,
      })
      shaderVersions = versions
    } catch (error) {
      console.error("Failed to load versions:", error)
    } finally {
      isLoadingVersions = false
    }
  }

  function isShaderInstalled(version: ModrinthVersion): boolean {
    return version.files.some(file => installedShaderFiles.has(file.filename))
  }

  async function handleDownloadShader(version: ModrinthVersion) {
    if (!selectedInstance) return
    const primaryFile = version.files.find(f => f.primary) || version.files[0]
    if (!primaryFile) return
    downloadingShaders = new Set(downloadingShaders).add(version.id)
    try {
      await invoke<string>("download_shaderpack", {
        instanceName: selectedInstance.name, downloadUrl: primaryFile.url, filename: primaryFile.filename,
      })
      installedShaderFiles = new Set(installedShaderFiles).add(primaryFile.filename)
    } catch (error) {
      console.error("Download error:", error)
    } finally {
      const n = new Set(downloadingShaders)
      n.delete(version.id)
      downloadingShaders = n
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
            placeholder="Search shader packs..."
            value={searchQuery ?? internalSearchQuery}
            oninput={(e) => onSearchQueryChange ? onSearchQueryChange((e.target as HTMLInputElement).value) : (internalSearchQuery = (e.target as HTMLInputElement).value)}
            class="w-full bg-transparent rounded-md pl-10 pr-4 py-2.5 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none transition-all relative z-10"
          />
          {#if isSearching}
            <div class="absolute right-3 top-1/2 -translate-y-1/2 z-20">
              <Loader2 size={16} class="animate-spin text-[#f59e0b]" />
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
      {#each hits as shader (shader.project_id)}
        <div
          class="rounded-md overflow-hidden cursor-pointer transition-all {selectedShader?.project_id === shader.project_id ? 'bg-[var(--bg-elevated)]' : 'bg-[var(--bg-tertiary)]'}"
          role="button"
          tabindex="0"
          onkeydown={(e) => { if (e.key === 'Enter') handleShaderSelect(shader); }}
          onclick={() => { if (onViewProjectDetail) onViewProjectDetail("modrinth", shader.project_id, shader.slug, "shaderpack", shader.author); handleShaderSelect(shader); }}
        >
          <div class="flex min-h-0 relative z-0">
            {#if shader.icon_url}
              <div class="w-24 h-24 flex items-center justify-center flex-shrink-0 rounded m-2">
                <img src={shader.icon_url} alt={shader.title} class="w-full h-full object-contain rounded" />
              </div>
            {:else}
              <div class="w-24 h-24 bg-gradient-to-br from-[#f59e0b]/10 to-[#fbbf24]/10 flex items-center justify-center flex-shrink-0 rounded m-2">
                <Package size={48} class="text-[#f59e0b]" />
              </div>
            {/if}
            <div class="flex-1 min-w-0 py-2 px-3 flex items-center gap-3">
              <div class="flex-1 min-w-0">
                <div class="flex items-start justify-between gap-2 mb-0">
                  <h3 class="font-semibold text-base text-[var(--text-primary)] truncate">{shader.title}</h3>
                  <span class="text-xs text-[var(--text-muted)] whitespace-nowrap">by {shader.author}</span>
                </div>
                <p class="text-sm text-[var(--text-muted)] line-clamp-2 mb-2">{shader.description}</p>
                <div class="flex items-center gap-2 text-xs flex-wrap">
                  <span class="flex items-center gap-1 bg-[var(--bg-secondary)] px-2 py-1 rounded text-[var(--text-muted)]">
                    <Download size={12} />
                    {formatDownloads(shader.downloads)}
                  </span>
                  {#each shader.categories.slice(0, 2) as category}
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
          <Loader2 size={20} class="animate-spin text-[#f59e0b]" />
        {/if}
      </div>
    </div>

    {#if !hideToolbar && selectedShader}
      <div class="bg-[var(--bg-tertiary)] rounded-md p-3 sticky top-0 self-start">
        <div class="flex gap-3 mb-4">
          {#if selectedShader.icon_url}
            <img src={selectedShader.icon_url} alt={selectedShader.title} class="w-16 h-16 rounded" />
          {/if}
          <div class="flex-1 min-w-0">
            <h2 class="text-xl font-semibold text-[var(--text-primary)] truncate">{selectedShader.title}</h2>
            <p class="text-sm text-[var(--text-muted)]">by {selectedShader.author}</p>
          </div>
        </div>

        <p class="text-sm text-[var(--text-muted)] mb-4 leading-relaxed">{selectedShader.description}</p>
        <div class="flex gap-2 mb-5 text-xs flex-wrap">
          <span class="flex items-center gap-1 bg-[var(--bg-secondary)] px-2 py-1 rounded text-[var(--text-muted)]">
            <Download size={12} />
            {formatDownloads(selectedShader.downloads)}
          </span>
          <span class="bg-[var(--bg-secondary)] px-2 py-1 rounded text-[var(--text-muted)]">
            {selectedShader.follows.toLocaleString()} followers
          </span>
        </div>

        <div class="pt-4">
          <h3 class="font-semibold text-sm text-[var(--text-primary)] mb-3">Versions</h3>
          {#if isLoadingVersions}
            <div class="text-center py-6">
              <Loader2 size={20} class="animate-spin text-[#f59e0b] mx-auto" />
            </div>
          {:else if shaderVersions.length === 0}
            <p class="text-sm text-[var(--text-muted)] text-center py-3">No compatible versions</p>
          {:else}
            <div class="space-y-2 max-h-[40vh] overflow-y-auto pr-1.5">
              {#each shaderVersions as version (version.id)}
                {@const installed = isShaderInstalled(version)}
                {@const downloading = downloadingShaders.has(version.id)}
                <div class="bg-[var(--bg-secondary)] rounded p-3 flex items-center justify-between gap-2">
                  <div class="flex-1 min-w-0">
                    <div class="text-sm font-medium text-[var(--text-primary)] truncate">{version.name}</div>
                    <div class="text-xs text-[var(--text-muted)] truncate mt-0.5">
                      {version.game_versions[0]}
                    </div>
                  </div>
                  <button
                    onclick={() => handleDownloadShader(version)}
                    disabled={!selectedInstance || downloading || installed}
                    class="px-3 py-2 bg-[#f59e0b] hover:bg-[#fbbf24] disabled:opacity-50 disabled:cursor-not-allowed text-white rounded text-xs font-medium whitespace-nowrap transition-all cursor-pointer flex items-center gap-1"
                  >
                    {#if downloading}
                      <Loader2 size={14} class="animate-spin" />
                    {:else if installed}
                      Installed
                    {:else}
                      <Download size={14} />Install
                    {/if}
                  </button>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>
