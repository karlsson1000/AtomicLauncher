<script lang="ts">
  import { invoke } from "@tauri-apps/api/core"
  import { Loader2, ImagePlus, FolderOpen, X, Check, ChevronDown, Paintbrush, Play, AppWindow, HardDrive } from "lucide-svelte"
  import AlertModal from "../../components/ui/AlertModal.svelte"
  import TrashSection from "./TrashSection.svelte"
  import { untrack } from "svelte"
  import { store, setSettings, loadBackground, setShowOnboarding } from "../../lib/launcherStore.svelte"
  import { storeSet } from "../../lib/store"
  import type { LauncherSettings } from "../../types"

  const THEMES = [
    { id: "octane", label: "Octane", colors: ["#15171c", "#252932", "#4572e3", "#e6e6e6"] },
    { id: "light", label: "Light", colors: ["#f5f5f5", "#ffffff", "#4361ee", "#1a1d23"] },
    { id: "rose", label: "Rosé", colors: ["#1a1423", "#2a1a33", "#f472b6", "#e6e6ee"] },
    { id: "cherry", label: "Cherry", colors: ["#1a0d0f", "#2a1417", "#dc2626", "#f4ecec"] },
  ] as const

  type TabId = "appearance" | "game" | "launcher" | "storage"

  const TABS: Array<{ id: TabId; label: string; icon: typeof Paintbrush }> = [
    { id: "appearance", label: "Appearance", icon: Paintbrush },
    { id: "game", label: "Game", icon: Play },
    { id: "launcher", label: "Launcher", icon: AppWindow },
    { id: "storage", label: "Storage", icon: HardDrive },
  ]

  const SECTION_TAB: Record<string, TabId> = {
    memory: "game",
    console: "launcher",
    java: "game",
    appearance: "appearance",
    background: "appearance",
    directory: "storage",
    storage: "storage",
  }

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

  let activeTab = $state<TabId>("appearance")
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
  let javaDropdownUp = $state(false)
  let javaDropdownMaxH = $state(240)

  function toggleJavaDropdown() {
    if (isJavaDropdownOpen) {
      isJavaDropdownOpen = false
      return
    }
    const el = javaDropdownEl
    const scroller = el?.closest(".overflow-y-auto")
    if (el && scroller) {
      const trig = el.getBoundingClientRect()
      const sc = scroller.getBoundingClientRect()
      const spaceBelow = sc.bottom - trig.bottom - 8
      const spaceAbove = trig.top - sc.top - 8
      javaDropdownUp = spaceAbove > spaceBelow
      javaDropdownMaxH = Math.min(240, Math.max(javaDropdownUp ? spaceAbove : spaceBelow, 96))
    }
    isJavaDropdownOpen = true
  }
  let isTabDropdownOpen = $state(false)
  let tabDropdownEl: HTMLDivElement | undefined = $state()
  let saveTimeout: ReturnType<typeof setTimeout> | undefined
  let ramSliderValue = $state(store.settings?.memory_mb ?? 4096)
  let ramTextValue = $state(((store.settings?.memory_mb ?? 4096) / 1024).toFixed(1))
  let ramTextFocused = $state(false)
  let javaArgsText = $state("")
  let javaArgsFocused = $state(false)

  function commitJavaArgsFocus() {
    javaArgsFocused = false
    if (saveTimeout) { clearTimeout(saveTimeout); saveTimeout = undefined }
    const normalized = javaArgsText.replace(/\s+/g, " ").trim()
    handleSettingChange({ ...store.settings!, java_args: normalized === "" ? null : normalized } as LauncherSettings)
    javaArgsText = store.settings?.java_args ?? ""
  }

  $effect(() => {
    if (!ramTextFocused) ramTextValue = (ramSliderValue / 1024).toFixed(1)
  })

  function commitRamText() {
    ramTextFocused = false
    const parsed = parseFloat(ramTextValue.replace(",", "."))
    if (isNaN(parsed)) {
      ramTextValue = (ramSliderValue / 1024).toFixed(1)
      return
    }
    const maxMb = (systemInfo as SystemInfo | null)?.total_memory_mb || 32768
    const mb = Math.min(maxMb, Math.max(1024, Math.round((parsed * 1024) / 512) * 512))
    ramTextValue = (mb / 1024).toFixed(1)
    if (mb === ramSliderValue) return
    ramSliderValue = mb
    handleSettingChangeDebounced({ ...store.settings!, memory_mb: mb } as LauncherSettings)
  }

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
      untrack(() => {
        ramSliderValue = store.settings?.memory_mb ?? 4096
        if (!javaArgsFocused) javaArgsText = store.settings?.java_args ?? ""
        loadSystemInfo()
        loadSidebarBackground()
        loadJavaInstallations()
        loadAppVersion()
        loadStorageUsage()

        const target = store.settingsScrollTarget
        if (target) {
          activeTab = SECTION_TAB[target] ?? "appearance"
          store.settingsScrollTarget = null
          setTimeout(() => {
            document
              .getElementById(`settings-${target}`)
              ?.scrollIntoView({ behavior: "smooth", block: "center" })
          }, 80)
        }
      })
    }

    return () => {
      if (saveTimeout) clearTimeout(saveTimeout)
    }
  })

  $effect(() => {
    const nextJavaArgs = store.settings?.java_args ?? ""
    if (isOpen && !javaArgsFocused && nextJavaArgs !== javaArgsText) {
      javaArgsText = nextJavaArgs
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
      const isCustom = !javaInstallations.includes(store.settings?.java_path as string)
      showCustomPath = isCustom
      if (isCustom) customPathValue = (store.settings?.java_path as string)
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

  function handleJavaArgsChange(value: string) {
    javaArgsText = value
    const normalized = value.replace(/\s+/g, " ").trim()
    handleSettingChangeDebounced({ ...store.settings!, java_args: normalized === "" ? null : normalized } as LauncherSettings)
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
      <div class="bg-[var(--bg-primary)] rounded-lg p-8">
        <div class="flex items-center gap-2 text-[var(--text-muted)]">
          <Loader2 size={20} class="animate-spin" />
          <span>Loading settings...</span>
        </div>
      </div>
    </div>
  {:else}
    <div
      class="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-6 modal-backdrop"
      class:closing={isClosing}
      role="presentation"
      onclick={handleClose}
      onkeydown={(e) => { if (e.key === 'Escape') handleClose() }}
    >
      <div
        role="presentation"
        class="blur-border bg-[var(--bg-primary)] rounded w-full max-w-5xl h-full max-h-[74vh] flex flex-col modal-content overflow-hidden"
        class:closing={isClosing}
        onclick={(e) => e.stopPropagation()}
      >
        <div class="flex items-center justify-between pl-7 pr-9 pt-5 pb-1">
          <h2 class="text-lg font-semibold text-[var(--text-primary)]">Settings</h2>
          <div class="flex items-center gap-3">
            {#if appVersion}
              <span class="text-xs text-[var(--text-muted)] font-mono px-2 py-1 rounded-md bg-[var(--bg-elevated)]">{appVersion.split('-')[1] || appVersion}</span>
            {/if}
            <button onclick={handleClose} class="p-1.5 hover:bg-[var(--bg-hover)] rounded-md transition-colors text-[var(--text-muted)] hover:text-[var(--text-primary)] cursor-pointer">
              <X size={18} />
            </button>
          </div>
        </div>

        <div class="flex flex-1 min-h-0 gap-4 px-7 pb-7 pt-4">
          <nav class="w-40 flex-shrink-0 flex flex-col gap-y-1">
            {#each TABS as tab (tab.id)}
              <button
                onclick={() => { activeTab = tab.id }}
                class="flex items-center gap-3 px-4 py-2.5 rounded-lg text-left text-sm font-medium transition-colors cursor-pointer {activeTab === tab.id
                  ? 'bg-[var(--bg-elevated)] text-[var(--text-primary)]'
                  : 'text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-hover)]'}"
              >
                <tab.icon size={16} strokeWidth={2} class={"flex-shrink-0 " + (activeTab === tab.id ? "text-[var(--accent-primary)]" : "")} />
                <span>{tab.label}</span>
              </button>
            {/each}

            <div class="mt-auto px-4 pt-4">
              <p class="text-xs leading-relaxed text-[var(--text-muted)]">
                Octane Launcher<br />v{semanticVersion || "…"}
              </p>
            </div>
          </nav>

          <div class="flex-1 min-w-0 overflow-y-auto pr-2 space-y-6">
            {#if activeTab === "appearance"}
              <section class="bg-[var(--bg-elevated)] rounded-lg p-4">
                <h4 class="text-base font-semibold text-[var(--text-primary)] mb-1">Theme</h4>
                <p class="text-xs text-[var(--text-muted)] mb-5">Pick the color scheme for the whole launcher.</p>
                <div class="grid grid-cols-4 gap-4">
                  {#each THEMES as theme (theme.id)}
                    <button
                      onclick={() => store.settings && handleSettingChange({ ...store.settings, theme: theme.id })}
                      class="group rounded-lg overflow-hidden text-left cursor-pointer transition-all relative {store.settings?.theme === theme.id
                        ? 'ring-2 ring-[var(--accent-primary)]'
                        : 'hover:brightness-110'}"
                    >
                      <div class="flex h-20" style="background: {theme.colors[0]}">
                        <div class="w-7 h-full flex flex-col gap-1.5 p-1.5" style="background: {theme.colors[1]}">
                          <div class="w-full h-2 rounded-sm" style="background: {theme.colors[2]}"></div>
                          <div class="w-full h-2 rounded-sm opacity-30" style="background: {theme.colors[3]}"></div>
                          <div class="w-full h-2 rounded-sm opacity-30" style="background: {theme.colors[3]}"></div>
                        </div>
                        <div class="flex-1 p-2.5 space-y-1.5">
                          <div class="h-2 w-14 rounded-sm" style="background: {theme.colors[3]}"></div>
                          <div class="h-2 w-20 rounded-sm opacity-30" style="background: {theme.colors[3]}"></div>
                          <div class="h-8 w-full rounded-md mt-2 opacity-15" style="background: {theme.colors[1]}"></div>
                        </div>
                      </div>
                      <div class="flex items-center justify-between px-3.5 py-2.5" style="background: {theme.colors[1]}">
                        <span class="text-xs font-semibold" style="color: {theme.colors[3]}">{theme.label}</span>
                        <div class="w-3.5 h-3.5 rounded-full flex items-center justify-center" style="background: {theme.colors[2]}">
                          {#if store.settings?.theme === theme.id}<Check size={10} class="text-white" strokeWidth={3} />{/if}
                        </div>
                      </div>
                    </button>
                  {/each}
                </div>
              </section>

              <section id="settings-background" class="bg-[var(--bg-elevated)] rounded-lg p-4">
                <h4 class="text-base font-semibold text-[var(--text-primary)] mb-4">Custom background</h4>
                {#if sidebarBgPreview}
                  <div class="relative group">
                    <div class="h-40 rounded-lg overflow-hidden bg-[var(--bg-primary)]">
                      <img src={sidebarBgPreview} alt="Background" class="w-full h-full object-cover" />
                    </div>
                    <div class="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity rounded-lg flex items-center justify-center gap-2">
                      <button onclick={() => fileInputEl?.click()} class="px-3 py-1.5 bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white rounded-lg text-xs font-medium cursor-pointer">Change</button>
                      <button onclick={handleRemoveBackground} class="px-3 py-1.5 bg-red-500 hover:bg-red-600 text-white rounded-lg text-xs font-medium cursor-pointer">Remove</button>
                    </div>
                  </div>
                {:else}
                  <button onclick={() => fileInputEl?.click()} class="w-full h-28 bg-[var(--bg-primary)] hover:bg-[var(--bg-hover)] rounded-lg transition-all flex flex-col items-center justify-center gap-1.5 cursor-pointer">
                    <ImagePlus size={22} class="text-[var(--text-muted)]" />
                    <span class="text-xs text-[var(--text-muted)]">Click to upload an image · PNG, JPG up to 10MB</span>
                  </button>
                {/if}
                <input bind:this={fileInputEl} type="file" accept="image/*" onchange={handleFileSelect} class="hidden" />
              </section>
            {:else if activeTab === "game"}
              <section id="settings-memory" class="bg-[var(--bg-elevated)] rounded-lg p-4">
                <div class="flex items-center justify-between mb-4">
                  <div>
                    <h4 class="text-base font-semibold text-[var(--text-primary)]">Memory allocation</h4>
                    <p class="text-xs text-[var(--text-muted)] mt-0.5">
                      {systemInfo ? `${(systemInfo.available_memory_mb / 1024).toFixed(1)} GB` : "?"} currently available on your system
                    </p>
                  </div>
                  <div class="flex items-baseline">
                    <input
                      type="text"
                      inputmode="decimal"
                      aria-label="Memory allocation in GB"
                      bind:value={ramTextValue}
                      onfocus={() => ramTextFocused = true}
                      onblur={commitRamText}
                      onkeydown={(e) => { if (e.key === 'Enter') (e.currentTarget as HTMLInputElement).blur() }}
                      class="w-[4ch] rounded-md px-1 -mx-1 text-3xl font-bold text-[var(--text-primary)] font-mono text-right cursor-text bg-[var(--bg-primary)] hover:brightness-125 focus:outline-none focus:ring-2 focus:ring-[var(--accent-primary)]"
                    />
                    <span class="text-sm text-[var(--text-muted)] font-sans font-normal ml-1">GB</span>
                  </div>
                </div>
                <div class="relative h-6 flex items-center">
                  <div class="absolute inset-x-0 h-2 bg-[var(--bg-primary)] rounded-full"></div>
                  <div class="absolute h-2 rounded-full" style="width: {ramPercent}%; background: var(--accent-primary)"></div>
                  <div class="absolute w-4 h-4 rounded-full bg-white border-2 border-[var(--accent-primary)] -translate-x-1/2" style="left: {ramPercent}%"></div>
                  <input
                    type="range" min="1024" max={systemInfo?.total_memory_mb || 32768} step="512"
                    bind:value={ramSliderValue}
                    oninput={() => handleSettingChangeDebounced({ ...store.settings!, memory_mb: ramSliderValue } as LauncherSettings)}
                    class="absolute inset-0 w-full opacity-0 cursor-pointer"
                  />
                </div>
              </section>

              <section id="settings-java" class="bg-[var(--bg-elevated)] rounded-lg p-4">
                <div class="flex items-start justify-between gap-6 mb-4">
                  <div class="min-w-0">
                    <h4 class="text-base font-semibold text-[var(--text-primary)]">Java runtime</h4>
                    <p class="text-xs text-[var(--text-muted)] mt-0.5">Which runtime Minecraft runs on.</p>
                  </div>
                  <button onclick={loadJavaInstallations} disabled={isLoadingJava} class="flex-shrink-0 px-4 py-2 text-sm font-medium rounded-lg bg-[var(--bg-primary)] hover:bg-[var(--bg-hover)] text-[var(--text-primary)] cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-1.5">
                    {#if isLoadingJava}<Loader2 size={14} class="animate-spin" />{/if}
                    Rescan
                  </button>
                </div>

                <div class="relative" bind:this={javaDropdownEl}>
                  <button
                    onclick={toggleJavaDropdown}
                    class="w-full bg-[var(--bg-primary)] px-4 py-2 text-sm text-[var(--text-primary)] text-left flex items-center justify-between cursor-pointer min-w-0 rounded-lg hover:bg-[var(--bg-hover)] transition-colors"
                  >
                    <span class="truncate">
                      {showCustomPath ? "Custom Path..." : (store.settings?.java_path || "Auto-detect (Recommended)")}
                    </span>
                    <ChevronDown size={14} class="flex-shrink-0 ml-2 transition-transform {isJavaDropdownOpen ? 'rotate-180' : ''}" />
                  </button>

                  {#if isJavaDropdownOpen}
                    <div class="absolute z-[60] w-full bg-[var(--bg-secondary)] rounded-lg overflow-y-auto custom-scrollbar {javaDropdownUp ? 'bottom-full mb-2' : 'mt-2'}" style="max-height: {javaDropdownMaxH}px">
                      <button
                        onclick={() => { showCustomPath = false; customPathValue = ""; handleSettingChange({ ...store.settings!, java_path: null } as LauncherSettings); isJavaDropdownOpen = false }}
                        class="w-full px-4 py-2 text-sm text-left hover:bg-[var(--bg-hover)] text-[var(--text-primary)] first:rounded-t-xl last:rounded-b-xl flex items-center justify-between cursor-pointer"
                      >
                        <span>Auto-detect (Recommended)</span>
                        {#if !store.settings?.java_path && !showCustomPath}<Check size={14} class="text-[var(--accent-primary)]" />{/if}
                      </button>
                      {#each javaInstallations as path (path)}
                        <button
                          onclick={() => { showCustomPath = false; customPathValue = ""; handleSettingChange({ ...store.settings!, java_path: path } as LauncherSettings); isJavaDropdownOpen = false }}
                          class="w-full px-4 py-2 text-sm text-left hover:bg-[var(--bg-hover)] text-[var(--text-primary)] flex items-center justify-between cursor-pointer"
                        >
                          <span class="truncate">{path}</span>
                          {#if store.settings?.java_path === path && !showCustomPath}<Check size={14} class="text-[var(--accent-primary)] flex-shrink-0 ml-2" />{/if}
                        </button>
                      {/each}
                      <button
                        onclick={() => { showCustomPath = true; customPathValue = store.settings?.java_path || ""; isJavaDropdownOpen = false }}
                        class="w-full px-4 py-2 text-sm text-left hover:bg-[var(--bg-hover)] text-[var(--text-primary)] flex items-center justify-between cursor-pointer"
                      >
                        <span>Custom Path...</span>
                        {#if showCustomPath}<Check size={14} class="text-[var(--accent-primary)]" />{/if}
                      </button>
                    </div>
                  {/if}
                </div>

                {#if showCustomPath}
                  <input
                    type="text"
                    class="w-full bg-[var(--bg-primary)] rounded-lg px-4 py-2 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none focus:ring-2 focus:ring-[var(--accent-primary)] font-mono min-w-0 mt-2"
                    placeholder="C:\\Program Files\\Java\\jdk-21\\bin\\javaw.exe"
                    bind:value={customPathValue}
                    onblur={() => { if (customPathValue.trim()) handleSettingChange({ ...store.settings!, java_path: customPathValue.trim() } as LauncherSettings) }}
                    onkeydown={(e) => { if (e.key === 'Enter' && customPathValue.trim()) { handleSettingChange({ ...store.settings!, java_path: customPathValue.trim() } as LauncherSettings); (e.currentTarget as HTMLInputElement).blur() } }}
                  />
                {/if}

                <div class="mt-5 pt-4 border-t border-[var(--bg-hover)]">
                  <h5 class="text-sm font-semibold text-[var(--text-primary)] mb-1">Custom Java arguments</h5>
                  <p class="text-xs text-[var(--text-muted)] mb-3">
                    Extra JVM flags appended at launch for every instance.
                  </p>
                  <textarea
                    bind:value={javaArgsText}
                    oninput={(e) => handleJavaArgsChange((e.currentTarget as HTMLTextAreaElement).value)}
                    onfocus={() => (javaArgsFocused = true)}
                    onblur={() => commitJavaArgsFocus()}
                    rows={2}
                    spellcheck="false"
                    placeholder="-Xmx4G -XX:+UseG1GC"
                    class="w-full bg-[var(--bg-primary)] rounded-lg px-4 py-2.5 text-xs font-mono text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none resize-y min-h-[60px] max-h-40 custom-scrollbar"
                  ></textarea>
                </div>
              </section>
            {:else if activeTab === "launcher"}
              <section class="bg-[var(--bg-elevated)] rounded-lg p-4 flex items-center justify-between gap-8">
                <div class="min-w-0">
                  <h4 class="text-base font-semibold text-[var(--text-primary)]">Default tab</h4>
                  <p class="text-xs text-[var(--text-muted)] mt-0.5">The tab shown when the launcher opens.</p>
                </div>
                <div class="relative flex-shrink-0" bind:this={tabDropdownEl}>
                  <button
                    onclick={() => isTabDropdownOpen = !isTabDropdownOpen}
                    class="bg-[var(--bg-primary)] hover:bg-[var(--bg-hover)] px-4 py-2 text-sm text-[var(--text-primary)] rounded-lg flex items-center gap-2 cursor-pointer capitalize transition-colors min-w-[140px] justify-between"
                  >
                    {store.settings?.default_tab || "home"}
                    <ChevronDown size={14} class="transition-transform {isTabDropdownOpen ? 'rotate-180' : ''}" />
                  </button>
                  {#if isTabDropdownOpen}
                    <div class="absolute right-0 z-[60] w-full min-w-[160px] bg-[var(--bg-secondary)] rounded-lg overflow-hidden mt-2">
                      {#each ["home", "instances", "addons", "servers", "skins", "screenshots"] as tab (tab)}
                        <button
                          onclick={() => { handleSettingChange({ ...store.settings!, default_tab: tab } as LauncherSettings); isTabDropdownOpen = false }}
                          class="w-full px-4 py-2 text-sm text-left hover:bg-[var(--bg-hover)] text-[var(--text-primary)] flex items-center justify-between cursor-pointer capitalize"
                        >
                          {tab}
                          {#if (store.settings?.default_tab || "home") === tab}<Check size={14} class="text-[var(--accent-primary)]" />{/if}
                        </button>
                      {/each}
                    </div>
                  {/if}
                </div>
              </section>

              <section id="settings-console" class="bg-[var(--bg-elevated)] rounded-lg p-4 flex items-center justify-between gap-8">
                <div class="min-w-0">
                  <h4 class="text-base font-semibold text-[var(--text-primary)]">Auto-navigate to console</h4>
                  <p class="text-xs text-[var(--text-muted)] mt-0.5">Switch to the Console tab when a game launches</p>
                </div>
                <button
                  onclick={() => handleSettingChange({ ...store.settings!, auto_navigate_to_console: !(store.settings?.auto_navigate_to_console ?? true) } as LauncherSettings)}
                  class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors cursor-pointer flex-shrink-0 {(store.settings?.auto_navigate_to_console ?? true) ? 'bg-[var(--accent-primary)]' : 'bg-[var(--bg-hover-strong)]'}"
                  role="switch"
                  aria-checked={(store.settings?.auto_navigate_to_console ?? true)}
                  aria-label="Toggle auto-navigate to console"
                >
                  <span class="inline-block h-5 w-5 transform rounded-full bg-white transition-transform {(store.settings?.auto_navigate_to_console ?? true) ? 'translate-x-[22px]' : 'translate-x-0.5'}"></span>
                </button>
              </section>

              <section class="bg-[var(--bg-elevated)] rounded-lg p-4 flex items-center justify-between gap-8">
                <div class="min-w-0">
                  <h4 class="text-base font-semibold text-[var(--text-primary)]">Cat mode</h4>
                  <p class="text-xs text-[var(--text-muted)] mt-0.5">A cat lays down at the top of the launcher.</p>
                </div>
                <button
                  onclick={() => handleSettingChange({ ...store.settings!, cat_mode: !(store.settings?.cat_mode ?? false) } as LauncherSettings)}
                  class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors cursor-pointer flex-shrink-0 {(store.settings?.cat_mode ?? false) ? 'bg-[var(--accent-primary)]' : 'bg-[var(--bg-hover-strong)]'}"
                  role="switch"
                  aria-checked={(store.settings?.cat_mode ?? false)}
                  aria-label="Toggle cat mode"
                >
                  <span class="inline-block h-5 w-5 transform rounded-full bg-white transition-transform {(store.settings?.cat_mode ?? false) ? 'translate-x-[22px]' : 'translate-x-0.5'}"></span>
                </button>
              </section>

              <section class="bg-[var(--bg-elevated)] rounded-lg p-4 flex items-center justify-between gap-8">
                <div class="min-w-0 pr-4">
                  <h4 class="text-base font-semibold text-[var(--text-primary)]">Run onboarding again</h4>
                  <p class="text-xs text-[var(--text-muted)] mt-0.5">Revisit first-time setup or import instances from other launchers.</p>
                </div>
                <button
                  onclick={() => { onClose(); setShowOnboarding(true) }}
                  class="flex-shrink-0 px-4 py-2 text-sm font-medium rounded-lg bg-[var(--bg-primary)] hover:bg-[var(--bg-hover)] text-[var(--text-primary)] cursor-pointer transition-colors"
                >
                  Show onboarding
                </button>
              </section>
            {:else if activeTab === "storage"}
              <section id="settings-storage" class="bg-[var(--bg-elevated)] rounded-lg p-4">
                <h4 class="text-base font-semibold text-[var(--text-primary)] mb-4">Disk usage</h4>
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
                  <div class="flex flex-wrap gap-x-5 gap-y-1.5 mt-3">
                    {#each storageCategories as cat (cat.name)}
                      <div class="flex items-center gap-1.5 text-xs">
                        <div class="w-2.5 h-2.5 rounded-sm" style="background-color: {storageColors[cat.name] || '#6b7280'}"></div>
                        <span class="text-[var(--text-muted)]">{cat.name}</span>
                        <span class="text-[var(--text-primary)] font-medium">{formatBytes(cat.size_bytes)}</span>
                      </div>
                    {/each}
                  </div>
                {/if}
              </section>

              <section id="settings-directory" class="bg-[var(--bg-elevated)] rounded-lg p-4 flex items-center justify-between gap-8">
                <div class="min-w-0">
                  <h4 class="text-base font-semibold text-[var(--text-primary)]">Game directory</h4>
                  <p class="text-xs text-[var(--text-muted)] font-mono break-all mt-0.5">{store.launcherDirectory || "Loading..."}</p>
                </div>
                <button
                  onclick={() => handleOpenDirectory(store.launcherDirectory)}
                  disabled={!store.launcherDirectory}
                  class="flex-shrink-0 px-4 py-2 text-sm font-medium rounded-lg bg-[var(--bg-primary)] hover:bg-[var(--bg-hover)] disabled:opacity-50 text-[var(--text-primary)] cursor-pointer disabled:cursor-not-allowed flex items-center gap-1.5 transition-colors"
                >
                  <FolderOpen size={14} />
                  Open folder
                </button>
              </section>

              <section class="bg-[var(--bg-elevated)] rounded-lg p-4">
                <TrashSection onAlert={(alert) => alertModal = alert} />
              </section>
            {/if}
          </div>
        </div>
      </div>
    </div>

    {#if alertModal}
      <AlertModal isOpen={alertModal.isOpen} title={alertModal.title} message={alertModal.message} type={alertModal.type} onClose={() => alertModal = null} />
    {/if}
  {/if}
{/if}