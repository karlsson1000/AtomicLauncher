<script lang="ts">
  import { invoke } from "@tauri-apps/api/core"
  import { convertFileSrc } from "@tauri-apps/api/core"
  import { Camera, Package, X, Trash2, ExternalLink, ChevronLeft, ChevronRight, Calendar, Check, FolderOpen } from "lucide-svelte"
  import type { Screenshot } from "../../types"
  import { formatFileSize, formatDate } from "../../lib/format"

  let screenshots = $state<Screenshot[]>([])
  let loading = $state(true)
  let selectedInstance = $state<string | null>(null)
  let viewerOpen = $state(false)
  let currentImageIndex = $state(0)
  let sortBy = $state<"date" | "instance">("date")
  let isInstanceDropdownOpen = $state(false)
  $effect(() => { loadScreenshots() })

  async function loadScreenshots() {
    loading = true
    try {
      const allScreenshots = await invoke<Screenshot[]>("get_all_screenshots")
      screenshots = allScreenshots
    } catch (error) {
      console.error("Failed to load screenshots:", error)
    } finally {
      loading = false
    }
  }

  async function handleDeleteScreenshot(screenshot: Screenshot) {
    try {
      await invoke("delete_screenshot", { path: screenshot.path })
      delete startedImages[screenshot.path]
      await loadScreenshots()
    } catch (error) {
      console.error("Failed to delete screenshot:", error)
    }
  }

  async function handleOpenScreenshot(screenshot: Screenshot) {
    try { await invoke("open_screenshot", { path: screenshot.path }) } catch (error) {
      console.error("Failed to open screenshot:", error)
    }
  }

  async function handleOpenScreenshotsFolder() {
    try { await invoke("open_screenshots_folder", { instanceName: selectedInstance }) } catch (error) {
      console.error("Failed to open screenshots folder:", error)
    }
  }

  let filteredScreenshots = $derived(
    screenshots
      .filter((s) => !selectedInstance || s.instance_name === selectedInstance)
      .sort((a, b) => sortBy === "date" ? b.timestamp - a.timestamp : a.instance_name.localeCompare(b.instance_name) || b.timestamp - a.timestamp)
  )

  let instanceCounts = $derived(
    screenshots.reduce((acc, s) => {
      acc[s.instance_name] = (acc[s.instance_name] || 0) + 1
      return acc
    }, {} as Record<string, number>)
  )

  function openViewer(index: number) {
    currentImageIndex = index
    viewerOpen = true
    const screenshot = filteredScreenshots[index]
    if (screenshot) startImage(screenshot.path)
    if (filteredScreenshots[index + 1]) startImage(filteredScreenshots[index + 1].path)
    if (filteredScreenshots[index - 1]) startImage(filteredScreenshots[index - 1].path)
  }

  function closeViewer() { viewerOpen = false }

  function nextImage() {
    const newIndex = (currentImageIndex + 1) % filteredScreenshots.length
    currentImageIndex = newIndex
    if (filteredScreenshots[newIndex + 1]) startImage(filteredScreenshots[newIndex + 1].path)
  }

  function prevImage() {
    const newIndex = currentImageIndex === 0 ? filteredScreenshots.length - 1 : currentImageIndex - 1
    currentImageIndex = newIndex
    if (filteredScreenshots[newIndex - 1]) startImage(filteredScreenshots[newIndex - 1].path)
  }

  function handleViewerDelete(s: Screenshot) {
    handleDeleteScreenshot(s)
    if (filteredScreenshots.length === 1) viewerOpen = false
  }

  let skeletonItems = Array.from({ length: 8 })
  let startedImages = $state<Record<string, boolean>>({})

  function startImage(path: string) {
    if (!startedImages[path]) startedImages[path] = true
  }

  function lazyLoad(node: HTMLElement, path: string) {
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          startImage(path)
          observer.disconnect()
        }
      },
      { rootMargin: '200px' }
    )
    observer.observe(node)
    return { destroy() { observer.disconnect() } }
  }

  $effect(() => {
    if (!viewerOpen) return
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') closeViewer()
      if (e.key === 'ArrowLeft') prevImage()
      if (e.key === 'ArrowRight') nextImage()
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  })
</script>

{#if loading}
  <div class="p-8 space-y-4">
    <div class="max-w-7xl mx-auto">
      <div class="flex items-center justify-between mb-4 invisible">
        <h1 class="text-2xl font-semibold text-[var(--text-primary)] tracking-tight">0 screenshots</h1>
        <div class="flex items-center gap-2">
          <div class="w-8 h-8 bg-[var(--bg-tertiary)] rounded"></div>
          <div class="w-28 h-8 bg-[var(--bg-tertiary)] rounded"></div>
          <div class="w-16 h-8 bg-[var(--bg-tertiary)] rounded"></div>
        </div>
      </div>
      <div class="grid grid-cols-2 sm:grid-cols-2 lg:grid-cols-3 gap-3">
        {#each skeletonItems as _}
          <div class="bg-[var(--bg-tertiary)] rounded-md overflow-hidden">
            <div class="aspect-video bg-[var(--bg-tertiary)] animate-pulse"></div>
            <div class="p-2 space-y-2">
              <div class="h-3 bg-[var(--bg-tertiary)] animate-pulse rounded w-3/4"></div>
              <div class="h-3 bg-[var(--bg-tertiary)] animate-pulse rounded w-1/2"></div>
            </div>
          </div>
        {/each}
      </div>
    </div>
  </div>
{:else}
  <div class="p-8 space-y-4">
    <div class="max-w-7xl mx-auto">
      <div class="flex items-center justify-between mb-4">
        <div>
          <h1 class="text-2xl font-semibold text-[var(--text-primary)] tracking-tight">{filteredScreenshots.length} screenshot{filteredScreenshots.length !== 1 ? 's' : ''}</h1>
        </div>
        <div class="flex items-center gap-2">
          <button
            onclick={handleOpenScreenshotsFolder}
            class="w-8 h-8 bg-[var(--bg-tertiary)] hover:bg-[var(--bg-hover)] text-[var(--text-primary)] rounded flex items-center justify-center transition-colors cursor-pointer"
            title="Open screenshots folder"
          >
            <FolderOpen size={16} />
          </button>

          <div class="relative">
            <button
              type="button"
              onclick={() => isInstanceDropdownOpen = !isInstanceDropdownOpen}
              class="h-8 bg-[var(--bg-tertiary)] px-3 pr-8 text-sm text-[var(--text-primary)] focus:outline-none transition-all text-left cursor-pointer {isInstanceDropdownOpen ? 'rounded-t' : 'rounded'}"
            >
              {selectedInstance ? `${selectedInstance} (${instanceCounts[selectedInstance]})` : `All Instances (${screenshots.length})`}
            </button>
            <div class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#e6e6e6" stroke-width="3">
                <polyline points={isInstanceDropdownOpen ? "18 15 12 9 6 15" : "6 9 12 15 18 9"} />
              </svg>
            </div>
            {#if isInstanceDropdownOpen}
              <div class="absolute z-10 min-w-full w-max bg-[var(--bg-tertiary)] rounded-b max-h-60 overflow-y-auto">
                <button
                  type="button"
                  onclick={() => { selectedInstance = null; isInstanceDropdownOpen = false }}
                  class="w-full px-3 py-2 text-sm text-left hover:bg-[var(--bg-hover)] transition-colors flex items-center justify-between cursor-pointer text-[var(--text-primary)]"
                >
                  <span>All Instances ({screenshots.length})</span>
                  {#if !selectedInstance}
                    <Check size={16} class="text-[var(--text-primary)]" strokeWidth={2} />
                  {/if}
                </button>
                {#each Object.entries(instanceCounts).sort(([a], [b]) => a.localeCompare(b)) as [instanceName, count]}
                  <button
                    type="button"
                    onclick={() => { selectedInstance = instanceName; isInstanceDropdownOpen = false }}
                    class="w-full px-3 py-2 text-sm text-left hover:bg-[var(--bg-hover)] transition-colors flex items-center justify-between cursor-pointer text-[var(--text-primary)]"
                  >
                    <span>{instanceName} ({count})</span>
                    {#if selectedInstance === instanceName}
                      <Check size={16} class="text-[var(--text-primary)]" strokeWidth={2} />
                    {/if}
                  </button>
                {/each}
              </div>
            {/if}
          </div>

          <button
            onclick={() => sortBy = sortBy === "date" ? "instance" : "date"}
            class="px-3 h-8 bg-[var(--bg-tertiary)] hover:bg-[var(--bg-hover)] text-[var(--text-primary)] rounded text-sm flex items-center gap-2 transition-colors cursor-pointer"
          >
            {#if sortBy === "date"}
              <Calendar size={14} />
              Date
            {:else}
              <Package size={14} />
              Instance
            {/if}
          </button>
        </div>
      </div>

      {#if filteredScreenshots.length === 0}
        <div class="flex flex-col items-center justify-center py-16">
          <Camera size={56} class="text-[#3a3f4b] mb-4" strokeWidth={1.5} />
          <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-2">
            {screenshots.length === 0 ? "No screenshots yet" : "No results found"}
          </h3>
          <p class="text-sm text-[var(--text-muted)] text-center max-w-md">
            {screenshots.length === 0 ? "Take some screenshots in-game using F2 to see them here." : "Try selecting a different instance or clearing your filter."}
          </p>
        </div>
      {:else}
        <div class="grid grid-cols-2 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {#each filteredScreenshots as screenshot, index (screenshot.path)}
            <div
              use:lazyLoad={screenshot.path}
              onclick={() => openViewer(index)}
              role="button"
              tabindex="0"
              onkeydown={(e) => { if (e.key === 'Enter') openViewer(index) }}
              class="group relative bg-[var(--bg-tertiary)] rounded-md overflow-hidden cursor-pointer transition-all hover:bg-[var(--bg-hover)] screenshot-card"
            >
              <div class="aspect-video bg-[var(--bg-secondary)] overflow-hidden relative" style="opacity: {startedImages[screenshot.path] ? 1 : 0}; transition: opacity 0.2s">
                {#if !startedImages[screenshot.path]}
                  <div class="absolute inset-0 bg-[var(--bg-tertiary)] animate-pulse"></div>
                {:else}
                  <img
                    src={convertFileSrc(screenshot.path)}
                    alt={screenshot.filename}
                    class="w-full h-full object-cover"
                    loading="lazy"
                    decoding="async"
                  />
                {/if}
              </div>
              <div class="p-2">
                <p class="text-xs font-medium text-[var(--text-primary)] truncate mb-0.5">{screenshot.instance_name}</p>
                <div class="flex items-center justify-between text-xs text-[var(--text-muted)]">
                  <span>{formatDate(screenshot.timestamp)}</span>
                  <span>{formatFileSize(screenshot.size)}</span>
                </div>
              </div>
              <div class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity flex gap-1">
                <button
                  onclick={(e) => { e.stopPropagation(); handleOpenScreenshot(screenshot); }}
                  class="w-7 h-7 bg-[var(--bg-tertiary)]/90 hover:bg-[var(--bg-hover)] rounded flex items-center justify-center transition-colors cursor-pointer"
                  title="Open in default viewer"
                >
                  <ExternalLink size={14} class="text-[var(--text-primary)]" />
                </button>
                <button
                  onclick={(e) => { e.stopPropagation(); handleDeleteScreenshot(screenshot); }}
                  class="w-7 h-7 bg-red-500/90 hover:bg-red-600 rounded flex items-center justify-center transition-colors cursor-pointer"
                  title="Delete screenshot"
                >
                  <Trash2 size={14} class="text-white" />
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    {#if viewerOpen && filteredScreenshots[currentImageIndex]}
      <div class="fixed inset-0 bg-black/80 backdrop-blur-sm z-50 flex flex-col">
        <div class="absolute top-4 left-4 z-30">
          <button onclick={closeViewer} class="w-9 h-9 hover:bg-white/10 text-white/80 hover:text-white rounded-lg flex items-center justify-center transition-colors cursor-pointer">
            <X size={18} />
          </button>
        </div>
        {#if filteredScreenshots.length > 1}
          <span class="absolute top-4 left-[60px] py-[9px] text-sm text-gray-400 select-none z-30">{currentImageIndex + 1} / {filteredScreenshots.length}</span>
        {/if}
        <div class="absolute top-4 right-4 flex items-center gap-2 z-30">
          <button onclick={() => handleOpenScreenshot(filteredScreenshots[currentImageIndex])} class="px-3 h-9 bg-white/10 hover:bg-white/20 text-white rounded-lg text-sm flex items-center gap-2 transition-colors cursor-pointer">
            <ExternalLink size={14} />
            Open
          </button>
          <button onclick={() => handleViewerDelete(filteredScreenshots[currentImageIndex])} class="px-3 h-9 bg-red-500/80 hover:bg-red-500 text-white rounded-lg text-sm flex items-center gap-2 transition-colors cursor-pointer">
            <Trash2 size={14} />
            Delete
          </button>
        </div>

        <div class="flex-1 flex items-center justify-center relative min-h-0 px-4 pt-14 pb-4">
          {#if filteredScreenshots.length > 1}
            <button onclick={prevImage} class="absolute left-6 top-1/2 -translate-y-1/2 w-10 h-10 hover:bg-white/10 rounded-full flex items-center justify-center transition-colors cursor-pointer text-white/60 hover:text-white z-10">
              <ChevronLeft size={24} />
            </button>
            <button onclick={nextImage} class="absolute right-6 top-1/2 -translate-y-1/2 w-10 h-10 hover:bg-white/10 rounded-full flex items-center justify-center transition-colors cursor-pointer text-white/60 hover:text-white z-10">
              <ChevronRight size={24} />
            </button>
          {/if}
          <div class="flex items-center justify-center w-full h-full" style="opacity: {startedImages[filteredScreenshots[currentImageIndex].path] ? 1 : 0}; transition: opacity 0.2s">
            {#if !startedImages[filteredScreenshots[currentImageIndex].path]}
              <div class="w-full max-w-[900px] aspect-video bg-[var(--bg-tertiary)] animate-pulse rounded-lg"></div>
            {:else}
              <img src={convertFileSrc(filteredScreenshots[currentImageIndex].path)} alt={filteredScreenshots[currentImageIndex].filename} class="max-w-[85vw] max-h-[83vh] object-contain rounded-md" />
            {/if}
          </div>
        </div>
      </div>
    {/if}
  </div>
{/if}
