<script lang="ts">
  import { Search, Download, Loader2, Image } from "lucide-svelte"
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

  let selectedPack = $state<ModrinthProject | null>(null)
  let packVersions = $state<ModrinthVersion[]>([])
  let isLoadingVersions = $state(false)
  let downloadingPacks = $state<Set<string>>(new Set())
  let installedPacks = $state<Set<string>>(new Set())

  $effect(() => {
    if (selectedInstance) loadInstalledResourcePacks()
  })

  $effect(() => {
    debounceSearchQuery;
    if (searchTimeout) clearTimeout(searchTimeout)
    offset = 0
    hasMore = true
    searchTimeout = setTimeout(() => {
      fetchPacks(0, true)
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

  async function fetchPacks(offsetVal: number, replace: boolean) {
    const query = debounceSearchQuery.trim()
    if (replace) isSearching = true
    else isLoadingMore = true
    try {
      const result = await invoke<ModrinthSearchResult>("search_mods", {
        query: query || "",
        facets: JSON.stringify([["project_type:resourcepack"]]),
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
    fetchPacks(offset, false)
  }

  async function loadInstalledResourcePacks() {
    if (!selectedInstance) return
    try {
      const packs = await invoke<string[]>("get_installed_resourcepacks", { instanceName: selectedInstance.name })
      installedPacks = new Set(packs)
    } catch (error) {
      console.error("Failed to load installed resource packs:", error)
    }
  }

  async function handlePackSelect(pack: ModrinthProject) {
    if (!selectedInstance) return
    selectedPack = pack
    isLoadingVersions = true
    try {
      const versions = await invoke<ModrinthVersion[]>("get_mod_versions", {
        idOrSlug: pack.project_id,
      })
      packVersions = versions
    } catch (error) {
      console.error("Failed to load versions:", error)
    } finally {
      isLoadingVersions = false
    }
  }

  function isPackInstalled(version: ModrinthVersion): boolean {
    return version.files.some(file => installedPacks.has(file.filename))
  }

  async function handleDownloadPack(version: ModrinthVersion) {
    if (!selectedInstance) return
    const primaryFile = version.files.find(f => f.primary) || version.files[0]
    if (!primaryFile) return
    downloadingPacks = new Set(downloadingPacks).add(version.id)
    try {
      await invoke<string>("download_resourcepack", {
        instanceName: selectedInstance.name,
        downloadUrl: primaryFile.url,
        filename: primaryFile.filename,
      })
      installedPacks = new Set(installedPacks).add(primaryFile.filename)
    } catch (error) {
      console.error("Download error:", error)
    } finally {
      const n = new Set(downloadingPacks)
      n.delete(version.id)
      downloadingPacks = n
    }
  }

</script>

{#if !selectedInstance}
  <div class="max-w-7xl mx-auto">
    <div class="flex items-center justify-center py-20">
      <div class="text-center">
        <Image size={64} class="mx-auto mb-4 text-[var(--text-muted)]" strokeWidth={1.5} />
        <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-2">No instance selected</h3>
        <p class="text-sm text-[var(--text-muted)]">Select an instance to manage resource packs</p>
      </div>
    </div>
  </div>
{:else}
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
              placeholder="Search resource packs..."
              value={searchQuery ?? internalSearchQuery}
              oninput={(e) => onSearchQueryChange ? onSearchQueryChange((e.target as HTMLInputElement).value) : (internalSearchQuery = (e.target as HTMLInputElement).value)}
              class="w-full bg-transparent rounded-md pl-10 pr-4 py-2.5 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none transition-all relative z-10"
            />
            {#if isSearching}
              <div class="absolute right-3 top-1/2 -translate-y-1/2 z-20">
                <Loader2 size={16} class="animate-spin text-[#8b5cf6]" />
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
        {#each hits as pack (pack.project_id)}
          <div
            class="rounded-md overflow-hidden cursor-pointer transition-all {selectedPack?.project_id === pack.project_id ? 'bg-[var(--bg-elevated)]' : 'bg-[var(--bg-tertiary)]'}"
          role="button"
          tabindex="0"
          onkeydown={(e) => { if (e.key === 'Enter') handlePackSelect(pack); }}
          onclick={() => { if (onViewProjectDetail) onViewProjectDetail("modrinth", pack.project_id, pack.slug, "resourcepack", pack.author); handlePackSelect(pack); }}
        >
          <div class="flex min-h-0 relative z-0">
            {#if pack.icon_url}
              <div class="w-24 h-24 flex items-center justify-center flex-shrink-0 rounded m-2">
                <img src={pack.icon_url} alt={pack.title} class="w-full h-full object-contain rounded" />
                </div>
              {:else}
                <div class="w-24 h-24 bg-gradient-to-br from-[#8b5cf6]/10 to-[#a78bfa]/10 flex items-center justify-center flex-shrink-0 rounded m-2">
                  <Image size={48} class="text-[#8b5cf6]" />
                </div>
              {/if}
              <div class="flex-1 min-w-0 py-2 px-3 flex items-center gap-3">
                <div class="flex-1 min-w-0">
                  <div class="flex items-start justify-between gap-2 mb-0">
                    <h3 class="font-semibold text-base text-[var(--text-primary)] truncate">{pack.title}</h3>
                    <span class="text-xs text-[var(--text-muted)] whitespace-nowrap">by {pack.author}</span>
                  </div>
                  <p class="text-sm text-[var(--text-muted)] line-clamp-2 mb-2">{pack.description}</p>
                  <div class="flex items-center gap-2 text-xs flex-wrap">
                    <span class="flex items-center gap-1 bg-[var(--bg-secondary)] px-2 py-1 rounded text-[var(--text-muted)]">
                      <Download size={12} />
                      {formatDownloads(pack.downloads)}
                    </span>
                    {#each pack.categories.slice(0, 2) as category}
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
            <Loader2 size={20} class="animate-spin text-[#8b5cf6]" />
          {/if}
        </div>
      </div>

      {#if !hideToolbar && selectedPack}
        <div class="bg-[var(--bg-tertiary)] rounded-md p-3 sticky top-0 self-start">
          <div class="flex gap-3 mb-4">
            {#if selectedPack.icon_url}
              <img src={selectedPack.icon_url} alt={selectedPack.title} class="w-16 h-16 rounded" />
            {/if}
            <div class="flex-1 min-w-0">
              <h2 class="text-xl font-semibold text-[var(--text-primary)] truncate">{selectedPack.title}</h2>
              <p class="text-sm text-[var(--text-muted)]">by {selectedPack.author}</p>
            </div>
          </div>

          <p class="text-sm text-[var(--text-muted)] mb-4 leading-relaxed">{selectedPack.description}</p>
          <div class="flex gap-2 mb-5 text-xs flex-wrap">
            <span class="flex items-center gap-1 bg-[var(--bg-secondary)] px-2 py-1 rounded text-[var(--text-muted)]">
              <Download size={12} />
              {formatDownloads(selectedPack.downloads)}
            </span>
            <span class="bg-[var(--bg-secondary)] px-2 py-1 rounded text-[var(--text-muted)]">
              {selectedPack.follows.toLocaleString()} followers
            </span>
          </div>

          <div class="pt-4">
            <h3 class="font-semibold text-sm text-[var(--text-primary)] mb-3">Versions</h3>
            {#if isLoadingVersions}
              <div class="text-center py-6">
                <Loader2 size={20} class="animate-spin text-[#8b5cf6] mx-auto" />
              </div>
            {:else if packVersions.length === 0}
              <p class="text-sm text-[var(--text-muted)] text-center py-3">No compatible versions</p>
            {:else}
              <div class="space-y-2 max-h-[40vh] overflow-y-auto pr-1.5">
                {#each packVersions as version (version.id)}
                  {@const installed = isPackInstalled(version)}
                  {@const downloading = downloadingPacks.has(version.id)}
                  <div class="bg-[var(--bg-secondary)] rounded p-3 flex items-center justify-between gap-2">
                    <div class="flex-1 min-w-0">
                      <div class="text-sm font-medium text-[var(--text-primary)] truncate">{version.name}</div>
                      <div class="text-xs text-[var(--text-muted)] truncate mt-0.5">
                        {version.game_versions[0]}
                      </div>
                    </div>
                    <button
                      onclick={() => handleDownloadPack(version)}
                      disabled={!selectedInstance || downloading || installed}
                      class="px-3 py-2 bg-[#8b5cf6] hover:bg-[#a78bfa] disabled:opacity-50 disabled:cursor-not-allowed text-white rounded text-xs font-medium whitespace-nowrap transition-all cursor-pointer flex items-center gap-1"
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
{/if}
