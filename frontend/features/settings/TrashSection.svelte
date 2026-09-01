<script lang="ts">
  import { invoke } from "@tauri-apps/api/core"
  import { Loader2, Trash2, Package, Image } from "lucide-svelte"
  import type { TrashItem } from "../../types"

  let { onAlert }: { onAlert: (alert: any) => void } = $props()

  let items = $state<TrashItem[]>([])
  let loading = $state(false)
  let emptying = $state(false)
  let confirmClear = $state(false)

  let count = $derived(items.length)
  let totalSize = $derived(items.reduce((sum, item) => sum + item.size, 0))

  async function loadTrash() {
    loading = true
    try {
      items = await invoke<TrashItem[]>("get_trash_items")
    } catch {
      items = []
    }
    loading = false
  }

  $effect(() => { loadTrash() })

  async function handleEmptyTrash() {
    if (!confirmClear) {
      confirmClear = true
      setTimeout(() => confirmClear = false, 3000)
      return
    }
    emptying = true
    try {
      await invoke("empty_trash")
      items = []
      confirmClear = false
    } catch (e) {
      onAlert({ isOpen: true, title: "Error", message: `Failed to empty trash: ${e}`, type: "danger" })
    }
    emptying = false
  }

  function formatBytes(bytes: number) {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`
  }

  function formatDate(rfc3339: string) {
    const d = new Date(rfc3339)
    return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" })
  }
</script>

<div class="space-y-3">
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-2 text-[var(--text-primary)]">
      <Trash2 size={16} class="text-red-400" />
      <span class="font-medium text-sm">Trash</span>
      {#if !loading && count > 0}
        <span class="text-xs text-[var(--text-muted)]">
          {count} item{count !== 1 ? "s" : ""} ({formatBytes(totalSize)})
        </span>
      {/if}
    </div>
    {#if !loading && count > 0}
      <button
        onclick={handleEmptyTrash}
        disabled={emptying}
        class="px-2.5 py-1 rounded text-xs font-medium transition-colors cursor-pointer disabled:opacity-50 {confirmClear ? 'bg-red-500 text-white hover:bg-red-600' : 'bg-[var(--bg-hover)] text-[var(--text-muted)] hover:text-red-400 hover:bg-red-500/10'}"
      >
        {#if emptying}
          <Loader2 size={12} class="animate-spin" />
        {:else if confirmClear}
          Click again to confirm
        {:else}
          Empty Trash
        {/if}
      </button>
    {/if}
  </div>
  <div class="space-y-1.5">
    {#if loading}
      <div class="flex items-center gap-2 text-xs text-[var(--text-muted)]">
        <Loader2 size={14} class="animate-spin" />
        <span>Loading trash...</span>
      </div>
    {:else if count === 0}
      <p class="text-xs text-[var(--text-muted)]">Trash is empty</p>
    {:else}
      <div class="space-y-1 max-h-48 overflow-y-auto">
        {#each items as item (item.id)}
          <div class="flex items-center gap-2 px-1.5 py-1 rounded bg-[var(--bg-secondary)] text-xs">
            {#if item.original_type === "screenshot"}
              <Image size={14} class="text-[var(--text-muted)] shrink-0" />
            {:else}
              <Package size={14} class="text-[var(--text-muted)] shrink-0" />
            {/if}
            <span class="text-[var(--text-primary)] truncate flex-1" title={item.original_name}>{item.original_name}</span>
            <span class="text-[var(--text-muted)] shrink-0">{formatDate(item.moved_at)}</span>
            <span class="text-[var(--text-muted)] shrink-0 w-14 text-right">{formatBytes(item.size)}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
