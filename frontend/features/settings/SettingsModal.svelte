<script lang="ts">
  import { invoke } from "@tauri-apps/api/core"
  import { Loader2, Coffee, Cpu, ImagePlus, FolderOpen, X, Check, ChevronDown, Info, Terminal, Paintbrush, Play, Sparkles } from "lucide-svelte"
  import AlertModal from "../../components/ui/AlertModal.svelte"
  import TrashSection from "./TrashSection.svelte"
  import { store, setSettings, loadBackground, setShowOnboarding } from "../../lib/launcherStore.svelte"
  import { storeSet } from "../../lib/store"
  import type { LauncherSettings } from "../../types"

  const THEMES = [
    { id: "octane", label: "Octane", colors: ["#1a1d23", "#252932", "#4572e3", "#e6e6e6"] },
    { id: "light", label: "Light", colors: ["#ffffff", "#f0f0f0", "#4361ee", "#1a1d23"] },
    { id: "rose", label: "Rosé", colors: ["#1a1423", "#2a1a33", "#f472b6", "#e6e6e6"] },
    { id: "cherry", label: "Cherry", colors: ["#1a0d0f", "#2a1417", "#dc2626", "#e6e6e6"] },
  ] as const

  let { isOpen, onClose }: { isOpen: boolean; onClose: () => void } = $props()

  interface SystemInfo {
    total_memory_mb: number
    available_memory_mb: number
    recommended_max_memory_mb: number
  }

  interface StorageCategory {
    name: string
    size_bytes: number
  }

  let javaInstallations: string[] = $state([])
  let isLoadingJava = $state(false)
  let showCustomPath = $state(false)
  let customPathValue = $state("")
  let systemInfo: SystemInfo | null = $state(null)
  let sidebarBgPreview: string | null = $state(null)
  let appVersion = $state("")
  let semanticVersion = $state("")
  let storageCategories: StorageCategory[] = $state([])
  let storageLoading = $state(false)
  let fileInputEl: HTMLInputElement | undefined = $state()
  let alertModal: {
    isOpen: boolean
    title: string
    message: string
    type: "warning" | "danger" | "success" | "info"
  } | null = $state(null)
  let isClosing = $state(false)
  let isJavaDropdownOpen = $state(false)
  let javaDropdownEl: HTMLDivElement | undefined = $state()
  let isTabDropdownOpen = $state(false)
  let tabDropdownEl: HTMLDivElement | undefined = $state()
  let saveTimeout: ReturnType<typeof setTimeout> | undefined
  let ramSliderValue = $state(store.settings?.memory_mb ?? 4096)

  let ramPercent = $derived(store.settings ? ((ramSliderValue - 1024) / (((systemInfo as SystemInfo | null)?.total_memory_mb || 32768) - 1024)) * 100 : 0)
  let totalBytes = $derived(storageCategories.reduce((sum, c) => sum + c.size_bytes, 0))

  const storageColors: Record<string, string> = {
    Instances: "#3b82f6",
    Cache: "#f59e0b",
    Trash: "#ef4444",
    Other: "#6b7280",
  }

  $effect(() => {
    if (isOpen) {
      ramSliderValue = store.settings?.memory_mb ?? 4096
      loadSystemInfo()
      loadSidebarBackground()
      loadJavaInstallations()
      loadAppVersion()
      loadStorageUsage()
    }

    return () => {
      if (saveTimeout) clearTimeout(saveTimeout)
    }
  })

  $effect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (javaDropdownEl && !javaDropdownEl.contains(event.target as Node)) {
        isJavaDropdownOpen = false
      }
      if (tabDropdownEl && !tabDropdownEl.contains(event.target as Node)) {
        isTabDropdownOpen = false
      }
    }

    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  })

  $effect(() => {
    if (!store.settings?.java_path) {
      showCustomPath = false
      customPathValue = ""
      return
    }

    if (javaInstallations.length > 0) {
      const isCustom = !javaInstallations.includes(store.settings.java_path)
      showCustomPath = isCustom
      if (isCustom) customPathValue = store.settings.java_path
      else customPathValue = ""
    }
  })

  async function loadAppVersion() {
    try {
      const version = await invoke<string>("get_app_version")
      appVersion = version
      semanticVersion = version.split('-')[0]
    } catch (error) {
      console.error("Failed to get app version:", error)
    }
  }

  async function loadStorageUsage() {
    storageLoading = true
    try {
      const data = await invoke<StorageCategory[]>("get_storage_usage")
      storageCategories = data
    } catch (error) {
      console.error("Failed to load storage usage:", error)
    } finally {
      storageLoading = false
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`
  }

  async function loadSystemInfo() {
    try {
      const info = await invoke<SystemInfo>("get_system_info")
      systemInfo = info
    } catch (error) {
      console.error("Failed to get system info:", error)
    }
  }

  async function loadSidebarBackground() {
    try {
      const bg = await invoke<string | null>("get_background")
      sidebarBgPreview = bg
    } catch (error) {
      console.error("Failed to load background:", error)
    }
  }

  async function loadJavaInstallations() {
    isLoadingJava = true
    try {
      const installations = await invoke<string[]>("detect_java_installations")
      javaInstallations = installations
    } catch (error) {
      console.error("Failed to detect Java installations:", error)
    } finally {
      isLoadingJava = false
    }
  }

  async function handleSettingChange(newSettings: LauncherSettings) {
    try {
      await invoke("save_settings", { settings: newSettings })
      await storeSet('octane_theme', newSettings.theme ?? 'octane')
      setSettings(newSettings)
    } catch (error) {
      console.error("Failed to save settings:", error)
      alertModal = { isOpen: true, title: "An error occurred", message: "Failed to save settings" + `: ${error}`, type: "danger" }
    }
  }

  function handleSettingChangeDebounced(newSettings: LauncherSettings) {
    setSettings(newSettings)
    if (saveTimeout) clearTimeout(saveTimeout)
    saveTimeout = setTimeout(() => handleSettingChange(newSettings), 500)
  }

  async function handleFileSelect(e: any) {
    const file = e.target.files?.[0]
    if (!file) return

    if (!file.type.startsWith('image/')) {
      alertModal = { isOpen: true, title: "Invalid File", message: "Please select an image file (PNG, JPG, etc.)", type: "warning" }
      return
    }

    if (file.size > 10 * 1024 * 1024) {
      alertModal = { isOpen: true, title: "File Too Large", message: "Image must be smaller than 10MB", type: "warning" }
      return
    }

    try {
      const reader = new FileReader()
      reader.onload = async (e) => {
        const base64 = e.target?.result as string

        try {
          await invoke("set_background", { imageData: base64 })
          sidebarBgPreview = base64
          loadBackground()
        } catch (error) {
          console.error("Failed to save background:", error)
          alertModal = { isOpen: true, title: "An error occurred", message: "Failed to save background" + `: ${error}`, type: "danger" }
        }
      }
      reader.readAsDataURL(file)
    } catch (error) {
      console.error("Failed to read file:", error)
      alertModal = { isOpen: true, title: "An error occurred", message: "Failed to read image file", type: "danger" }
    }

    if (fileInputEl) fileInputEl.value = ''
  }

  async function handleRemoveBackground() {
    try {
      await invoke("remove_background")
      sidebarBgPreview = null
      loadBackground()
    } catch (error) {
      console.error("Failed to remove background:", error)
      alertModal = { isOpen: true, title: "An error occurred", message: "Failed to remove background" + `: ${error}`, type: "danger" }
    }
  }

  async function handleOpenDirectory(path: string) {
    try {
      await invoke("open_directory", { path })
    } catch (error) {
      console.error("Failed to open directory:", error)
      alertModal = { isOpen: true, title: "An error occurred", message: "Failed to open directory" + `: ${error}`, type: "danger" }
    }
  }

  function handleClose() {
    isClosing = true
    setTimeout(() => { isClosing = false; onClose() }, 150)
  }
</script>

{#if isOpen}
  {#if !store.settings}
    <div class="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50">
      <div class="bg-[var(--bg-primary)] rounded p-8">
        <div class="flex items-center gap-2 text-[var(--text-muted)] text-base">
          <Loader2 size={20} class="animate-spin" />
          <span>Loading settings...</span>
        </div>
      </div>
    </div>
  {:else}
    <div
      class="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4 modal-backdrop"
      class:closing={isClosing}
      role="presentation"
      onclick={handleClose}
      onkeydown={(e) => { if (e.key === 'Escape') handleClose() }}
    >
      <div
        role="presentation"
        class="blur-border bg-[var(--bg-primary)] rounded w-full max-w-2xl max-h-[80vh] flex flex-col shadow-2xl modal-content"
        class:closing={isClosing}
        onclick={(e) => e.stopPropagation()}
      >
        <div class="flex items-center justify-between px-5 pt-4 pb-3 border-b border-[var(--border-default)]">
          <h2 class="text-lg font-semibold text-[var(--text-primary)]">Settings</h2>
          <div class="flex items-center gap-3">
            {#if appVersion}
              <span class="bg-[var(--bg-elevated)] px-2.5 py-1 rounded text-xs text-[var(--text-muted)]">
                Build {appVersion.split('-')[1] || appVersion}
              </span>
            {/if}
            <button onclick={handleClose} class="p-1.5 hover:bg-[var(--bg-hover)] rounded transition-colors text-[var(--text-muted)] hover:text-[var(--text-primary)] cursor-pointer">
              <X size={18} />
            </button>
          </div>
        </div>

        <div class="flex-1 overflow-y-auto p-5 space-y-6">
          <!-- Memory -->
          <div class="space-y-2">
            <div class="flex items-center gap-2 text-[var(--text-primary)]">
              <Cpu size={16} class="text-[var(--accent-primary)]" />
              <span class="font-medium text-sm">Memory Allocation</span>
            </div>
            <div class="bg-[var(--bg-elevated)] rounded p-3 space-y-2">
              <div class="flex items-baseline justify-between">
                <span class="text-xl font-bold text-[var(--text-primary)]">{(ramSliderValue / 1024).toFixed(1)} GB</span>
                <span class="text-xs text-[var(--text-muted)]">of {systemInfo ? (systemInfo.total_memory_mb / 1024).toFixed(0) : '16'} GB total</span>
              </div>
              <div class="relative h-6 flex items-center">
                <div class="absolute inset-x-0 h-2 bg-[var(--bg-primary)] rounded-full"></div>
                <div class="absolute h-2 rounded-full" style="width: {ramPercent}%; background: var(--accent-primary)"></div>
                <div class="absolute w-4 h-4 rounded-full bg-[var(--accent-primary)] -translate-x-1/2 shadow-md" style="left: {ramPercent}%"></div>
                <input
                  type="range" min="1024" max={systemInfo?.total_memory_mb || 32768} step="512"
                  bind:value={ramSliderValue}
                  oninput={() => handleSettingChangeDebounced({ ...store.settings!, memory_mb: ramSliderValue } as LauncherSettings)}
                  class="absolute inset-0 w-full opacity-0 cursor-pointer"
                />
              </div>
              {#if systemInfo}
                <div class="flex justify-between text-xs">
                  <span class="text-[var(--text-muted)]">Available</span>
                  <span class="text-[var(--text-primary)] font-medium">{(systemInfo.available_memory_mb / 1024).toFixed(1)} GB</span>
                </div>
              {/if}
            </div>
          </div>

          <!-- Java -->
          <div class="space-y-2 min-w-0">
            <div class="flex items-center gap-2 text-[var(--text-primary)]">
              <Coffee size={16} class="text-[var(--accent-primary)]" />
              <span class="font-medium text-sm">Java Runtime</span>
            </div>
            <div class="flex gap-2 min-w-0">
              <div class="relative flex-1 min-w-0" bind:this={javaDropdownEl}>
                <button
                  onclick={() => isJavaDropdownOpen = !isJavaDropdownOpen}
                  class="w-full bg-[var(--bg-elevated)] px-3 py-2 text-sm text-[var(--text-primary)] text-left flex items-center justify-between cursor-pointer min-w-0 rounded"
                >
                  <span class="truncate">
                    {showCustomPath ? "Custom Path..." : (store.settings.java_path || "Auto-detect (Recommended)")}
                  </span>
                  <ChevronDown size={14} class="flex-shrink-0 ml-2 transition-transform {isJavaDropdownOpen ? 'rotate-180' : ''}" />
                </button>

                {#if isJavaDropdownOpen}
                  <div class="absolute z-[60] w-full bg-[var(--bg-elevated)] rounded shadow-lg max-h-60 overflow-y-auto mt-1">
                    <button
                      onclick={() => { showCustomPath = false; customPathValue = ""; handleSettingChange({ ...store.settings!, java_path: null } as LauncherSettings); isJavaDropdownOpen = false }}
                      class="w-full px-3 py-2 text-sm text-left hover:bg-[var(--bg-hover)] text-[var(--text-primary)] flex items-center justify-between cursor-pointer"
                    >
                      <span>Auto-detect (Recommended)</span>
                      {#if !store.settings.java_path && !showCustomPath}<Check size={14} class="text-[var(--text-primary)]" />{/if}
                    </button>
                    {#each javaInstallations as path (path)}
                      <button
                        onclick={() => { showCustomPath = false; customPathValue = ""; handleSettingChange({ ...store.settings!, java_path: path } as LauncherSettings); isJavaDropdownOpen = false }}
                        class="w-full px-3 py-2 text-sm text-left hover:bg-[var(--bg-hover)] text-[var(--text-primary)] flex items-center justify-between cursor-pointer"
                      >
                        <span class="truncate">{path}</span>
                        {#if store.settings.java_path === path && !showCustomPath}<Check size={14} class="text-[var(--text-primary)] flex-shrink-0 ml-2" />{/if}
                      </button>
                    {/each}
                    <button
                      onclick={() => { showCustomPath = true; customPathValue = store.settings?.java_path || ""; isJavaDropdownOpen = false }}
                      class="w-full px-3 py-2 text-sm text-left hover:bg-[var(--bg-hover)] text-[var(--text-primary)] flex items-center justify-between cursor-pointer"
                    >
                      <span>Custom Path...</span>
                      {#if showCustomPath}<Check size={14} class="text-[var(--text-primary)]" />{/if}
                    </button>
                  </div>
                {/if}
              </div>

              <button onclick={loadJavaInstallations} disabled={isLoadingJava} class="px-3 py-2 bg-[var(--bg-elevated)] hover:bg-[var(--bg-hover)] disabled:opacity-50 rounded text-sm font-medium text-[var(--text-primary)] cursor-pointer disabled:cursor-not-allowed">
                {#if isLoadingJava}<Loader2 size={14} class="animate-spin" />{:else}Scan{/if}
              </button>
            </div>

            {#if showCustomPath}
              <input
                type="text"
                class="w-full bg-[var(--bg-elevated)] rounded px-3 py-2 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none focus:ring-2 focus:ring-[var(--accent-primary)] font-mono min-w-0"
                placeholder="C:\\Program Files\\Java\\jdk-21\\bin\\javaw.exe"
                bind:value={customPathValue}
                onblur={() => { if (customPathValue.trim()) handleSettingChange({ ...store.settings!, java_path: customPathValue.trim() } as LauncherSettings) }}
                onkeydown={(e) => { if (e.key === 'Enter' && customPathValue.trim()) { handleSettingChange({ ...store.settings!, java_path: customPathValue.trim() } as LauncherSettings); (e.currentTarget as HTMLInputElement).blur() } }}
              />
            {/if}
          </div>

          <!-- Appearance -->
          <div class="space-y-2">
            <div class="flex items-center gap-2 text-[var(--text-primary)]">
              <Paintbrush size={16} class="text-[var(--accent-primary)]" />
              <span class="font-medium text-sm">Theme</span>
            </div>
            <div class="grid grid-cols-2 gap-2">
              {#each THEMES as theme}
                <button
                  onclick={() => store.settings && handleSettingChange({ ...store.settings, theme: theme.id })}
                  class="flex items-center justify-between gap-2 p-2.5 rounded transition-all cursor-pointer {store.settings?.theme === theme.id ? 'bg-[var(--accent-primary)]/10 ring-1 ring-[var(--accent-primary)]' : 'bg-[var(--bg-elevated)] hover:bg-[var(--bg-hover)]'}"
                >
                  <span class="text-sm font-medium text-[var(--text-primary)]">{theme.label}</span>
                  <div class="flex items-center gap-1">
                    {#each theme.colors as color}
                      <div class="w-3.5 h-3.5 rounded-full border border-white/10" style="background-color: {color}"></div>
                    {/each}
                  </div>
                </button>
              {/each}
            </div>
          </div>

          <!-- Background -->
          <div class="space-y-2">
            <div class="flex items-center gap-2 text-[var(--text-primary)]">
              <ImagePlus size={16} class="text-[var(--accent-primary)]" />
              <span class="font-medium text-sm">Background</span>
            </div>
            {#if sidebarBgPreview}
              <div class="relative group">
                <div class="h-32 rounded overflow-hidden bg-[var(--bg-elevated)]">
                  <img src={sidebarBgPreview} alt="Background" class="w-full h-full object-cover" />
                </div>
                <div class="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity rounded flex items-center justify-center gap-2">
                  <button onclick={() => fileInputEl?.click()} class="px-3 py-1.5 bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white rounded text-sm font-medium cursor-pointer">Change</button>
                  <button onclick={handleRemoveBackground} class="px-3 py-1.5 bg-red-500 hover:bg-red-600 text-white rounded text-sm font-medium cursor-pointer">Remove</button>
                </div>
              </div>
            {:else}
              <button onclick={() => fileInputEl?.click()} class="w-full h-24 bg-[var(--bg-elevated)] hover:bg-[var(--bg-hover)] border-2 border-dashed border-[var(--border-default)] hover:border-[var(--accent-primary)] rounded transition-all flex flex-col items-center justify-center gap-1.5 cursor-pointer">
                <ImagePlus size={24} class="text-[var(--text-muted)]" />
                <span class="text-xs text-[var(--text-muted)]">Click to upload image</span>
                <span class="text-[10px] text-[var(--text-muted)]">PNG, JPG up to 10MB</span>
              </button>
            {/if}
            <input bind:this={fileInputEl} type="file" accept="image/*" onchange={handleFileSelect} class="hidden" />
          </div>

          <!-- Startup -->
          <div class="space-y-2">
            <div class="flex items-center gap-2 text-[var(--text-primary)]">
              <Play size={16} class="text-[var(--accent-primary)]" />
              <span class="font-medium text-sm">Startup</span>
            </div>
            <div class="flex items-center justify-between bg-[var(--bg-elevated)] rounded p-3">
              <div>
                <span class="text-sm font-medium text-[var(--text-primary)]">Default Tab</span>
                <p class="text-xs text-[var(--text-muted)]">Tab shown when the launcher opens</p>
              </div>
              <div class="relative" bind:this={tabDropdownEl}>
                <button
                  onclick={() => isTabDropdownOpen = !isTabDropdownOpen}
                  class="bg-[var(--bg-hover)] px-3 py-1.5 text-sm text-[var(--text-primary)] rounded flex items-center gap-2 cursor-pointer capitalize"
                >
                  {store.settings.default_tab || "home"}
                  <ChevronDown size={14} class="transition-transform {isTabDropdownOpen ? 'rotate-180' : ''}" />
                </button>
                {#if isTabDropdownOpen}
                  <div class="absolute right-0 z-[60] w-36 bg-[var(--bg-elevated)] rounded shadow-lg overflow-hidden mt-1">
                    {#each ["home", "instances", "browse", "servers", "skins", "screenshots"] as tab}
                      <button
                        onclick={() => { handleSettingChange({ ...store.settings!, default_tab: tab } as LauncherSettings); isTabDropdownOpen = false }}
                        class="w-full px-3 py-2 text-sm text-left hover:bg-[var(--bg-hover)] text-[var(--text-primary)] flex items-center justify-between cursor-pointer capitalize"
                      >
                        {tab}
                        {#if (store.settings.default_tab || "home") === tab}<Check size={14} class="text-[var(--text-primary)]" />{/if}
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          </div>

          <!-- Console -->
          <div class="space-y-2">
            <div class="flex items-center gap-2 text-[var(--text-primary)]">
              <Terminal size={16} class="text-[var(--accent-primary)]" />
              <span class="font-medium text-sm">Console</span>
            </div>
            <div class="flex items-center justify-between bg-[var(--bg-elevated)] rounded p-3">
              <div>
                <span class="text-sm font-medium text-[var(--text-primary)]">Auto-Navigate to Console</span>
                <p class="text-xs text-[var(--text-muted)]">
                  {(store.settings.auto_navigate_to_console ?? true) ? "Switch to Console tab when launching" : "Stay on current tab when launching"}
                </p>
              </div>
              <button
                onclick={() => handleSettingChange({ ...store.settings!, auto_navigate_to_console: !(store.settings?.auto_navigate_to_console ?? true) } as LauncherSettings)}
                class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors cursor-pointer flex-shrink-0 ml-3 {(store.settings.auto_navigate_to_console ?? true) ? 'bg-[var(--accent-primary)]' : 'bg-[var(--bg-hover)]'}"
                aria-label="Toggle auto-navigate to console"
              >
                <span class="inline-block h-4 w-4 transform rounded-full bg-white transition-transform {(store.settings.auto_navigate_to_console ?? true) ? 'translate-x-6' : 'translate-x-1'}"></span>
              </button>
            </div>
          </div>

          <!-- Game Directory -->
          <div class="space-y-2">
            <div class="flex items-center gap-2 text-[var(--text-primary)]">
              <FolderOpen size={16} class="text-[var(--accent-primary)]" />
              <span class="font-medium text-sm">Game Directory</span>
            </div>
            <div class="bg-[var(--bg-elevated)] rounded p-3 flex items-center justify-between gap-3">
              <p class="text-xs text-[var(--text-muted)] font-mono break-all flex-1">{store.launcherDirectory || "Loading..."}</p>
              <button
                onclick={() => handleOpenDirectory(store.launcherDirectory)}
                disabled={!store.launcherDirectory}
                class="flex-shrink-0 px-2.5 py-1.5 bg-[var(--bg-hover)] hover:bg-[var(--bg-hover)] disabled:opacity-50 rounded text-xs font-medium text-[var(--text-primary)] cursor-pointer disabled:cursor-not-allowed flex items-center gap-1.5 transition-colors"
              >
                <FolderOpen size={12} />
                Open
              </button>
            </div>
          </div>

          <!-- Storage Overview -->
          <div class="space-y-2">
            <div class="flex items-center justify-between text-[var(--text-primary)]">
              <div class="flex items-center gap-2">
                <FolderOpen size={16} class="text-[var(--accent-primary)]" />
                <span class="font-medium text-sm">Storage Overview</span>
              </div>
              {#if storageCategories.length > 0}
                <span class="text-xs text-[var(--text-muted)]">{formatBytes(totalBytes)}</span>
              {/if}
            </div>
            <div class="bg-[var(--bg-elevated)] rounded p-3 space-y-3">
              {#if storageLoading}
                <div class="flex items-center gap-2 text-xs text-[var(--text-muted)]">
                  <Loader2 size={14} class="animate-spin" />
                  <span>Calculating storage usage...</span>
                </div>
              {:else if storageCategories.length === 0}
                <div class="text-xs text-[var(--text-muted)]">No data</div>
              {:else}
                <div class="h-2 rounded-full overflow-hidden flex bg-[var(--bg-primary)]">
                  {#each storageCategories as cat (cat.name)}
                    <div
                      style="width: {(cat.size_bytes / totalBytes) * 100}%; background-color: {storageColors[cat.name] || '#6b7280'}"
                      class="h-full first:rounded-l-full last:rounded-r-full"
                    ></div>
                  {/each}
                </div>
                <div class="flex flex-wrap gap-x-4 gap-y-1">
                  {#each storageCategories as cat (cat.name)}
                    <div class="flex items-center gap-1.5 text-xs">
                      <div class="w-2.5 h-2.5 rounded-sm" style="background-color: {storageColors[cat.name] || '#6b7280'}"></div>
                      <span class="text-[var(--text-muted)]">{cat.name}</span>
                      <span class="text-[var(--text-primary)] font-medium">{formatBytes(cat.size_bytes)}</span>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          </div>

          <!-- Trash -->
          <TrashSection onAlert={(alert) => alertModal = alert} />

          <!-- Onboarding -->
          <div class="space-y-2">
            <div class="flex items-center gap-2 text-[var(--text-primary)]">
              <Sparkles size={16} class="text-[var(--accent-primary)]" />
              <span class="font-medium text-sm">Onboarding</span>
            </div>
            <div class="bg-[var(--bg-elevated)] rounded p-3 flex items-center justify-between gap-3">
              <span class="text-xs text-[var(--text-muted)] leading-relaxed">
                Revisit the first-time setup, import instances from other launchers.
              </span>
              <button
                onclick={() => { onClose(); setShowOnboarding(true) }}
                class="flex-shrink-0 px-2.5 py-1.5 bg-[var(--bg-hover)] hover:bg-[var(--bg-hover)] rounded text-xs font-medium text-[var(--text-primary)] cursor-pointer transition-colors"
              >
                Show onboarding
              </button>
            </div>
          </div>

          <!-- Version Information -->
          <div class="space-y-2">
            <div class="flex items-center gap-2 text-[var(--text-primary)]">
              <Info size={16} class="text-[var(--accent-primary)]" />
              <span class="font-medium text-sm">Version Information</span>
            </div>
            <div class="bg-[var(--bg-elevated)] rounded p-3 space-y-2">
              <div class="flex justify-between text-sm">
                <span class="text-[var(--text-muted)]">Launcher Version</span>
                <span class="text-[var(--text-primary)] font-medium">{semanticVersion || "Loading..."}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    {#if alertModal}
      <AlertModal isOpen={alertModal.isOpen} title={alertModal.title} message={alertModal.message} type={alertModal.type} onClose={() => alertModal = null} />
    {/if}
  {/if}
{/if}
