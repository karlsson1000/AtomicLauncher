<script lang="ts">
  import { Package, ExternalLink, FolderOpen, Copy, FileArchive, Trash2, Play, ChevronRight } from "lucide-svelte"
  import { invoke } from "@tauri-apps/api/core"
  import type { Instance, Snapshot, SnapshotsResponse } from "../../types"
  import { getMinecraftVersion } from "../../lib/version"
  import { formatPlaytime, cleanVersionName } from "../../lib/format"
  import ContextMenu from "../../components/ui/ContextMenu.svelte"
  import {
    store,
    handleLaunch, handleDeleteInstance, handleShowDetails,
    handleOpenInstanceFolderByInstance, handleDuplicateInstance,
    handleKillInstance, handleNavigateToInstances,
    setExportModal,
  } from "../../lib/launcherStore.svelte"

  let contextMenu = $state<{ x: number; y: number; instance: Instance } | null>(null)
  let snapshots = $state<Snapshot[]>([])
  let loadingSnapshots = $state(true)
  let instanceIcons = $state<Record<string, string | null>>({})
  let tooltipInstance = $state<Instance | null>(null)
  let tooltipTimer: ReturnType<typeof setTimeout> | undefined

  let lastPlayedInstance = $derived<Instance | null>(
    store.instances
      .filter(i => i.last_played)
      .sort((a, b) => new Date(b.last_played!).getTime() - new Date(a.last_played!).getTime())[0] ?? null
  )

  let recentInstances = $derived<Instance[]>(
    [...store.instances]
      .filter(i => i.last_played)
      .sort((a, b) => new Date(b.last_played!).getTime() - new Date(a.last_played!).getTime())
      .slice(0, 5)
  )

  let isLastPlayedRunning = $derived(lastPlayedInstance ? store.runningInstances.has(lastPlayedInstance.name) : false)
  let isLastPlayedLaunching = $derived(lastPlayedInstance ? store.launchingInstanceName === lastPlayedInstance.name : false)

  let heroBtnClass = $derived(
    isLastPlayedRunning || isLastPlayedLaunching
      ? "bg-red-500/15 text-red-400 hover:bg-red-500/25"
      : "bg-[#16a34a] text-[#181a1f] hover:bg-[#15803d]"
  )

  let heroDisabled = $derived(store.launchingInstanceName !== null && !isLastPlayedRunning && !isLastPlayedLaunching)

  $effect(() => {
    if (recentInstances.length === 0) return
    const loadIcons = async () => {
      const icons: Record<string, string | null> = {}
      await Promise.all(recentInstances.map(async (instance) => {
        try {
          icons[instance.name] = await invoke<string | null>("get_instance_icon", { instanceName: instance.name })
        } catch { icons[instance.name] = null }
      }))
      instanceIcons = icons
    }
    loadIcons()
  })

  $effect(() => {
    const loadSnapshots = async () => {
      try {
        const response = await fetch('https://launchercontent.mojang.com/v2/javaPatchNotes.json')
        const data: SnapshotsResponse = await response.json()
        snapshots = data.entries.slice(0, 3)
      } catch (error) {
        console.error('Failed to load snapshots:', error)
      } finally {
        loadingSnapshots = false
      }
    }
    loadSnapshots()
  })

  function handleContextMenu(e: MouseEvent, instance: Instance) {
    e.preventDefault()
    contextMenu = { x: e.clientX, y: e.clientY, instance }
  }

  function formatDate(dateString: string): string {
    const date = new Date(dateString)
    const now = new Date()
    const dateOnly = new Date(date.getFullYear(), date.getMonth(), date.getDate())
    const todayOnly = new Date(now.getFullYear(), now.getMonth(), now.getDate())
    const diffDays = Math.floor((todayOnly.getTime() - dateOnly.getTime()) / (1000 * 60 * 60 * 24))
    if (diffDays === 0) return "Today"
    if (diffDays === 1) return "Yesterday"
    if (diffDays < 7) return `${diffDays} days ago`
    if (diffDays < 30) return Math.floor(diffDays / 7) === 1 ? "1 week ago" : `${Math.floor(diffDays / 7)} weeks ago`
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
  }

  function getVersionUrl(version: string): string {
    const cleanVersion = cleanVersionName(version)
    return `https://www.minecraft.net/en-us/article/minecraft-${cleanVersion.toLowerCase().replace('.', '-')}`
  }

  function getGreeting(): string {
    const h = new Date().getHours()
    if (h < 12) return "Good morning"
    if (h < 18) return "Good afternoon"
    return "Good evening"
  }
</script>

<div class="p-12 pt-[75px] space-y-10">
  <div class="max-w-7xl mx-auto">
    <div class="w-full h-36 bg-[var(--bg-tertiary)] rounded-md relative flex items-center px-8">
      {#if store.activeAccount}
        <div class="absolute left-12 bottom-0" style="z-index: 10">
          <img
            src="https://renders.stellarmc.gg/bust/{store.activeAccount.username}"
            alt={store.activeAccount.username}
            class="h-48 w-auto object-contain"
            style="image-rendering: pixelated; display: block"
          />
        </div>
      {/if}

      <div class="{store.activeAccount ? 'w-44' : 'w-0'} flex-shrink-0"></div>

      <div class="flex-1 flex justify-center">
        {#if store.isAuthenticated && store.activeAccount}
          <h2 class="text-3xl font-semibold text-[var(--text-primary)] tracking-tight">
            {getGreeting()}, {store.activeAccount.username}
          </h2>
        {:else}
          <div class="text-center">
            <h2 class="text-3xl font-semibold text-[var(--text-primary)] tracking-tight">
              Welcome to Octane Launcher
            </h2>
            <p class="text-sm text-[var(--text-secondary)] mt-1">Sign in to get started</p>
          </div>
        {/if}
      </div>

      <div class="{store.isAuthenticated && lastPlayedInstance ? 'w-auto' : (store.activeAccount ? 'w-44' : 'w-0')} flex-shrink-0 flex items-center justify-end">
        {#if store.isAuthenticated && lastPlayedInstance}
          <button
            onclick={() => isLastPlayedRunning && handleKillInstance
              ? handleKillInstance(lastPlayedInstance)
              : handleLaunch(lastPlayedInstance)
            }
            disabled={heroDisabled}
            class="h-11 px-5 rounded-md flex items-center justify-center gap-3 text-lg font-semibold transition-all cursor-pointer disabled:opacity-40 {heroBtnClass}"
            title={isLastPlayedRunning ? "Stop" : `Play ${lastPlayedInstance.name}`}
          >
            {#if isLastPlayedLaunching || isLastPlayedRunning}
              <div class="w-5 h-5 border-2 border-red-400/30 border-t-red-400 rounded-full animate-spin"></div>
            {:else}
              <Play size={24} fill="currentColor" strokeWidth={0} />
            {/if}
            <span class="truncate max-w-[140px]">{isLastPlayedRunning ? "Stop" : isLastPlayedLaunching ? "Launching..." : lastPlayedInstance.name}</span>
          </button>
        {/if}
      </div>
    </div>
  </div>

  {#if recentInstances.length > 0}
    <div class="max-w-7xl mx-auto">
      <div class="mb-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <button onclick={handleNavigateToInstances} class="flex items-center gap-2 cursor-pointer group">
              <h2 class="text-xl font-semibold text-[var(--text-primary)] tracking-tight group-hover:text-[var(--text-primary)] transition-colors">
                Recently Played
              </h2>
              <ChevronRight size={18} strokeWidth={3} class="text-[var(--text-muted)] group-hover:text-[var(--text-primary)] transition-colors" />
            </button>
          </div>
        </div>
      </div>
      <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3">
        {#each recentInstances as instance (instance.name)}
          {@const isRunning = store.runningInstances.has(instance.name)}
          {@const isLaunching = store.launchingInstanceName === instance.name}
          {@const icon = instanceIcons[instance.name]}
          <div
            role="button"
            tabindex="0"
            onclick={() => handleShowDetails(instance)}
            oncontextmenu={(e) => handleContextMenu(e, instance)}
            onkeydown={(e) => { if (e.key === 'Enter') handleShowDetails(instance) }}
            onmouseenter={() => {
              tooltipTimer = setTimeout(() => tooltipInstance = instance, 1000)
            }}
            onmouseleave={() => {
              if (tooltipTimer) clearTimeout(tooltipTimer)
              tooltipInstance = null
            }}
            class="bg-[var(--bg-tertiary)] rounded-md p-2 flex items-center gap-2 hover:bg-[var(--bg-hover)] transition-all cursor-pointer group relative"
          >
            {#if tooltipInstance === instance}
              <div class="absolute bottom-full left-0 right-0 mb-2 z-50 pointer-events-none">
                <div class="bg-[var(--bg-secondary)] rounded-md p-3 mx-2 border border-[var(--border-default)]">
                  <div class="text-sm font-medium text-[var(--text-primary)] mb-1.5">{instance.name}</div>
                  <div class="space-y-1 text-xs text-[var(--text-muted)]">
                    <div class="flex justify-between gap-4">
                      <span>Version</span>
                      <span class="text-[var(--text-primary)]">{getMinecraftVersion(instance)}</span>
                    </div>
                    {#if instance.loader}
                      <div class="flex justify-between gap-4">
                        <span>Loader</span>
                        <span class="text-[var(--text-primary)] capitalize">{instance.loader}{instance.loader_version ? ` ${instance.loader_version}` : ''}</span>
                      </div>
                    {/if}
                    <div class="flex justify-between gap-4">
                      <span>Created</span>
                      <span class="text-[var(--text-primary)]">{formatDate(instance.created_at)}</span>
                    </div>
                    {#if instance.total_playtime_seconds !== undefined && instance.total_playtime_seconds > 0}
                      <div class="flex justify-between gap-4">
                        <span>Playtime</span>
                        <span class="text-[var(--text-primary)]">{formatPlaytime(instance.total_playtime_seconds)}</span>
                      </div>
                    {/if}
                  </div>
                </div>
              </div>
            {/if}
            <div class="relative flex-shrink-0">
              {#if icon}
                <img src={icon} alt={instance.name} class="w-10 h-10 rounded object-cover" />
              {:else}
                <div class="w-10 h-10 bg-[var(--bg-secondary)] rounded flex items-center justify-center">
                  <Package size={20} class="text-[var(--text-muted)]" />
                </div>
              {/if}
            </div>
            <div class="flex-1 min-w-0 {(isRunning || isLaunching) ? 'pr-12' : 'group-hover:pr-12'}">
              <div class="text-sm font-medium text-[var(--text-primary)] truncate leading-tight">{instance.name}</div>
              <div class="text-xs text-[var(--text-muted)] truncate leading-tight mt-0.5">
                {instance.last_played && formatDate(instance.last_played)}
              </div>
            </div>
            <button
              onclick={(e) => { e.stopPropagation();
                if (isRunning) handleKillInstance(instance)
                else handleLaunch(instance)
              }}
              disabled={store.launchingInstanceName !== null && !isLaunching && !isRunning}
              class="absolute right-2 flex-shrink-0 w-10 h-10 flex items-center justify-center rounded transition-all {(isRunning || isLaunching) ? 'bg-red-500/10 text-red-400 opacity-100 hover:bg-red-500/20 cursor-pointer' : store.launchingInstanceName !== null ? 'opacity-0 pointer-events-none' : 'opacity-0 group-hover:opacity-100 bg-[#16a34a] hover:bg-[#15803d] text-[#181a1f] cursor-pointer'}"
              title={isRunning ? "Stop instance" : "Launch instance"}
            >
              {#if isLaunching || isRunning}
                <div class="w-4 h-4 border-2 border-red-400/30 border-t-red-400 rounded-full animate-spin"></div>
              {:else}
                <Play size={20} fill="currentColor" strokeWidth={0} />
              {/if}
            </button>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <div class="max-w-7xl mx-auto">
    <div class="mb-4">
      <h2 class="text-xl font-semibold text-[var(--text-primary)] tracking-tight">Latest Snapshots</h2>
    </div>
    {#if loadingSnapshots}
      <div class="flex items-center justify-center py-12">
        <div class="w-8 h-8 rounded-full animate-spin"></div>
      </div>
    {:else if snapshots.length === 0}
      <div class="bg-[var(--bg-tertiary)] rounded-md p-8 text-center">
        <p class="text-[var(--text-muted)]">Unable to load snapshots</p>
      </div>
    {:else}
      <div class="grid grid-cols-3 gap-4">
        {#each snapshots as snapshot (snapshot.id)}
          <div
            role="button"
            tabindex="0"
            onclick={() => { invoke('open_url', { url: getVersionUrl(snapshot.version) }).catch(() => {}) }}
            onkeydown={(e) => { if (e.key === 'Enter') invoke('open_url', { url: getVersionUrl(snapshot.version) }).catch(() => {}) }}
            class="bg-[var(--bg-tertiary)] rounded-md overflow-hidden relative group cursor-pointer transition-all flex flex-col"
          >
            <div class="absolute top-2 right-2 z-10 w-7 h-7 flex items-center justify-center rounded opacity-0 group-hover:opacity-100 transition-opacity">
              <ExternalLink size={14} class="text-[var(--text-primary)]" />
            </div>
            <div class="h-40 bg-[var(--bg-secondary)] overflow-hidden relative flex-shrink-0 z-0">
              {#if snapshot.image?.url}
                <img src="https://launchercontent.mojang.com{snapshot.image.url}" alt={snapshot.title} class="w-full h-full object-cover" />
              {:else}
                <div class="w-full h-full flex items-center justify-center">
                  <Package size={48} class="text-[var(--text-muted)]" />
                </div>
              {/if}
              <div class="absolute inset-0 bg-gradient-to-t from-black/40 via-black/10 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300"></div>
            </div>
            <div class="p-4 flex-1 flex flex-col relative z-0">
              <div class="flex items-center justify-between gap-2 mb-2">
                <h3 class="text-sm font-semibold text-[var(--text-primary)] truncate">{cleanVersionName(snapshot.version)}</h3>
                {#if snapshot.date}
                  <span class="text-xs text-[var(--text-muted)] whitespace-nowrap">{formatDate(snapshot.date)}</span>
                {/if}
              </div>
              {#if snapshot.shortText}
                <p class="text-xs text-[var(--text-muted)] line-clamp-2 leading-snug">{snapshot.shortText}</p>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  {#if contextMenu}
    {@const cm = contextMenu}
    <ContextMenu
      x={cm.x}
      y={cm.y}
      onClose={() => contextMenu = null}
      items={[
        { label: "Open", icon: Package, onClick: () => handleShowDetails(cm.instance) },
        { label: "Open Folder", icon: FolderOpen, onClick: () => handleOpenInstanceFolderByInstance(cm.instance) },
        { label: "Duplicate", icon: Copy, onClick: () => handleDuplicateInstance(cm.instance) },
        { label: "Export", icon: FileArchive, onClick: () => { setExportModal(cm.instance); contextMenu = null } },
        { separator: true },
        { label: "Delete", icon: Trash2, onClick: () => handleDeleteInstance(cm.instance.name), danger: true },
      ]}
    />
  {/if}
</div>
