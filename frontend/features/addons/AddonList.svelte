<script lang="ts">
  import { Download, Loader2 } from "lucide-svelte"
  import { untrack } from "svelte"
  import type { Instance } from "../../types"
  import { formatDownloads } from "../../lib/format"
  import type {
    AddonCategoryConfig,
    AddonHit,
    AddonSourceAdapter,
    ContentSource,
  } from "./adapters"
  import { isModdedLoader } from "./adapters"

  const ITEMS_PER_PAGE = 20

  let {
    source,
    category,
    searchQuery = "",
    onViewProjectDetail,
    selectedInstance = null,
    instances = [],
    onSetSelectedInstance = (_: Instance) => {},
  }: {
    source: ContentSource
    category: AddonCategoryConfig
    searchQuery?: string
    onViewProjectDetail: (source: ContentSource, projectId: string, slug: string, projectType: string, author?: string) => void
    selectedInstance?: Instance | null
    instances?: Instance[]
    onSetSelectedInstance?: (instance: Instance) => void
  } = $props()

  const adapter: AddonSourceAdapter = $derived(category.sources[source])

  let hits: AddonHit[] = $state([])
  let isSearching = $state(false)
  let isLoadingMore = $state(false)
  let sentinelEl: HTMLDivElement | undefined = $state()
  let searchTimeout: ReturnType<typeof setTimeout> | undefined
  let offset = 0
  let hasMore = true

  let selectedId: string | null = $state(null)
  let pinnedHit: AddonHit | null = $state(null)

  $effect(() => {
    if (!category.requiresModdedLoader) return
    if (!selectedInstance || !isModdedLoader(selectedInstance.loader)) {
      const modded = instances.filter(i => isModdedLoader(i.loader))
      if (modded.length > 0) onSetSelectedInstance(modded[0])
    }
  })

  $effect(() => {
    let cancelled = false
    adapter.loadPinned?.().then(hit => {
      if (!cancelled && hit) pinnedHit = hit
    })
    return () => { cancelled = true }
  })

  $effect(() => {
    if (searchTimeout) clearTimeout(searchTimeout)
    offset = 0
    hasMore = true
    const delay = untrack(() => hits.length === 0 ? 0 : 300)
    searchTimeout = setTimeout(() => {
      fetchHits(0, true)
    }, delay)
    return () => { clearTimeout(searchTimeout) }
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

  const displayedHits = $derived.by(() => {
    if (pinnedHit && !searchQuery.trim() && hits.length > 0 && !hits.some(h => h.id === pinnedHit!.id)) {
      return [pinnedHit, ...hits]
    }
    return hits
  })

  async function fetchHits(offsetVal: number, replace: boolean) {
    const query = searchQuery.trim()
    if (replace) isSearching = true
    else isLoadingMore = true
    try {
      const result = await adapter.search(query, offsetVal, ITEMS_PER_PAGE)
      offset = offsetVal + result.hits.length
      hasMore = offsetVal + result.hits.length < result.total
      if (replace) {
        hits = result.hits
        selectedId = null
      } else {
        const ids = new Set(hits.map(h => h.id))
        hits = [...hits, ...result.hits.filter(h => !ids.has(h.id))]
      }
    } catch (error) {
      console.error("Addon search error:", error)
    } finally {
      if (replace) isSearching = false
      else isLoadingMore = false
    }
  }

  function loadMore() {
    if (!hasMore || isLoadingMore || isSearching) return
    fetchHits(offset, false)
  }

  function openDetail(hit: AddonHit) {
    selectedId = hit.id
    onViewProjectDetail(source, hit.id, hit.slug, category.projectType, hit.author)
  }
</script>

{#if category.requiresInstance && !selectedInstance}
  {@const EmptyIcon = category.fallbackIcon}
  <div class="max-w-7xl mx-auto">
    <div class="flex items-center justify-center py-20">
      <div class="text-center">
        <EmptyIcon size={64} class="mx-auto mb-4 text-[var(--text-muted)]" strokeWidth={1.5} />
        <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-2">No instance selected</h3>
        <p class="text-sm text-[var(--text-muted)]">{category.noInstanceHint}</p>
      </div>
    </div>
  </div>
{:else}
  <div class="max-w-7xl mx-auto h-full flex flex-col">
    <div class="flex-1 min-h-0">
      <div class="space-y-3 overflow-y-auto pr-2">
        {#each displayedHits as hit (hit.id)}
          {@const selected = selectedId === hit.id}
          <div
            class="rounded-md overflow-hidden cursor-pointer transition-all {selected ? 'bg-[var(--bg-elevated)]' : 'bg-[var(--bg-tertiary)]'}"
            role="button"
            tabindex="0"
            onkeydown={(e) => { if (e.key === 'Enter') openDetail(hit); }}
            onclick={() => openDetail(hit)}
          >
            <div class="flex min-h-0 relative z-0">
              {#if hit.imageUrl}
                <div class="w-24 h-24 flex items-center justify-center flex-shrink-0 rounded m-2">
                  <img src={hit.imageUrl} alt={hit.title} class="w-full h-full object-contain rounded" />
                </div>
              {:else}
                {@const FallbackIcon = category.fallbackIcon}
                <div
                  class="w-24 h-24 bg-gradient-to-br flex items-center justify-center flex-shrink-0 rounded m-2"
                  style={`background-image: linear-gradient(to bottom right, ${category.accentMain}1a, ${category.accentHover}1a)`}
                >
                  <FallbackIcon size={48} style={`color: ${category.accentMain}`} />
                </div>
              {/if}
              <div class="flex-1 min-w-0 py-2 px-3 flex items-center gap-3">
                <div class="flex-1 min-w-0">
                  <div class="flex items-start justify-between gap-2 mb-0">
                    <h3 class="font-semibold text-base text-[var(--text-primary)] truncate">{hit.title}</h3>
                    <span class="text-xs text-[var(--text-muted)] whitespace-nowrap">by {hit.author}</span>
                  </div>
                  <p class="text-sm text-[var(--text-muted)] line-clamp-2 mb-2">{hit.summary}</p>
                  <div class="flex items-center gap-2 text-xs flex-wrap">
                    <span class="flex items-center gap-1 bg-[var(--bg-secondary)] px-2 py-1 rounded text-[var(--text-muted)]">
                      <Download size={12} />
                      {formatDownloads(hit.downloads)}
                    </span>
                    {#each hit.categories.slice(0, 2) as cat}
                      <span class="bg-[var(--bg-secondary)] px-2 py-1 rounded text-[var(--text-muted)]">{cat}</span>
                    {/each}
                  </div>
                </div>
              </div>
            </div>
          </div>
        {/each}

        <div bind:this={sentinelEl} class="flex items-center justify-center py-4">
          {#if isLoadingMore}
            <Loader2 size={20} class="animate-spin" style={`color: ${category.accentMain}`} />
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}
