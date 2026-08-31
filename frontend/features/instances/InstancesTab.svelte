<script lang="ts">
  import { Package, FolderOpen, Copy, FileArchive, FolderSymlink, FolderX, Trash2, Play, Search, Plus, FolderPlus, ChevronDown, ChevronUp, Save, ClipboardCheck } from "lucide-svelte"
  import { invoke } from "@tauri-apps/api/core"
import { instanceIconSrc } from "../../lib/icons"
  import type { Instance } from "../../types"
  import { getMinecraftVersion } from "../../lib/version"
  import ContextMenu from "../../components/ui/ContextMenu.svelte"

  import {
    store, setSelectedInstance, handleLaunch, handleCreateNew,
    handleShowDetails, handleOpenInstanceFolderByInstance,
    handleDuplicateInstance, handleDeleteInstance, handleKillInstance,
    setExportModal,
  } from "../../lib/launcherStore.svelte"
  import { storeGet, storeSet } from "../../lib/store"
  import { showToast } from "../../lib/toastStore.svelte"

  async function handleSaveOptionsAsDefault(name: string) {
    try {
      await invoke("save_options_as_default", { instanceName: name })
      showToast("success", "Options saved", 5000)
    } catch (error) {
      showToast("error", `Failed to save options: ${error}`, 5000)
    }
  }

  async function handleApplySavedOptions(name: string) {
    try {
      await invoke("apply_saved_options", { instanceName: name })
      showToast("success", "Saved options applied", 5000)
    } catch (error) {
      showToast("error", `Failed to apply options: ${error}`, 5000)
    }
  }

  type SortOption = "recently-played" | "name-asc" | "name-desc"

  const SORT_CYCLE: SortOption[] = ["recently-played", "name-asc", "name-desc"]

  const SORT_LABELS: Record<SortOption, string> = {
    "recently-played": "Recently played",
    "name-asc": "Name (A–Z)",
    "name-desc": "Name (Z–A)",
  }

  const DEFAULT_GROUP = "__ungrouped__"

  let contextMenu = $state<{ x: number; y: number; instance: Instance } | null>(null)
  let groupContextMenu = $state<{ x: number; y: number; group: string } | null>(null)
  let groups = $state<Record<string, string[]>>({})
  let searchQuery = $state("")
  let showGroupModal = $state(false)
  let sortBy = $state<SortOption>("recently-played")
  let collapsed = $state<Record<string, boolean>>({})
  let groupModalInstance = $state<Instance | null>(null)
  let groupModalValue = $state("")
  let groupModalInputEl: HTMLInputElement | undefined = $state()

  $effect(() => {
    storeGet<Record<string, string[]>>("instance_groups").then(g => { if (g) groups = g })
    storeGet<Record<string, boolean>>("group_collapsed").then(c => { if (c) collapsed = c })
  })

  $effect(() => {
    if (showGroupModal && groupModalInputEl) {
      const t = setTimeout(() => groupModalInputEl?.focus(), 50)
      return () => clearTimeout(t)
    }
  })

  $effect(() => {
    const nameSet = new Set(store.instances.map(i => i.name))
    let changed = false
    const next: Record<string, string[]> = {}
    for (const [group, names] of Object.entries(groups)) {
      const filtered = names.filter(n => nameSet.has(n))
      if (filtered.length !== names.length) changed = true
      if (filtered.length > 0) next[group] = filtered
    }
    if (changed) persistGroups(next)
  })

  async function persistGroups(next: Record<string, string[]>) {
    groups = next
    await storeSet("instance_groups", next)
  }

  async function persistCollapsed(next: Record<string, boolean>) {
    collapsed = next
    await storeSet("group_collapsed", next)
  }

  function handleCycleSort() {
    const currentIndex = SORT_CYCLE.indexOf(sortBy)
    sortBy = SORT_CYCLE[(currentIndex + 1) % SORT_CYCLE.length]
  }

  async function handleCreateGroup(instance: Instance, name: string) {
    const trimmed = name.trim()
    if (!trimmed || trimmed === DEFAULT_GROUP) return
    const next = { ...groups }
    for (const key of Object.keys(next)) {
      next[key] = next[key].filter(n => n !== instance.name)
    }
    next[trimmed] = [...(next[trimmed] ?? []), instance.name]
    await persistGroups(next)
  }

  async function handleMoveToGroup(instance: Instance, group: string) {
    const next = { ...groups }
    for (const key of Object.keys(next)) {
      next[key] = next[key].filter(n => n !== instance.name)
    }
    next[group] = [...(next[group] ?? []), instance.name]
    await persistGroups(next)
  }

  async function handleRemoveFromGroup(instance: Instance) {
    const next = { ...groups }
    for (const key of Object.keys(next)) {
      next[key] = next[key].filter(n => n !== instance.name)
    }
    await persistGroups(next)
  }

  async function handleDeleteGroup(group: string) {
    const next = { ...groups }
    delete next[group]
    await persistGroups(next)
    groupContextMenu = null
  }

  async function toggleCollapsed(group: string) {
    await persistCollapsed({ ...collapsed, [group]: !collapsed[group] })
  }

  function handleQuickLaunch(instance: Instance) {
    setSelectedInstance(instance)
    handleLaunch(instance)
  }

  function handleContextMenu(e: MouseEvent, instance: Instance) {
    e.preventDefault()
    contextMenu = { x: e.clientX, y: e.clientY, instance }
  }

  function handleGroupHeaderContextMenu(e: MouseEvent, group: string) {
    e.preventDefault()
    groupContextMenu = { x: e.clientX, y: e.clientY, group }
  }

  function sortInstances(list: Instance[]) {
    return [...list].sort((a, b) => {
      switch (sortBy) {
        case "name-asc": return a.name.localeCompare(b.name)
        case "name-desc": return b.name.localeCompare(a.name)
        case "recently-played":
        default: {
          const aTime = a.last_played ? new Date(a.last_played).getTime() : 0
          const bTime = b.last_played ? new Date(b.last_played).getTime() : 0
          return bTime - aTime
        }
      }
    })
  }

  let filteredInstances = $derived(
    store.instances.filter(i => i.name.toLowerCase().includes(searchQuery.toLowerCase()))
  )

  let instanceToGroup = $derived.by(() => {
    const map = new Map<string, string>()
    for (const [group, names] of Object.entries(groups)) {
      for (const name of names) {
        map.set(name, group)
      }
    }
    return map
  })

  let namedGroups = $derived(Object.keys(groups))
  let ungrouped = $derived(filteredInstances.filter(i => (instanceToGroup.get(i.name) ?? DEFAULT_GROUP) === DEFAULT_GROUP))
  let hasAnyGroups = $derived(namedGroups.length > 0)

  let visibleNamedGroups = $derived(
    namedGroups.map(group => ({
      name: group,
        instances: sortInstances(filteredInstances.filter(i => instanceToGroup.get(i.name) === group)),
    }))
  )

  let sortIsDesc = $derived(sortBy === "recently-played" || sortBy === "name-desc")

  let contextMenuInstanceGroup = $derived(
    contextMenu ? (instanceToGroup.get(contextMenu.instance.name) ?? DEFAULT_GROUP) : null
  )

  let contextMenuItems = $derived.by(() => {
    const cm = contextMenu
    if (!cm) return []
    const items: Array<{
      label?: string
      icon?: any
      onClick?: () => void
      danger?: boolean
      separator?: boolean
    }> = [
      { label: "Open", icon: Package, onClick: () => { setSelectedInstance(cm.instance); handleShowDetails(cm.instance) } },
      { label: "Open Folder", icon: FolderOpen, onClick: () => handleOpenInstanceFolderByInstance(cm.instance) },
      { label: "Duplicate", icon: Copy, onClick: () => handleDuplicateInstance(cm.instance) },
      { label: "Export", icon: FileArchive, onClick: () => { setExportModal(cm.instance); contextMenu = null } },
      { separator: true },
      { label: "Save Instance Options", icon: Save, onClick: () => { handleSaveOptionsAsDefault(cm.instance.name); contextMenu = null } },
      { label: "Apply Saved Options", icon: ClipboardCheck, onClick: () => { handleApplySavedOptions(cm.instance.name); contextMenu = null } },
      { separator: true },
      { label: "Add to group", icon: FolderPlus, onClick: () => { groupModalInstance = cm.instance; groupModalValue = ""; showGroupModal = true; contextMenu = null } },
    ]
    if (contextMenuInstanceGroup !== DEFAULT_GROUP) {
      items.push({ label: "Remove from group", icon: FolderX, onClick: () => { handleRemoveFromGroup(cm.instance); contextMenu = null }, danger: true })
    }
    const otherGroups = namedGroups.filter(g => g !== contextMenuInstanceGroup)
    if (otherGroups.length > 0) {
      items.push({ separator: true })
      for (const g of otherGroups) {
        items.push({ label: `Move to "${g}"`, icon: FolderSymlink, onClick: () => { handleMoveToGroup(cm.instance, g); contextMenu = null } })
      }
    }
    items.push({ separator: true })
    items.push({ label: "Delete", icon: Trash2, onClick: () => handleDeleteInstance(cm.instance.name), danger: true })
    return items
  })
</script>

<div class="p-8 space-y-4">
  <div class="max-w-7xl mx-auto">
    <div class="flex items-center justify-between mb-4">
      <div class="flex items-center gap-2">
        {#if store.instances.length > 0}
          <div class="relative rounded-md bg-[var(--bg-tertiary)]">
            <Search size={16} class="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--text-muted)] z-20 pointer-events-none" strokeWidth={2} />
            <input
              type="text"
              placeholder="Search instances..."
              bind:value={searchQuery}
              class="w-72 bg-transparent rounded-md pl-9 pr-3 py-1.5 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none transition-all relative z-10"
            />
          </div>
        {/if}
      </div>
      <div class="flex items-center gap-2">
        {#if store.instances.length > 0}
          <button
            onclick={handleCycleSort}
            class="h-8 px-2.5 hover:bg-[var(--bg-tertiary)] rounded flex items-center gap-1.5 transition-colors cursor-pointer group"
          >
            <span class="text-sm text-[var(--text-muted)] group-hover:text-[var(--text-muted)] transition-colors font-medium">Sort by:</span>
            <span class="text-sm text-[var(--text-muted)] group-hover:text-[var(--text-primary)] transition-colors font-semibold">{SORT_LABELS[sortBy]}</span>
            {#if sortIsDesc}
              <ChevronDown size={14} class="text-[var(--text-muted)] group-hover:text-[var(--text-primary)] transition-colors" strokeWidth={2.5} />
            {:else}
              <ChevronUp size={14} class="text-[var(--text-muted)] group-hover:text-[var(--text-primary)] transition-colors" strokeWidth={2.5} />
            {/if}
          </button>
        {/if}
        <button
          onclick={handleCreateNew}
          class="px-4 h-8 bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white rounded-md text-sm font-medium flex items-center gap-2 transition-colors cursor-pointer"
        >
          <Plus size={16} />
          New
        </button>
      </div>
    </div>

    {#if store.instances.length === 0}
      <div class="flex flex-col items-center justify-center py-14">
        <Package size={48} class="text-[var(--text-muted)] mb-3" strokeWidth={1.5} />
        <h3 class="text-base font-semibold text-[var(--text-primary)] mb-1">No instances yet</h3>
        <p class="text-sm text-[var(--text-muted)] mb-4">Create your first instance to get started</p>
        <button
          onclick={handleCreateNew}
          class="px-4 py-2 bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white rounded font-medium text-sm flex items-center gap-2 transition-all cursor-pointer"
        >
          <Plus size={16} strokeWidth={2} />
          <span>Create Instance</span>
        </button>
      </div>
    {:else if filteredInstances.length === 0}
      <div class="rounded-md p-8 flex flex-col items-center justify-center">
        <Search size={48} class="text-[var(--text-primary)] mb-3" strokeWidth={1.5} />
        <h3 class="text-base font-semibold text-[var(--text-primary)] mb-1">No instances found</h3>
        <p class="text-sm text-[var(--text-muted)]">Try adjusting your search query</p>
      </div>
    {:else}
      <div class="space-y-5">
        {#each visibleNamedGroups as group (group.name)}
          <div>
            <div
              role="presentation"
              class="flex items-center gap-1.5 mb-2 select-none"
              oncontextmenu={(e) => handleGroupHeaderContextMenu(e, group.name)}
            >
              <ChevronDown
                size={16}
                strokeWidth={3}
                class="text-[var(--text-muted)] flex-shrink-0 transition-transform duration-150 {collapsed[group.name] ? '-rotate-90' : ''}"
              />
              <span role="button" tabindex="0" class="text-sm font-semibold text-[var(--text-muted)] truncate cursor-pointer" onclick={() => toggleCollapsed(group.name)} onkeydown={(e) => { if (e.key === 'Enter') toggleCollapsed(group.name) }}>{group.name}</span>
            </div>
            {#if !collapsed[group.name]}
              <div class="grid grid-cols-2 gap-3">
                {#each group.instances as instance (instance.name)}
                  {@const isRunning = store.runningInstances.has(instance.name)}
                  {@const isLaunching = store.launchingInstanceName === instance.name}
                  <div
                    role="button"
                    tabindex="0"
                    onclick={() => { setSelectedInstance(instance); handleShowDetails(instance) }}
                    oncontextmenu={(e) => handleContextMenu(e, instance)}
                    onkeydown={(e) => { if (e.key === 'Enter') { setSelectedInstance(instance); handleShowDetails(instance) } }}
                    class="bg-[var(--bg-tertiary)] rounded-md flex items-center hover:bg-[var(--bg-hover)] transition-all cursor-pointer group relative overflow-hidden"
                  >
                    <div class="relative flex-shrink-0">
                      {#if instanceIconSrc(instance.icon_path)}
                        <img src={instanceIconSrc(instance.icon_path)} alt={instance.name} class="w-20 h-20 object-cover" />
                      {:else}
                        <div class="w-20 h-20 flex items-center justify-center">
                          <Package size={36} class="text-[var(--text-muted)]" />
                        </div>
                      {/if}
                    </div>
                    <div class="py-2 pr-2 pl-4 flex-1 min-w-0 {(isRunning || isLaunching) ? 'pr-12' : 'group-hover:pr-12'}">
                      <div class="text-[17px] font-semibold text-[var(--text-primary)] truncate leading-tight">{instance.name}</div>
                      <div class="flex items-center gap-2 text-[15px] text-[var(--text-muted)] mt-0.5">
                        <span>{getMinecraftVersion(instance)}</span>
                        <span class="text-[var(--text-muted)]">•</span>
                        {#if instance.loader === "fabric"}
                          <span class="text-[#3b82f6] flex-shrink-0 flex items-center gap-1">
                            <img src="/loaders/fabric.png" alt="Fabric" class="w-3.5 h-3.5" />
                            Fabric
                          </span>
                        {:else if instance.loader === "neoforge"}
                          <span class="text-[#f97316] flex-shrink-0 flex items-center gap-1">
                            <img src="/loaders/neoforge.png" alt="NeoForge" class="w-3.5 h-3.5" />
                            NeoForge
                          </span>
                        {:else if instance.loader === "forge"}
                          <span class="text-[#e05d2e] flex-shrink-0 flex items-center gap-1">
                            <img src="/loaders/forge.png" alt="Forge" class="w-3.5 h-3.5" />
                            Forge
                          </span>
                        {:else}
                          <span class="text-[#16a34a] flex-shrink-0">Vanilla</span>
                        {/if}
                      </div>
                    </div>

                    {#if store.isAuthenticated}
                        <button
                          title={isRunning ? "Stop instance" : "Launch instance"}
                          onclick={(e) => {
                            e.stopPropagation()
                            if (isRunning) handleKillInstance(instance)
                            else handleQuickLaunch(instance)
                          }}
                          disabled={store.launchingInstanceName !== null && !isRunning}
                          class="flex-shrink-0 w-12 h-12 mr-4 flex items-center justify-center rounded transition-all cursor-pointer {(isRunning || isLaunching) ? 'bg-red-500/10 text-red-400 hover:bg-red-500/20' : 'opacity-0 group-hover:opacity-100 bg-[#16a34a] hover:bg-[#15803d] text-[#181a1f]'} disabled:opacity-50"
                        >
                          {#if isLaunching || isRunning}
                            <div class="w-5 h-5 border-2 border-red-400/30 border-t-red-400 rounded-full animate-spin"></div>
                          {:else}
                            <Play size={24} fill="currentColor" strokeWidth={0} />
                          {/if}
                        </button>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/each}

        {#if ungrouped.length > 0}
          <div>
            {#if hasAnyGroups}
              <div class="flex items-center gap-1.5 mb-2 select-none">
                <ChevronDown
                  size={16}
                  strokeWidth={3}
                  class="text-[var(--text-muted)] flex-shrink-0 transition-transform duration-150 {collapsed[DEFAULT_GROUP] ? '-rotate-90' : ''}"
                />
                <span role="button" tabindex="0" class="text-sm font-semibold text-[var(--text-muted)] cursor-pointer" onclick={() => toggleCollapsed(DEFAULT_GROUP)} onkeydown={(e) => { if (e.key === 'Enter') toggleCollapsed(DEFAULT_GROUP) }}>Ungrouped</span>
              </div>
            {/if}
            {#if !collapsed[DEFAULT_GROUP]}
              <div class="grid grid-cols-2 gap-3">
                {#each sortInstances(ungrouped) as instance (instance.name)}
                  {@const isRunning = store.runningInstances.has(instance.name)}
                  {@const isLaunching = store.launchingInstanceName === instance.name}
                  <div
                    role="button"
                    tabindex="0"
                    onclick={() => { setSelectedInstance(instance); handleShowDetails(instance) }}
                    oncontextmenu={(e) => handleContextMenu(e, instance)}
                    onkeydown={(e) => { if (e.key === 'Enter') { setSelectedInstance(instance); handleShowDetails(instance) } }}
                    class="bg-[var(--bg-tertiary)] rounded-md flex items-center hover:bg-[var(--bg-hover)] transition-all cursor-pointer group relative overflow-hidden"
                  >
                    <div class="relative flex-shrink-0">
                      {#if instanceIconSrc(instance.icon_path)}
                        <img src={instanceIconSrc(instance.icon_path)} alt={instance.name} class="w-20 h-20 object-cover" />
                      {:else}
                        <div class="w-20 h-20 flex items-center justify-center">
                          <Package size={36} class="text-[var(--text-muted)]" />
                        </div>
                      {/if}
                    </div>
                    <div class="py-2 pr-2 pl-4 flex-1 min-w-0 {(isRunning || isLaunching) ? 'pr-12' : 'group-hover:pr-12'}">
                      <div class="text-[17px] font-semibold text-[var(--text-primary)] truncate leading-tight">{instance.name}</div>
                      <div class="flex items-center gap-2 text-[15px] text-[var(--text-muted)] mt-0.5">
                        <span>{getMinecraftVersion(instance)}</span>
                        <span class="text-[var(--text-muted)]">•</span>
                        {#if instance.loader === "fabric"}
                          <span class="text-[#3b82f6] flex-shrink-0 flex items-center gap-1">
                            <img src="/loaders/fabric.png" alt="Fabric" class="w-3.5 h-3.5" />
                            Fabric
                          </span>
                        {:else if instance.loader === "neoforge"}
                          <span class="text-[#f97316] flex-shrink-0 flex items-center gap-1">
                            <img src="/loaders/neoforge.png" alt="NeoForge" class="w-3.5 h-3.5" />
                            NeoForge
                          </span>
                        {:else if instance.loader === "forge"}
                          <span class="text-[#e05d2e] flex-shrink-0 flex items-center gap-1">
                            <img src="/loaders/forge.png" alt="Forge" class="w-3.5 h-3.5" />
                            Forge
                          </span>
                        {:else}
                          <span class="text-[#16a34a] flex-shrink-0">Vanilla</span>
                        {/if}
                      </div>
                    </div>

                    {#if store.isAuthenticated}
                        <button
                          title={isRunning ? "Stop instance" : "Launch instance"}
                          onclick={(e) => {
                            e.stopPropagation()
                            if (isRunning) handleKillInstance(instance)
                            else handleQuickLaunch(instance)
                          }}
                          disabled={store.launchingInstanceName !== null && !isRunning}
                          class="flex-shrink-0 w-12 h-12 mr-4 flex items-center justify-center rounded transition-all cursor-pointer {(isRunning || isLaunching) ? 'bg-red-500/10 text-red-400 hover:bg-red-500/20' : 'opacity-0 group-hover:opacity-100 bg-[#16a34a] hover:bg-[#15803d] text-[#181a1f]'} disabled:opacity-50"
                        >
                          {#if isLaunching || isRunning}
                            <div class="w-5 h-5 border-2 border-red-400/30 border-t-red-400 rounded-full animate-spin"></div>
                          {:else}
                            <Play size={24} fill="currentColor" strokeWidth={0} />
                          {/if}
                        </button>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

{#if contextMenu}
  <ContextMenu
    x={contextMenu.x}
    y={contextMenu.y}
    onClose={() => contextMenu = null}
    items={contextMenuItems}
  />
{/if}

{#if groupContextMenu}
  {@const gcm = groupContextMenu}
  <ContextMenu
    x={gcm.x}
    y={gcm.y}
    onClose={() => groupContextMenu = null}
    items={[
      { label: "Delete group", icon: Trash2, onClick: () => handleDeleteGroup(gcm.group), danger: true },
    ]}
  />
{/if}

{#if showGroupModal && groupModalInstance}
  {@const gmi = groupModalInstance}
  {@const otherGroups = namedGroups.filter(g => g !== instanceToGroup.get(gmi.name))}
  <div
    role="presentation"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm modal-backdrop"
    onclick={() => showGroupModal = false}
    onkeydown={(e) => { if (e.key === 'Escape') showGroupModal = false }}
  >
    <div
      role="presentation"
      class="blur-border bg-[var(--bg-secondary)] rounded-lg w-80 shadow-xl modal-content p-4"
      onclick={(e) => e.stopPropagation()}
    >
      <h3 class="text-base font-semibold text-[var(--text-primary)] tracking-tight">Add to group</h3>
      <p class="text-xs text-[var(--text-muted)] mt-0.5 mb-3">Move "{gmi.name}" into a group</p>

      {#if otherGroups.length > 0}
        <div class="flex flex-col gap-y-0.5 max-h-48 overflow-y-auto custom-scrollbar mb-1">
          {#each otherGroups as g (g)}
            <button
              onclick={() => { handleCreateGroup(gmi, g); showGroupModal = false }}
              class="flex items-center gap-2.5 px-2 py-2 text-sm text-[var(--text-primary)] hover:bg-[var(--bg-hover-light)] rounded-md transition-colors cursor-pointer text-left"
            >
              <FolderOpen size={16} strokeWidth={2} class="text-[var(--text-muted)] flex-shrink-0" />
              <span class="truncate">{g}</span>
            </button>
          {/each}
          <div class="h-px bg-[var(--border-subtle)] my-2"></div>
        </div>
      {/if}

      <div class="flex gap-2">
        <input
          bind:this={groupModalInputEl}
          type="text"
          placeholder="New group name"
          bind:value={groupModalValue}
          onkeydown={(e) => {
            if (e.key === "Enter" && groupModalValue.trim()) {
              handleCreateGroup(gmi, groupModalValue)
              showGroupModal = false
            }
            if (e.key === "Escape") showGroupModal = false
          }}
          class="flex-1 min-w-0 bg-[var(--bg-tertiary)] text-[var(--text-primary)] placeholder-[var(--text-muted)] rounded-md px-3 py-2 text-sm outline-none border border-transparent transition-colors"
        />
        <button
          onclick={() => {
            if (groupModalValue.trim()) {
              handleCreateGroup(gmi, groupModalValue)
              showGroupModal = false
            }
          }}
          disabled={!groupModalValue.trim()}
          class="px-3 py-1.5 text-sm bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white rounded-md transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
        >
          Create
        </button>
      </div>
    </div>
  </div>
{/if}
