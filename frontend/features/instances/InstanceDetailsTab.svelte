<script lang="ts">
  import { Play, FolderOpen, Package, Loader2, ExternalLink, Globe, Search, X, Image, Palette, Trash2, RefreshCw, Settings, Check } from "lucide-svelte"
  import { invoke } from "@tauri-apps/api/core"
  import ConfirmModal from "../../components/ui/ConfirmModal.svelte"
  import AlertModal from "../../components/ui/AlertModal.svelte"
  import InstanceSettingsModal from "./InstanceSettingsModal.svelte"
  import type { Instance, ModFileWithMetadata, ModrinthVersion, ModrinthFile } from "../../types"
  import { getMinecraftVersion } from "../../lib/version"
  import { formatFileSize, formatPlaytime, formatDate } from "../../lib/format"
  import {
    store, handleLaunch, handleWorldLaunch, handleCloseDetails,
    loadInstances
  } from "../../lib/launcherStore.svelte"

  type InstalledMod = ModFileWithMetadata

  interface ModUpdate {
    filename: string
    projectId: string
    currentVersionId: string
    latestVersion: {
      id: string
      name: string
      version_number: string
      downloadUrl: string
      filename: string
    }
  }

  interface World {
    name: string
    folder_name: string
    size: number
    last_played?: number
    game_mode?: string
    version?: string
    icon?: string
    created?: number
  }

  let { instance, onInstanceUpdated }: {
    instance: Instance
    onInstanceUpdated?: () => void
  } = $props()

  let installedMods = $state<InstalledMod[]>([])
  let worlds = $state<World[]>([])
  let isLoadingMods = $state(true)
  let isLoadingWorlds = $state(true)
  let instanceIcon = $state<string | null>(null)
  let isSettingsOpen = $state(false)
  let availableUpdates = $state<ModUpdate[]>([])
  let isCheckingUpdates = $state(false)
  let isUpdatingMods = $state(false)
  let modSearchQuery = $state("")
  let worldSearchQuery = $state("")
  let launchingWorld = $state<string | null>(null)
  let resourcePacks = $state<ModFileWithMetadata[]>([])
  let shaderPacks = $state<ModFileWithMetadata[]>([])
  let isLoadingResourcePacks = $state(true)
  let isLoadingShaderPacks = $state(true)
  let resourcePackSearchQuery = $state("")
  let shaderPackSearchQuery = $state("")
  let detailSection = $state<"mods" | "worlds" | "resourcepacks" | "shaderpacks">("mods")

  function tabClass(section: string) {
    return detailSection === section
      ? "bg-[var(--bg-tertiary)] text-[var(--text-primary)]"
      : "text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)]"
  }
  let confirmModal = $state<{
    isOpen: boolean
    title: string
    message: string
    type: "warning" | "danger" | "success" | "info"
    onConfirm: () => void
  } | null>(null)
  let alertModal = $state<{
    isOpen: boolean
    title: string
    message: string
    type: "warning" | "danger" | "success" | "info"
  } | null>(null)

  $effect(() => {
    void instance.name
    loadInstalledMods()
    loadWorlds()
    loadResourcePacks()
    loadShaderPacks()
    loadInstanceIcon()
  })

  async function loadInstanceIcon() {
    try {
      const icon = await invoke<string | null>("get_instance_icon", { instanceName: instance.name })
      instanceIcon = icon
    } catch (error) {
      console.error("Failed to load instance icon:", error)
      instanceIcon = null
    }
  }

  async function loadWorlds() {
    isLoadingWorlds = true
    try {
      const worldsList = await invoke<World[]>("get_instance_worlds", { instanceName: instance.name })
      worlds = worldsList
    } catch (error) {
      console.error("Failed to load worlds:", error)
      worlds = []
    } finally {
      isLoadingWorlds = false
    }
  }

  async function loadInstalledMods() {
    isLoadingMods = true
    try {
      const mods = await invoke<InstalledMod[]>("get_installed_mods_with_metadata", { instanceName: instance.name })
      installedMods = mods
    } catch (error) {
      console.error("Failed to load installed mods:", error)
      installedMods = []
    } finally {
      isLoadingMods = false
    }
  }

  async function loadResourcePacks() {
    isLoadingResourcePacks = true
    try {
      const packs = await invoke<ModFileWithMetadata[]>("get_installed_resourcepacks_with_metadata", { instanceName: instance.name })
      resourcePacks = packs
    } catch (error) {
      console.error("Failed to load resource packs:", error)
      resourcePacks = []
    } finally {
      isLoadingResourcePacks = false
    }
  }

  async function loadShaderPacks() {
    isLoadingShaderPacks = true
    try {
      const packs = await invoke<ModFileWithMetadata[]>("get_installed_shaderpacks_with_metadata", { instanceName: instance.name })
      shaderPacks = packs
    } catch (error) {
      console.error("Failed to load shader packs:", error)
      shaderPacks = []
    } finally {
      isLoadingShaderPacks = false
    }
  }

  async function checkForUpdates() {
    if (!instance || (instance.loader !== "fabric" && instance.loader !== "neoforge" && instance.loader !== "forge")) return

    isCheckingUpdates = true
    const updates: ModUpdate[] = []

    try {
      const mcVersion = getMinecraftVersion(instance)

      for (const mod of installedMods) {
        if (!mod.project_id || !mod.current_version_id || mod.disabled) continue

        try {
          const versions = await invoke<ModrinthVersion[]>("get_mod_versions", {
            idOrSlug: mod.project_id,
            loaders: [instance.loader],
            gameVersions: [mcVersion],
          })

          if (versions && versions.length > 0) {
            const latestVersion = versions[0]

            if (latestVersion.id !== mod.current_version_id) {
              const primaryFile = latestVersion.files.find((f: ModrinthFile) => f.primary) || latestVersion.files[0]

              if (primaryFile) {
                updates.push({
                  filename: mod.filename,
                  projectId: mod.project_id,
                  currentVersionId: mod.current_version_id,
                  latestVersion: { id: latestVersion.id, name: latestVersion.name, version_number: latestVersion.version_number, downloadUrl: primaryFile.url, filename: primaryFile.filename },
                })
              }
            }
          }
        } catch (error) {
          console.error(`Failed to check updates for ${mod.name || mod.filename}:`, error)
        }
      }

      availableUpdates = updates
    } catch (error) {
      console.error("Failed to check for updates:", error)
      alertModal = { isOpen: true, title: "Error", message: `Failed to check for updates: ${String(error)}`, type: "danger" }
    } finally {
      isCheckingUpdates = false
    }
  }

  async function updateAllMods() {
    if (availableUpdates.length === 0) return

    isUpdatingMods = true

    for (const update of availableUpdates) {
      try {
        await invoke("download_mod", { instanceName: instance.name, downloadUrl: update.latestVersion.downloadUrl, filename: update.latestVersion.filename })
        if (update.filename !== update.latestVersion.filename) {
          await invoke("delete_mod", { instanceName: instance.name, filename: update.filename }).catch((err: Error) =>
            console.error(`Failed to delete old version ${update.filename}:`, err)
          )
        }
      } catch (error) {
        console.error(`Failed to update mod ${update.filename}:`, error)
      }
    }

    isUpdatingMods = false
    availableUpdates = []

    await loadInstalledMods()
  }

  async function updateSingleMod(update: ModUpdate) {
    try {
      await invoke("download_mod", { instanceName: instance.name, downloadUrl: update.latestVersion.downloadUrl, filename: update.latestVersion.filename })
      if (update.filename !== update.latestVersion.filename) {
        await invoke("delete_mod", { instanceName: instance.name, filename: update.filename }).catch((err: Error) =>
          console.error(`Failed to delete old version ${update.filename}:`, err)
        )
      }
      availableUpdates = availableUpdates.filter(u => u.filename !== update.filename)
      await loadInstalledMods()
    } catch (error) {
      console.error(`Failed to update mod ${update.filename}:`, error)
    }
  }

  function getFabricLoaderVersion(instance: Instance): string | null {
    if (instance.loader === "fabric") {
      const parts = instance.version.split("-")
      if (parts.length >= 2) return parts[parts.length - 2]
    }
    return null
  }

  function getNeoForgeVersion(instance: Instance): string | null {
    if (instance.loader === "neoforge") {
      const versionPart = instance.version.replace("neoforge-", "")
      const parts = versionPart.split("-")
      const mcVersion = getMinecraftVersion(instance)
      const neoforgeVersionParts = parts.filter(part => part !== mcVersion)
      if (neoforgeVersionParts.length > 0) return neoforgeVersionParts.join("-")
    }
    return null
  }

  function getForgeVersion(instance: Instance): string | null {
    if (instance.loader === "forge") {
      const parts = instance.version.split("-forge-")
      if (parts.length > 1) return parts[1]
    }
    return null
  }

  async function handleOpenFolder() {
    try {
      await invoke("open_instance_folder", { instanceName: instance.name })
    } catch (error) {
      console.error("Failed to open folder:", error)
    }
  }

  async function handleDeleteMod(filename: string) {
    try {
      await invoke("delete_mod", { instanceName: instance.name, filename })
      await loadInstalledMods()
      availableUpdates = []
    } catch (error) {
      console.error("Failed to delete mod:", error)
      alertModal = { isOpen: true, title: "Error", message: `Failed to delete mod: ${String(error)}`, type: "danger" }
    }
  }

  function handleDeleteWorld(folderName: string, worldName: string) {
    confirmModal = {
      isOpen: true,
      title: "Delete World",
      message: `Are you sure you want to delete "${worldName}"?\n\nThis action cannot be undone.`,
      type: "danger",
      onConfirm: async () => {
        confirmModal = null
        try {
          await invoke("delete_world", { instanceName: instance.name, folderName })
          await loadWorlds()
        } catch (error) {
          console.error("Failed to delete world:", error)
          alertModal = { isOpen: true, title: "Error", message: `Failed to delete world: ${String(error)}`, type: "danger" }
        }
      }
    }
  }

  async function handleDeleteResourcePack(filename: string) {
    try {
      await invoke("delete_resourcepack", { instanceName: instance.name, filename })
      await loadResourcePacks()
    } catch (error) {
      console.error("Failed to delete resource pack:", error)
      alertModal = { isOpen: true, title: "Error", message: `Failed to delete resource pack: ${String(error)}`, type: "danger" }
    }
  }

  async function handleDeleteShaderPack(filename: string) {
    try {
      await invoke("delete_shaderpack", { instanceName: instance.name, filename })
      await loadShaderPacks()
    } catch (error) {
      console.error("Failed to delete shader pack:", error)
      alertModal = { isOpen: true, title: "Error", message: `Failed to delete shader pack: ${String(error)}`, type: "danger" }
    }
  }

  async function handleToggleMod(mod: InstalledMod) {
    const disabled = !mod.disabled
    const filename = disabled
      ? (mod.filename.endsWith(".disabled") ? mod.filename : `${mod.filename}.disabled`)
      : (mod.filename.endsWith(".disabled") ? mod.filename.slice(0, -".disabled".length) : mod.filename)
    const updated = { ...mod, filename, disabled }
    installedMods = installedMods.map(m => m.filename === mod.filename ? updated : m)
    try {
      await invoke("toggle_mod", { instanceName: instance.name, filename: mod.filename, disable: disabled })
    } catch (error) {
      installedMods = installedMods.map(m => m.filename === filename ? mod : m)
      console.error("Failed to toggle mod:", error)
      alertModal = { isOpen: true, title: "Error", message: `Failed to ${mod.disabled ? "enable" : "disable"} mod: ${String(error)}`, type: "danger" }
    }
  }

  async function handleOpenWorldsFolder() {
    try {
      await invoke("open_worlds_folder", { instanceName: instance.name })
    } catch (error) {
      console.error("Failed to open worlds folder:", error)
      alertModal = { isOpen: true, title: "Error", message: `Failed to open worlds folder: ${String(error)}`, type: "danger" }
    }
  }

  async function handleOpenWorldFolder(folderName: string) {
    try {
      await invoke("open_world_folder", { instanceName: instance.name, folderName })
    } catch (error) {
      console.error("Failed to open world folder:", error)
      alertModal = { isOpen: true, title: "Error", message: `Failed to open world folder: ${String(error)}`, type: "danger" }
    }
  }

  async function handleLaunchWorld(worldName: string) {
    launchingWorld = worldName
    await handleWorldLaunch(worldName)
    launchingWorld = null
  }

  async function handleOpenResourcePacksFolder() {
    try {
      await invoke("open_resourcepacks_folder", { instanceName: instance.name })
    } catch (error) {
      console.error("Failed to open resourcepacks folder:", error)
    }
  }

  async function handleOpenShaderPacksFolder() {
    try {
      await invoke("open_shaderpacks_folder", { instanceName: instance.name })
    } catch (error) {
      console.error("Failed to open shaderpacks folder:", error)
    }
  }

  function handleInstanceUpdated() {
    loadInstanceIcon()
    onInstanceUpdated?.()
    loadInstances()
  }

  function handleInstanceDeleted() {
    handleCloseDetails()
  }

  let fabricLoaderVersion = $derived(getFabricLoaderVersion(instance))
  let neoforgeVersion = $derived(getNeoForgeVersion(instance))
  let modsWithProjectId = $derived(
    installedMods.filter(mod => mod.project_id && !mod.disabled).length
  )
  let filteredMods = $derived(
    installedMods.filter(mod => {
      if (!modSearchQuery.trim()) return true
      const query = modSearchQuery.toLowerCase()
      return (mod.name || mod.filename).toLowerCase().includes(query) || mod.filename.toLowerCase().includes(query)
    })
  )
  let filteredWorlds = $derived(
    worlds.filter(world => {
      if (!worldSearchQuery.trim()) return true
      return world.name.toLowerCase().includes(worldSearchQuery.toLowerCase())
    })
  )
  let filteredResourcePacks = $derived(
    resourcePacks.filter(pack => {
      if (!resourcePackSearchQuery.trim()) return true
      const query = resourcePackSearchQuery.toLowerCase()
      return (pack.name || pack.filename).toLowerCase().includes(query) || pack.filename.toLowerCase().includes(query)
    })
  )
  let filteredShaderPacks = $derived(
    shaderPacks.filter(pack => {
      if (!shaderPackSearchQuery.trim()) return true
      const query = shaderPackSearchQuery.toLowerCase()
      return (pack.name || pack.filename).toLowerCase().includes(query) || pack.filename.toLowerCase().includes(query)
    })
  )
</script>

<div class="flex flex-col h-full overflow-hidden">
  <div class="flex-shrink-0 px-8 pt-8 pb-6">
    <div class="max-w-7xl mx-auto">
      <div class="flex items-center gap-4">
        <div class="flex-shrink-0">
          {#if instanceIcon}
            <div class="w-16 h-16 rounded-md overflow-hidden bg-[var(--bg-secondary)]">
              <img src={instanceIcon} alt={instance.name} class="w-full h-full object-cover" />
            </div>
          {:else}
            <div class="w-16 h-16 rounded-md flex items-center justify-center bg-[var(--bg-secondary)]">
              <Package size={48} class="text-[var(--text-muted)]" strokeWidth={1.5} />
            </div>
          {/if}
        </div>

        <div class="flex-1 min-w-0">
          <div class="flex items-center justify-between">
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-3">
                <h1 class="text-2xl font-semibold text-[var(--text-primary)] tracking-tight leading-tight">{instance.name}</h1>
                {#if (instance.total_playtime_seconds ?? 0) > 0}
                  <span class="px-2 py-0.5 bg-[var(--bg-tertiary)] text-[var(--text-muted)] text-xs rounded">
                    {formatPlaytime(instance.total_playtime_seconds ?? 0)}
                  </span>
                {/if}
              </div>
              <p class="text-sm text-[var(--text-muted)] mt-1">
                Minecraft {getMinecraftVersion(instance)}
                {" • "}
                {#if instance.loader === "fabric"}
                  <span class="text-[#3b82f6]">Fabric Loader</span>
                  {#if fabricLoaderVersion}<span class="text-[var(--text-muted)]"> {fabricLoaderVersion}</span>{/if}
                {:else if instance.loader === "neoforge"}
                  <span class="text-[#f97316]">NeoForge</span>
                  {#if neoforgeVersion}<span class="text-[var(--text-muted)]"> {neoforgeVersion}</span>{/if}
                {:else if instance.loader === "forge"}
                  <span class="text-[#e05d2e]">Forge</span>
                  {#if getForgeVersion(instance)}<span class="text-[var(--text-muted)]"> {getForgeVersion(instance)}</span>{/if}
                {:else}
                  <span class="text-[#16a34a]">Vanilla</span>
                {/if}
              </p>
            </div>
            <div class="flex gap-2">
              <button
                onclick={() => handleLaunch(instance)}
                disabled={!store.isAuthenticated || store.launchingInstanceName !== null || store.runningInstances.has(instance.name)}
                class="px-6 py-2 rounded-md font-semibold text-base flex items-center gap-2 transition-all cursor-pointer {store.launchingInstanceName === instance.name || store.runningInstances.has(instance.name) ? 'bg-red-500/10 text-red-400' : 'bg-[#16a34a] hover:bg-[#15803d] text-[#181a1f]'} disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {#if store.launchingInstanceName === instance.name || store.runningInstances.has(instance.name)}
                    <div class="w-4 h-4 border-2 border-red-400/30 border-t-red-400 rounded-full animate-spin"></div>
                    <span>Running...</span>
                {:else}
                  <Play size={20} fill="currentColor" strokeWidth={0} />
                  <span>Play</span>
                {/if}
              </button>
              <button onclick={handleOpenFolder} class="px-4 py-2.5 bg-[var(--bg-tertiary)] hover:bg-[var(--bg-hover-strong)] text-[var(--text-primary)] rounded-md font-medium text-sm flex items-center gap-2 transition-all cursor-pointer">
                <FolderOpen size={18} strokeWidth={2.5} /><span>Open Folder</span>
              </button>
              <button onclick={() => isSettingsOpen = true} class="px-4 py-2.5 bg-[var(--bg-tertiary)] hover:bg-[var(--bg-hover-strong)] text-[var(--text-primary)] rounded-md font-medium text-sm flex items-center gap-2 transition-all cursor-pointer" title="Instance Settings">
                <Settings size={19} strokeWidth={2.5} />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>

  <div class="flex-1 overflow-hidden px-8">
    <div class="max-w-7xl mx-auto flex flex-col h-full">

      <!-- Section tabs -->
      <div class="flex-shrink-0 flex gap-1 mb-4">
        <button onclick={() => detailSection = "mods"} class="px-3 py-1.5 rounded-md text-sm font-medium transition-colors cursor-pointer {tabClass('mods')}">
          Mods
        </button>
        <button onclick={() => detailSection = "worlds"} class="px-3 py-1.5 rounded-md text-sm font-medium transition-colors cursor-pointer {tabClass('worlds')}">
          Worlds
        </button>
        <button onclick={() => detailSection = "resourcepacks"} class="px-3 py-1.5 rounded-md text-sm font-medium transition-colors cursor-pointer {tabClass('resourcepacks')}">
          Resource Packs
        </button>
        <button onclick={() => detailSection = "shaderpacks"} class="px-3 py-1.5 rounded-md text-sm font-medium transition-colors cursor-pointer {tabClass('shaderpacks')}">
          Shader Packs
        </button>
      </div>

      {#if detailSection === "mods"}
        <div class="flex-1 flex flex-col min-h-0 overflow-hidden">
          <div class="flex-shrink-0">
            <div class="flex items-center justify-between mb-3">
              <div class="flex items-center gap-3">
                <h2 class="text-lg font-semibold text-[var(--text-primary)] tracking-tight">Installed Mods</h2>
                <span class="px-2 py-0.5 bg-[var(--bg-tertiary)] text-[var(--text-muted)] text-xs rounded">{installedMods.length} mod{installedMods.length === 1 ? "" : "s"}</span>
              </div>
              <div class="flex items-center gap-2">
                {#if (instance.loader === "fabric" || instance.loader === "neoforge") && modsWithProjectId > 0}
                  {#if availableUpdates.length > 0}
                    <button onclick={updateAllMods} disabled={isUpdatingMods} class="flex items-center gap-1.5 px-2 py-0.5 bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] disabled:opacity-50 disabled:cursor-not-allowed text-white rounded text-xs font-medium transition-colors cursor-pointer">
                      {#if isUpdatingMods}
                        <Loader2 size={14} class="animate-spin" /><span>Updating...</span>
                      {:else}
                        <RefreshCw size={14} /><span>Update All ({availableUpdates.length})</span>
                      {/if}
                    </button>
                  {:else}
                    <button onclick={checkForUpdates} disabled={isCheckingUpdates} class="flex items-center gap-1.5 px-2 py-0.5 bg-[var(--bg-tertiary)] hover:bg-[var(--bg-hover-strong)] disabled:opacity-50 text-[var(--text-muted)] hover:text-[var(--text-primary)] rounded text-xs transition-colors cursor-pointer">
                      {#if isCheckingUpdates}
                        <Loader2 size={14} class="animate-spin" /><span>Checking...</span>
                      {:else}
                        <RefreshCw size={14} /><span>Check for Updates</span>
                      {/if}
                    </button>
                  {/if}
                {/if}
              </div>
            </div>

            <div class="relative mb-3">
              <Search size={14} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--text-muted)] pointer-events-none" />
              <input type="text" bind:value={modSearchQuery} placeholder="Search mods..." class="w-full bg-[var(--bg-tertiary)] rounded-md pl-8 pr-8 py-1.5 text-sm text-[var(--text-muted)] placeholder-[var(--text-muted)] focus:outline-none transition-colors" />
              {#if modSearchQuery}
                <button onclick={() => modSearchQuery = ""} class="absolute right-2.5 top-1/2 -translate-y-1/2 text-[var(--text-muted)] hover:text-[var(--text-muted)] transition-colors cursor-pointer"><X size={14} /></button>
              {/if}
            </div>
          </div>

          <div class="flex-1 overflow-y-auto min-h-0">
            {#if isLoadingMods}
              <div class="text-center py-16"><Loader2 size={32} class="animate-spin text-[#16a34a] mx-auto" /></div>
            {:else if installedMods.length === 0}
              <div class="flex flex-col items-center justify-center py-16">
                <Package size={48} class="text-[#16a34a] mb-3" strokeWidth={1.5} />
                <h3 class="text-base font-semibold text-[var(--text-primary)] mb-1">No mods installed</h3>
                <p class="text-sm text-[var(--text-muted)]">Browse the mods tab to add mods</p>
              </div>
            {:else if filteredMods.length === 0}
              <div class="flex flex-col items-center justify-center py-16">
                <Search size={32} class="text-[var(--text-muted)] mb-3" strokeWidth={1.5} />
                <p class="text-sm text-[var(--text-muted)]">No mods match your search</p>
              </div>
            {:else}
              <div class="space-y-3 pr-1">
                {#each filteredMods as mod (mod.filename)}
                  {@const hasUpdate = availableUpdates.some(u => u.filename === mod.filename)}
                  <div class="bg-[var(--bg-tertiary)] rounded-md overflow-hidden transition-all {mod.disabled ? 'opacity-60' : ''}">
                    <div class="flex min-h-0">
                      {#if mod.icon_url}
                        <div class="w-22 bg-[var(--bg-secondary)] flex items-center justify-center flex-shrink-0 self-stretch">
                          <img src={mod.icon_url} alt={mod.name || mod.filename} class="w-full h-full object-contain {mod.disabled ? 'grayscale' : ''}" />
                        </div>
                      {:else}
                        <div class="w-22 flex items-center justify-center flex-shrink-0 self-stretch">
                          <Package size={36} class="text-[var(--text-muted)]" strokeWidth={2} />
                        </div>
                      {/if}
                      <div class="flex-1 min-w-0 py-2 px-4 flex items-center gap-3 relative z-0">
                        <div class="flex-1 min-w-0">
                          <div class="flex items-center gap-2">
                            <h3 class="font-semibold text-base text-[var(--text-primary)] truncate">{mod.name || mod.filename}</h3>
                            {#if hasUpdate}
                              <button onclick={() => { const u = availableUpdates.find(up => up.filename === mod.filename); if (u) updateSingleMod(u) }} class="px-1.5 py-0.5 bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white text-xs rounded font-medium transition-colors cursor-pointer">Update</button>
                            {/if}
                          </div>
                          <p class="text-sm text-[var(--text-muted)] truncate">{mod.filename}{mod.disabled && !mod.filename.endsWith(".disabled") ? ".disabled" : ""}</p>
                          <p class="text-sm text-[var(--text-muted)] mt-0.5">{formatFileSize(mod.size)}</p>
                        </div>
                        <div class="flex items-center gap-3">
                            <button onclick={() => handleToggleMod(mod)} class="w-6 h-6 rounded border-2 flex items-center justify-center transition-all cursor-pointer {mod.disabled ? 'bg-[var(--bg-hover-strong)] border-[var(--text-muted)]' : 'bg-[#16a34a] border-[#16a34a]'}" aria-label="Toggle mod">
                            <Check size={16} strokeWidth={4} color={mod.disabled ? "#7d8590" : "#0f1115"} />
                          </button>
                          <button onclick={() => handleDeleteMod(mod.filename)} class="p-1 hover:bg-red-500/10 text-[var(--text-muted)] hover:text-red-400 rounded transition-all cursor-pointer">
                            <Trash2 size={20} />
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        </div>

      {:else if detailSection === "worlds"}
        <div class="flex-1 flex flex-col min-h-0 overflow-hidden">
          <div class="flex-shrink-0">
            <div class="flex items-center justify-between mb-3">
              <div class="flex items-center gap-3">
                <h2 class="text-lg font-semibold text-[var(--text-primary)] tracking-tight">Worlds</h2>
                <span class="px-2 py-0.5 bg-[var(--bg-tertiary)] text-[var(--text-muted)] text-xs rounded">{worlds.length} world{worlds.length === 1 ? "" : "s"}</span>
              </div>
              <button onclick={handleOpenWorldsFolder} class="flex items-center gap-1.5 px-2 py-0.5 bg-[var(--bg-tertiary)] hover:bg-[var(--bg-hover-strong)] text-[var(--text-muted)] hover:text-[var(--text-primary)] rounded text-xs transition-colors cursor-pointer">
                <ExternalLink size={14} /><span>Open Folder</span>
              </button>
            </div>

            <div class="relative mb-3">
              <Search size={14} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--text-muted)] pointer-events-none" />
              <input type="text" bind:value={worldSearchQuery} placeholder="Search worlds..." class="w-full bg-[var(--bg-tertiary)] rounded-md pl-8 pr-8 py-1.5 text-sm text-[var(--text-muted)] placeholder-[var(--text-muted)] focus:outline-none transition-colors" />
              {#if worldSearchQuery}
                <button onclick={() => worldSearchQuery = ""} class="absolute right-2.5 top-1/2 -translate-y-1/2 text-[var(--text-muted)] hover:text-[var(--text-muted)] transition-colors cursor-pointer"><X size={14} /></button>
              {/if}
            </div>
          </div>

          <div class="flex-1 overflow-y-auto min-h-0">
            {#if isLoadingWorlds}
              <div class="text-center py-16"><Loader2 size={32} class="animate-spin text-[#16a34a] mx-auto" /></div>
            {:else if worlds.length === 0}
              <div class="flex flex-col items-center justify-center py-16">
                <Globe size={48} class="text-[#16a34a] mb-3" strokeWidth={1.5} />
                <h3 class="text-base font-semibold text-[var(--text-primary)] mb-1">No worlds yet</h3>
                <p class="text-sm text-[var(--text-muted)]">Launch the game to create a world</p>
              </div>
            {:else if filteredWorlds.length === 0}
              <div class="flex flex-col items-center justify-center py-16">
                <Search size={32} class="text-[var(--text-muted)] mb-3" strokeWidth={1.5} />
                <p class="text-sm text-[var(--text-muted)]">No worlds match your search</p>
              </div>
            {:else}
              <div class="space-y-3 pr-1">
                {#each filteredWorlds as world (world.folder_name)}
                  <div class="bg-[var(--bg-tertiary)] rounded-md overflow-hidden transition-all">
                    <div class="flex min-h-0">
                      {#if world.icon}
                        <div class="w-22 bg-[var(--bg-secondary)] flex items-center justify-center flex-shrink-0 self-stretch">
                          <img src={world.icon} alt={world.name} class="w-full h-full object-contain" />
                        </div>
                      {:else}
                        <div class="w-22 flex items-center justify-center flex-shrink-0 self-stretch">
                          <Globe size={32} class="text-[var(--text-muted)]" strokeWidth={1.5} />
                        </div>
                      {/if}
                      <div class="flex-1 min-w-0 py-2 px-4 flex items-center gap-3 relative z-0">
                        <div class="flex-1 min-w-0">
                          <h3 class="font-semibold text-base text-[var(--text-primary)] truncate">{world.name}</h3>
                          <p class="text-xs text-[var(--text-muted)] mt-0.5">Created {world.created ? formatDate(world.created) : "Unknown"}</p>
                          <div class="flex items-center gap-2 text-sm text-[var(--text-muted)] mt-0.5">
                            <span>{formatFileSize(world.size)}</span>
                            {#if world.game_mode}
                              <span>•</span><span class="capitalize">{world.game_mode}</span>
                            {/if}
                          </div>
                        </div>
                        <div class="flex items-center gap-1">
                          <button onclick={() => handleLaunchWorld(world.folder_name)} disabled={!store.isAuthenticated || store.launchingInstanceName !== null || store.runningInstances.has(instance.name) || launchingWorld !== null} class="p-2.5 rounded-md transition-all cursor-pointer {(store.launchingInstanceName === instance.name || launchingWorld === world.folder_name) ? 'bg-red-500/10 text-red-400' : 'bg-[#16a34a] hover:bg-[#15803d] text-[#181a1f]'} disabled:opacity-40 disabled:cursor-not-allowed" title="Play this world">
                            {#if store.launchingInstanceName === instance.name || launchingWorld === world.folder_name}
                              <div class="w-4 h-4 border-2 border-red-400/30 border-t-red-400 rounded-full animate-spin"></div>
                            {:else}
                              <Play size={20} fill="currentColor" strokeWidth={0} />
                            {/if}
                          </button>
                          <button onclick={() => handleOpenWorldFolder(world.folder_name)} class="p-2.5 hover:bg-[var(--bg-hover-strong)] text-[var(--text-muted)] hover:text-[var(--text-primary)] rounded-md transition-all cursor-pointer" title="Open world folder"><FolderOpen size={20} /></button>
                          <button onclick={() => handleDeleteWorld(world.folder_name, world.name)} class="p-2.5 hover:bg-red-500/10 text-[var(--text-muted)] hover:text-red-400 rounded-md transition-all cursor-pointer" title="Delete world"><Trash2 size={20} /></button>
                        </div>
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        </div>

      {:else if detailSection === "resourcepacks"}
        <div class="flex-1 flex flex-col min-h-0 overflow-hidden">
          <div class="flex-shrink-0">
            <div class="flex items-center justify-between mb-3">
              <div class="flex items-center gap-3">
                <h2 class="text-lg font-semibold text-[var(--text-primary)] tracking-tight">Resource Packs</h2>
                <span class="px-2 py-0.5 bg-[var(--bg-tertiary)] text-[var(--text-muted)] text-xs rounded">{resourcePacks.length} pack{resourcePacks.length === 1 ? "" : "s"}</span>
              </div>
              <button onclick={handleOpenResourcePacksFolder} class="flex items-center gap-1.5 px-2 py-0.5 bg-[var(--bg-tertiary)] hover:bg-[var(--bg-hover-strong)] text-[var(--text-muted)] hover:text-[var(--text-primary)] rounded text-xs transition-colors cursor-pointer">
                <ExternalLink size={14} /><span>Open Folder</span>
              </button>
            </div>
            <div class="relative mb-3">
              <Search size={14} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--text-muted)] pointer-events-none" />
              <input type="text" bind:value={resourcePackSearchQuery} placeholder="Search resource packs..." class="w-full bg-[var(--bg-tertiary)] rounded-md pl-8 pr-8 py-1.5 text-sm text-[var(--text-muted)] placeholder-[var(--text-muted)] focus:outline-none transition-colors" />
              {#if resourcePackSearchQuery}
                <button onclick={() => resourcePackSearchQuery = ""} class="absolute right-2.5 top-1/2 -translate-y-1/2 text-[var(--text-muted)] hover:text-[var(--text-muted)] transition-colors cursor-pointer"><X size={14} /></button>
              {/if}
            </div>
          </div>
          <div class="flex-1 overflow-y-auto min-h-0">
            {#if isLoadingResourcePacks}
              <div class="text-center py-16"><Loader2 size={32} class="animate-spin text-[#16a34a] mx-auto" /></div>
            {:else if resourcePacks.length === 0}
              <div class="flex flex-col items-center justify-center py-16">
                <Image size={48} class="text-[#f97316] mb-3" strokeWidth={1.5} />
                <h3 class="text-base font-semibold text-[var(--text-primary)] mb-1">No resource packs</h3>
                <p class="text-sm text-[var(--text-muted)]">Browse the addons tab to add resource packs</p>
              </div>
            {:else if filteredResourcePacks.length === 0}
              <div class="flex flex-col items-center justify-center py-16">
                <Search size={32} class="text-[var(--text-muted)] mb-3" strokeWidth={1.5} />
                <p class="text-sm text-[var(--text-muted)]">No resource packs match your search</p>
              </div>
            {:else}
              <div class="space-y-3 pr-1">
                {#each filteredResourcePacks as pack (pack.filename)}
                  <div class="bg-[var(--bg-tertiary)] rounded-md overflow-hidden transition-all">
                    <div class="flex min-h-0">
                      {#if pack.icon_url}
                        <div class="w-22 bg-[var(--bg-secondary)] flex items-center justify-center flex-shrink-0 self-stretch">
                          <img src={pack.icon_url} alt={pack.name || pack.filename} class="w-full h-full object-contain" />
                        </div>
                      {:else}
                        <div class="w-22 flex items-center justify-center flex-shrink-0 self-stretch">
                          <Image size={36} class="text-[#f97316]" strokeWidth={1.5} />
                        </div>
                      {/if}
                      <div class="flex-1 min-w-0 py-2 px-4 flex items-center gap-3 relative z-0">
                        <div class="flex-1 min-w-0">
                          <h3 class="font-semibold text-base text-[var(--text-primary)] truncate">{pack.name || pack.filename}</h3>
                          <p class="text-sm text-[var(--text-muted)] truncate">{pack.filename}</p>
                          <p class="text-sm text-[var(--text-muted)] mt-0.5">{formatFileSize(pack.size)}</p>
                        </div>
                        <div class="flex flex-col items-center gap-1 self-center">
                          <button onclick={() => handleDeleteResourcePack(pack.filename)} class="p-1.5 hover:bg-red-500/10 text-[var(--text-muted)] hover:text-red-400 rounded-md transition-all cursor-pointer">
                            <Trash2 size={20} />
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        </div>

      {:else}
        <div class="flex-1 flex flex-col min-h-0 overflow-hidden">
          <div class="flex-shrink-0">
            <div class="flex items-center justify-between mb-3">
              <div class="flex items-center gap-3">
                <h2 class="text-lg font-semibold text-[var(--text-primary)] tracking-tight">Shader Packs</h2>
                <span class="px-2 py-0.5 bg-[var(--bg-tertiary)] text-[var(--text-muted)] text-xs rounded">{shaderPacks.length} pack{shaderPacks.length === 1 ? "" : "s"}</span>
              </div>
              <button onclick={handleOpenShaderPacksFolder} class="flex items-center gap-1.5 px-2 py-0.5 bg-[var(--bg-tertiary)] hover:bg-[var(--bg-hover-strong)] text-[var(--text-muted)] hover:text-[var(--text-primary)] rounded text-xs transition-colors cursor-pointer">
                <ExternalLink size={14} /><span>Open Folder</span>
              </button>
            </div>
            <div class="relative mb-3">
              <Search size={14} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--text-muted)] pointer-events-none" />
              <input type="text" bind:value={shaderPackSearchQuery} placeholder="Search shader packs..." class="w-full bg-[var(--bg-tertiary)] rounded-md pl-8 pr-8 py-1.5 text-sm text-[var(--text-muted)] placeholder-[var(--text-muted)] focus:outline-none transition-colors" />
              {#if shaderPackSearchQuery}
                <button onclick={() => shaderPackSearchQuery = ""} class="absolute right-2.5 top-1/2 -translate-y-1/2 text-[var(--text-muted)] hover:text-[var(--text-muted)] transition-colors cursor-pointer"><X size={14} /></button>
              {/if}
            </div>
          </div>
          <div class="flex-1 overflow-y-auto min-h-0">
            {#if isLoadingShaderPacks}
              <div class="text-center py-16"><Loader2 size={32} class="animate-spin text-[#16a34a] mx-auto" /></div>
            {:else if shaderPacks.length === 0}
              <div class="flex flex-col items-center justify-center py-16">
                <Palette size={48} class="text-[#8b5cf6] mb-3" strokeWidth={1.5} />
                <h3 class="text-base font-semibold text-[var(--text-primary)] mb-1">No shader packs</h3>
                <p class="text-sm text-[var(--text-muted)]">Browse the addons tab to add shader packs</p>
              </div>
            {:else if filteredShaderPacks.length === 0}
              <div class="flex flex-col items-center justify-center py-16">
                <Search size={32} class="text-[var(--text-muted)] mb-3" strokeWidth={1.5} />
                <p class="text-sm text-[var(--text-muted)]">No shader packs match your search</p>
              </div>
            {:else}
              <div class="space-y-3 pr-1">
                {#each filteredShaderPacks as pack (pack.filename)}
                  <div class="bg-[var(--bg-tertiary)] rounded-md overflow-hidden transition-all">
                    <div class="flex min-h-0">
                      {#if pack.icon_url}
                        <div class="w-22 bg-[var(--bg-secondary)] flex items-center justify-center flex-shrink-0 self-stretch">
                          <img src={pack.icon_url} alt={pack.name || pack.filename} class="w-full h-full object-contain" />
                        </div>
                      {:else}
                        <div class="w-22 flex items-center justify-center flex-shrink-0 self-stretch">
                          <Palette size={36} class="text-[#8b5cf6]" strokeWidth={1.5} />
                        </div>
                      {/if}
                      <div class="flex-1 min-w-0 py-2 px-4 flex items-center gap-3 relative z-0">
                        <div class="flex-1 min-w-0">
                          <h3 class="font-semibold text-base text-[var(--text-primary)] truncate">{pack.name || pack.filename}</h3>
                          <p class="text-sm text-[var(--text-muted)] truncate">{pack.filename}</p>
                          <p class="text-sm text-[var(--text-muted)] mt-0.5">{formatFileSize(pack.size)}</p>
                        </div>
                        <div class="flex flex-col items-center gap-1 self-center">
                          <button onclick={() => handleDeleteShaderPack(pack.filename)} class="p-1.5 hover:bg-red-500/10 text-[var(--text-muted)] hover:text-red-400 rounded-md transition-all cursor-pointer">
                            <Trash2 size={20} />
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        </div>
      {/if}

    </div>
  </div>
</div>

<InstanceSettingsModal
  isOpen={isSettingsOpen}
  instance={instance}
  instanceIcon={instanceIcon}
  onClose={() => isSettingsOpen = false}
  onInstanceUpdated={handleInstanceUpdated}
  onInstanceDeleted={handleInstanceDeleted}
/>

{#if confirmModal}
  <ConfirmModal
    isOpen={confirmModal.isOpen}
    title={confirmModal.title}
    message={confirmModal.message}
    type={confirmModal.type}
    confirmText={confirmModal.type === "danger" ? "Delete" : "Confirm"}
    onConfirm={confirmModal.onConfirm}
    onCancel={() => confirmModal = null}
  />
{/if}

{#if alertModal}
  <AlertModal
    isOpen={alertModal.isOpen}
    title={alertModal.title}
    message={alertModal.message}
    type={alertModal.type}
    onClose={() => alertModal = null}
  />
{/if}
