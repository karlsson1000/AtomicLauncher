<script lang="ts">
  import { invoke } from "@tauri-apps/api/core"
  import { Search, Download, Loader2, Package } from "lucide-svelte"
  import type { Instance, CurseforgeSearchResult, CurseforgeHit, CurseforgeGetModFilesResult, CurseforgeFile } from "../../types"
  import { formatDownloads } from "../../lib/format"
  import type { Snippet } from "svelte"
  import { untrack } from "svelte"

  let {
    selectedInstance,
    hideToolbar,
    sourceSelector,
    modsSelector,
    searchQuery,
    onSearchQueryChange,
    onViewProjectDetail,
  }: {
    selectedInstance: Instance | null
    hideToolbar?: boolean
    sourceSelector?: Snippet
    modsSelector?: Snippet
    searchQuery?: string
    onSearchQueryChange?: (query: string) => void
    onViewProjectDetail?: (source: "modrinth" | "curseforge", projectId: string, slug: string, projectType: string, author?: string) => void
  } = $props()

  const CLASS_ID = 6552
  const SEARCH_PLACEHOLDER = "Search shader packs..."
  const itemsPerPage = 20

  let internalSearchQuery = $state("")
  let hits: CurseforgeHit[] = $state([])
  let isSearching = $state(false)
  let isLoadingMore = $state(false)
  let sentinelEl: HTMLDivElement | undefined = $state()
  let searchTimeout: ReturnType<typeof setTimeout> | undefined
  let offset = 0
  let hasMore = true

  let selectedItem: CurseforgeHit | null = $state(null)
  let itemFiles: CurseforgeFile[] = $state([])
  let isLoadingFiles = $state(false)
  let downloadingItems: Set<number> = $state(new Set())
  let installedFiles: Set<string> = $state(new Set())

  let debounceSearchQuery = $derived(searchQuery ?? internalSearchQuery)

  $effect(() => {
    if (selectedInstance) loadInstalledItems()
  })

  $effect(() => {
    debounceSearchQuery;

    clearTimeout(searchTimeout)
    offset = 0
    hasMore = true
    const delay = untrack(() => hits.length === 0 ? 0 : 300)
    searchTimeout = setTimeout(() => {
      fetchItems(0, true)
    }, delay)

    return () => { clearTimeout(searchTimeout) }
  })

  $effect(() => {
    isLoadingMore;
    isSearching;

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && !isLoadingMore && !isSearching && hasMore) {
          loadMore()
        }
      },
      { threshold: 0.1 }
    )
    if (sentinelEl) observer.observe(sentinelEl)
    return () => observer.disconnect()
  })

  async function fetchItems(offsetVal: number, replace: boolean) {
    const query = debounceSearchQuery.trim()
    if (replace) isSearching = true
    else isLoadingMore = true
    try {
      const result = await invoke<CurseforgeSearchResult>("search_curseforge_mods", {
        query: query || "",
        classId: CLASS_ID,
        categoryIds: null,
        gameVersion: null,
        modLoaderTypes: null,
        sortField: query ? 4 : 6,
        sortOrder: query ? null : "desc",
        index: offsetVal,
        pageSize: itemsPerPage,
      })
      offset = offsetVal + result.data.length
      hasMore = offsetVal + result.data.length < result.pagination.totalCount
      if (replace) {
        hits = result.data
        selectedItem = null
      } else {
        const ids = new Set(hits.map(h => h.id))
        hits = [...hits, ...result.data.filter(h => !ids.has(h.id))]
      }
    } catch (error) {
      console.error("CurseForge search error:", error)
    } finally {
      if (replace) isSearching = false
      else isLoadingMore = false
    }
  }

  function loadMore() {
    if (!hasMore || isLoadingMore || isSearching) return
    fetchItems(offset, false)
  }

  async function loadInstalledItems() {
    if (!selectedInstance) return
    try {
      const packs = await invoke<string[]>("get_installed_shaderpacks", { instanceName: selectedInstance.name })
      installedFiles = new Set(packs)
    } catch (error) {
      console.error("Failed to load installed shader packs:", error)
    }
  }

  async function handleItemSelect(item: CurseforgeHit) {
    selectedItem = item
    isLoadingFiles = true
    try {
      const result = await invoke<CurseforgeGetModFilesResult>("get_curseforge_mod_files", {
        modId: item.id,
        gameVersion: null,
        modLoaderType: null,
        pageSize: 20,
      })
      itemFiles = result.data
    } catch (error) {
      console.error("Failed to load files:", error)
    } finally {
      isLoadingFiles = false
    }
  }

  function isItemInstalled(file: CurseforgeFile): boolean {
    return installedFiles.has(file.fileName)
  }

  async function handleDownload(file: CurseforgeFile) {
    if (!selectedInstance || !file.downloadUrl) return
    downloadingItems = new Set(downloadingItems).add(file.id)
    try {
      await invoke<string>("download_curseforge_file", {
        instanceName: selectedInstance.name,
        downloadUrl: file.downloadUrl,
        filename: file.fileName,
        targetFolder: "shaderpacks",
      })
      installedFiles = new Set(installedFiles).add(file.fileName)
    } catch (error) {
      console.error("Download error:", error)
    } finally {
      const next = new Set(downloadingItems)
      next.delete(file.id)
      downloadingItems = next
    }
  }

</script>

{#if !selectedInstance}
  <div class="max-w-7xl mx-auto">
    {#if !hideToolbar}
      <div class="sticky top-0 z-10 bg-[var(--content-bg)] pb-4">
        <div class="flex gap-2 items-stretch">
          {@render sourceSelector?.()}
          <div class="relative flex-1 rounded-md bg-[var(--bg-tertiary)]">
            <Search size={16} class="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--text-muted)] z-20 pointer-events-none" strokeWidth={2} />
            <input
              type="text"
              placeholder={SEARCH_PLACEHOLDER}
              value={searchQuery ?? internalSearchQuery}
              oninput={(e) => onSearchQueryChange ? onSearchQueryChange(e.currentTarget.value) : internalSearchQuery = e.currentTarget.value}
              class="w-full bg-transparent rounded-md pl-10 pr-4 py-2.5 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none transition-all relative z-10"
            />
          </div>
          {@render modsSelector?.()}
        </div>
      </div>
    {/if}
    <div class="flex items-center justify-center py-20">
      <div class="text-center">
        <Package size={64} class="mx-auto mb-4 text-[var(--text-muted)]" strokeWidth={1.5} />
        <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-2">No instance selected</h3>
        <p class="text-sm text-[var(--text-muted)]">Select an instance to manage shader packs</p>
      </div>
    </div>
  </div>
{:else}
  <div class="max-w-7xl mx-auto h-full flex flex-col">
    {#if !hideToolbar}
      <div class="sticky top-0 z-10 bg-[var(--content-bg)] pb-4 flex-shrink-0">
        <div class="flex gap-2 items-stretch">
          {@render sourceSelector?.()}
          <div class="relative flex-1 rounded-md bg-[var(--bg-tertiary)]">
            <Search size={16} class="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--text-muted)] z-20 pointer-events-none" strokeWidth={2} />
            <input
              type="text"
              placeholder={SEARCH_PLACEHOLDER}
              value={searchQuery ?? internalSearchQuery}
              oninput={(e) => onSearchQueryChange ? onSearchQueryChange(e.currentTarget.value) : internalSearchQuery = e.currentTarget.value}
              class="w-full bg-transparent rounded-md pl-10 pr-4 py-2.5 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none transition-all relative z-10"
            />
            {#if isSearching}
              <div class="absolute right-3 top-1/2 -translate-y-1/2 z-20">
                <Loader2 size={16} class="animate-spin text-[#f59e0b]" />
              </div>
            {/if}
          </div>
          {@render modsSelector?.()}
        </div>
      </div>
    {/if}

    <div class="flex-1 min-h-0 {hideToolbar ? '' : 'grid grid-cols-1 lg:grid-cols-3 gap-2'}">
      <div class="{hideToolbar ? '' : 'lg:col-span-2'} space-y-3 overflow-y-auto pr-2">
        {#each hits as item (item.id)}
          <div
            class="rounded-md overflow-hidden cursor-pointer transition-all {selectedItem?.id === item.id ? 'bg-[var(--bg-elevated)]' : 'bg-[var(--bg-tertiary)]'}"
          role="button"
          tabindex="0"
          onkeydown={(e) => { if (e.key === 'Enter') handleItemSelect(item); }}
          onclick={() => { if (onViewProjectDetail) onViewProjectDetail("curseforge", String(item.id), item.slug, "shaderpack", item.authors[0]?.name); handleItemSelect(item); }}
        >
          <div class="flex min-h-0 relative z-0">
            {#if item.logo?.thumbnailUrl}
              <div class="w-24 h-24 flex items-center justify-center flex-shrink-0 rounded m-2">
                <img src={item.logo.thumbnailUrl} alt={item.name} class="w-full h-full object-contain rounded" />
                </div>
              {:else}
                <div class="w-24 h-24 bg-gradient-to-br from-[#f59e0b]/10 to-[#fbbf24]/10 flex items-center justify-center flex-shrink-0 rounded m-2">
                  <Package size={48} class="text-[#f59e0b]" />
                </div>
              {/if}
              <div class="flex-1 min-w-0 py-2 px-3 flex items-center gap-3">
                <div class="flex-1 min-w-0">
                  <div class="flex items-start justify-between gap-2 mb-0">
                    <h3 class="font-semibold text-base text-[var(--text-primary)] truncate">{item.name}</h3>
                    <span class="text-xs text-[var(--text-muted)] whitespace-nowrap">
                      by {item.authors?.[0]?.name || "Unknown"}
                    </span>
                  </div>
                  <p class="text-sm text-[var(--text-muted)] line-clamp-2 mb-2">{item.summary}</p>
                  <div class="flex items-center gap-2 text-xs flex-wrap">
                    <span class="flex items-center gap-1 bg-[var(--bg-secondary)] px-2 py-1 rounded text-[var(--text-muted)]">
                      <Download size={12} />
                      {formatDownloads(item.downloadCount)}
                    </span>
                    {#each item.categories?.slice(0, 2) ?? [] as cat (cat.name)}
                      <span class="bg-[var(--bg-secondary)] px-2 py-1 rounded text-[var(--text-muted)]">{cat.name}</span>
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

      {#if !hideToolbar && selectedItem}
        <div class="bg-[var(--bg-tertiary)] rounded-md p-3 sticky top-0 self-start">
          <div class="flex gap-3 mb-4">
            {#if selectedItem.logo?.thumbnailUrl}
              <img src={selectedItem.logo.thumbnailUrl} alt={selectedItem.name} class="w-16 h-16 rounded" />
            {/if}
            <div class="flex-1 min-w-0">
              <h2 class="text-xl font-semibold text-[var(--text-primary)] truncate">{selectedItem.name}</h2>
              <p class="text-sm text-[var(--text-muted)]">by {selectedItem.authors?.[0]?.name || "Unknown"}</p>
            </div>
          </div>
          <p class="text-sm text-[var(--text-muted)] mb-4 leading-relaxed">{selectedItem.summary}</p>
          <div class="flex gap-2 mb-5 text-xs flex-wrap">
            <span class="flex items-center gap-1 bg-[var(--bg-secondary)] px-2 py-1 rounded text-[var(--text-muted)]">
              <Download size={12} />
              {formatDownloads(selectedItem.downloadCount)}
            </span>
          </div>
          <div class="pt-1">
            <h3 class="font-semibold text-sm text-[var(--text-primary)] mb-3">Files</h3>
            {#if isLoadingFiles}
              <div class="text-center py-6"><Loader2 size={20} class="animate-spin text-[#f59e0b] mx-auto" /></div>
            {:else if itemFiles.length === 0}
              <p class="text-sm text-[var(--text-muted)] text-center py-3">No files available</p>
            {:else}
              <div class="space-y-2 max-h-[40vh] overflow-y-auto pr-1.5">
                {#each itemFiles as file (file.id)}
                  {@const installed = isItemInstalled(file)}
                  {@const downloading = downloadingItems.has(file.id)}
                  <div class="bg-[var(--bg-secondary)] rounded p-3 flex items-center justify-between gap-2">
                    <div class="flex-1 min-w-0">
                      <div class="text-sm font-medium text-[var(--text-primary)] truncate">{file.fileName}</div>
                      <div class="text-xs text-[var(--text-muted)] truncate mt-0.5">
                        {file.releaseType === 1 ? "Release" : file.releaseType === 2 ? "Beta" : file.releaseType === 3 ? "Alpha" : "Other"} • {(file.fileLength / 1024 / 1024).toFixed(1)} MB
                      </div>
                    </div>
                    <button
                      onclick={() => handleDownload(file)}
                      disabled={!selectedInstance || downloading || installed || !file.downloadUrl}
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
{/if}
