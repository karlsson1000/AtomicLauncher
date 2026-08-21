<script lang="ts">
  import { X, Download, Loader2 } from "lucide-svelte"
  import { invoke } from "@tauri-apps/api/core"
  import { Marked } from "marked"
  import type {
    Instance, ModrinthProjectDetails, ModrinthVersion, ModFile,
    CurseforgeModDetail, CurseforgeFile, CurseforgeGetModFilesResult
  } from "../../types"
  import { getMinecraftVersion } from "../../lib/version"

  let {
    source,
    projectId,
    projectSlug = "",
    author = "",
    projectType,
    selectedInstance,
    instances = [],
    onBack,
    onShowCreationToast = undefined as ((instanceName: string) => void) | undefined,
    onRefreshInstances = undefined as (() => void) | undefined,
  }: {
    source: "modrinth" | "curseforge"
    projectId: string
    projectSlug?: string
    author?: string
    projectType: string
    selectedInstance: Instance | null
    instances?: Instance[]
    onBack: () => void
    onShowCreationToast?: (instanceName: string) => void
    onRefreshInstances?: () => void
  } = $props()

  const isModpack = $derived(projectType === "modpack")

  const marked = new Marked({
    breaks: true,
    gfm: true,
  })

  let details = $state<ModrinthProjectDetails | null>(null)
  let curseforgeDetails = $state<CurseforgeModDetail | null>(null)
  let isLoading = $state(true)
  let error = $state<string | null>(null)

  let versions = $state<ModrinthVersion[]>([])
  let curseforgeFiles = $state<CurseforgeFile[]>([])
  let isLoadingVersions = $state(false)

  let installedFiles = $state<Set<string>>(new Set())
  let downloadingMap = $state<Set<string>>(new Set())
  let completedIds = $state<Set<string>>(new Set())

  function uniqueInstanceName(name: string): string {
    return instances.some(i => i.name === name) ? `${name}-${Date.now()}` : name
  }

  let renderedBody = $state("")

  let bodyHtml = $derived.by(() => {
    if (renderedBody) return renderedBody
    if (curseforgeDetails?.description) return curseforgeDetails.description
    return ""
  })

    let spinnerColor = $derived.by(() => {
    switch (projectType) {
      case "mod": return "text-[#16a34a]"
      case "modpack": return "text-[#3b82f6]"
      case "resourcepack": return "text-[#8b5cf6]"
      case "shaderpack": return "text-[#f59e0b]"
      default: return "text-[var(--text-muted)]"
    }
  })

  $effect(() => {
    fetchDetails()
    loadInstalledFiles()
  })

  async function fetchDetails() {
    isLoading = true
    error = null
    try {
      if (source === "modrinth") {
        const idOrSlug = projectSlug || projectId
        const result = await invoke<ModrinthProjectDetails>("get_project_details", { idOrSlug })
        details = result
        if (result.body) {
          renderedBody = await marked.parse(result.body)
        }
        await fetchVersions()
      } else {
        const result = await invoke<CurseforgeModDetail>("get_curseforge_mod_details", { modId: parseInt(projectId) })
        curseforgeDetails = result
        await fetchCurseforgeFiles()
      }
    } catch (e) {
      console.error("Failed to fetch project details:", e)
      error = String(e)
    } finally {
      isLoading = false
    }
  }

  async function fetchVersions() {
    if (!details) return
    isLoadingVersions = true
    try {
      const loaders = selectedInstance && (selectedInstance.loader === "fabric" || selectedInstance.loader === "neoforge" || selectedInstance.loader === "forge")
        ? [selectedInstance.loader]
        : undefined
      const gameVersions = selectedInstance
        ? [getMinecraftVersion(selectedInstance)]
        : undefined
      let result = await invoke<ModrinthVersion[]>("get_mod_versions", {
        idOrSlug: details.id,
        loaders: projectType === "mod" ? loaders : undefined,
        gameVersions: projectType === "mod" || projectType === "modpack" ? gameVersions : undefined,
      })
      if (projectType === "modpack" || projectType === "resourcepack" || projectType === "shaderpack") {
        const seen = new Set<string>()
        result = result.filter(v => {
          const key = v.version_number || v.name
          if (seen.has(key)) return false
          seen.add(key)
          return true
        })
      }
      versions = result
    } catch (e) {
      console.error("Failed to fetch versions:", e)
    } finally {
      isLoadingVersions = false
    }
  }

  async function fetchCurseforgeFiles() {
    if (!curseforgeDetails) return
    isLoadingVersions = true
    try {
      const result = await invoke<CurseforgeGetModFilesResult>("get_curseforge_mod_files", {
        modId: curseforgeDetails.id,
        gameVersion: null,
        modLoaderType: null,
        pageSize: 20,
      })
      curseforgeFiles = result.data
    } catch (e) {
      console.error("Failed to fetch CurseForge files:", e)
    } finally {
      isLoadingVersions = false
    }
  }

  async function loadInstalledFiles() {
    if (!selectedInstance) return
    try {
      if (projectType === "mod") {
        const mods = await invoke<ModFile[]>("get_installed_mods", { instanceName: selectedInstance.name })
        installedFiles = new Set(mods.map(m => m.filename))
      } else if (projectType === "resourcepack") {
        const packs = await invoke<string[]>("get_installed_resourcepacks", { instanceName: selectedInstance.name })
        installedFiles = new Set(packs)
      } else if (projectType === "shaderpack") {
        const packs = await invoke<string[]>("get_installed_shaderpacks", { instanceName: selectedInstance.name })
        installedFiles = new Set(packs)
      }
    } catch (e) {
      console.error("Failed to load installed files:", e)
    }
  }

  function formatGameVersions(versions: string[]): string {
    return versions.slice(0, 3).join(", ")
  }

  function isInstalled(filename: string): boolean {
    return installedFiles.has(filename)
  }

  async function handleInstallVersion(version: ModrinthVersion) {
    if (!isModpack && !selectedInstance) return
    const primaryFile = version.files.find(f => f.primary) || version.files[0]
    if (!primaryFile && !isModpack) return
    downloadingMap = new Set(downloadingMap).add(version.id)
    try {
      if (source === "modrinth") {
        if (isModpack) {
          const instanceName = uniqueInstanceName(details?.title || version.name)
          onShowCreationToast?.(instanceName)
          await invoke("install_modpack", {
            modpackSlug: details!.id,
            instanceName,
            versionId: version.id,
            preferredGameVersion: null,
          })
          if (onRefreshInstances) setTimeout(() => onRefreshInstances!(), 500)
          completedIds = new Set(completedIds).add(version.id)
        } else {
          const targetCommand = projectType === "resourcepack" ? "download_resourcepack" : projectType === "shaderpack" ? "download_shaderpack" : "download_mod"
          await invoke<string>(targetCommand, {
            instanceName: selectedInstance!.name,
            downloadUrl: primaryFile!.url,
            filename: primaryFile!.filename,
          })
          installedFiles = new Set(installedFiles).add(primaryFile!.filename)
        }
      }
    } catch (e) {
      console.error("Download error:", e)
    } finally {
      const n = new Set(downloadingMap)
      n.delete(version.id)
      downloadingMap = n
    }
  }

  async function handleInstallCurseforgeFile(file: CurseforgeFile) {
    if (!file.downloadUrl) return
    if (!isModpack && !selectedInstance) return
    downloadingMap = new Set(downloadingMap).add(file.id.toString())
    try {
      if (isModpack) {
        const filePath = await invoke<string>("download_curseforge_file_temp", {
          downloadUrl: file.downloadUrl,
          filename: file.fileName,
        })
        const instanceName = uniqueInstanceName(curseforgeDetails?.name || file.fileName)
        onShowCreationToast?.(instanceName)
        await invoke("install_modpack_from_file", {
          filePath,
          instanceName,
          preferredGameVersion: null,
        })
        if (onRefreshInstances) setTimeout(() => onRefreshInstances!(), 500)
        completedIds = new Set(completedIds).add(file.id.toString())
      } else {
        const targetFolder = projectType === "resourcepack" ? "resourcepacks" : projectType === "shaderpack" ? "shaderpacks" : "mods"
        await invoke<string>("download_curseforge_file", {
          instanceName: selectedInstance!.name,
          downloadUrl: file.downloadUrl,
          filename: file.fileName,
          targetFolder,
        })
        installedFiles = new Set(installedFiles).add(file.fileName)
      }
    } catch (e) {
      console.error("Download error:", e)
    } finally {
      const n = new Set(downloadingMap)
      n.delete(file.id.toString())
      downloadingMap = n
    }
  }

  function getIconUrl(): string | null {
    if (details?.icon_url) return details.icon_url
    if (curseforgeDetails?.logo?.thumbnailUrl) return curseforgeDetails.logo.thumbnailUrl
    return null
  }

  function getTitle(): string {
    if (details?.title) return details.title
    if (curseforgeDetails?.name) return curseforgeDetails.name
    return ""
  }

  function getAuthor(): string {
    if (author) return author
    if (curseforgeDetails?.authors?.[0]?.name) return curseforgeDetails.authors[0].name
    return ""
  }

  function showInstallSection(): boolean {
    return true
  }

  function getProjectUrl(): string {
    if (source === "modrinth") {
      return `https://modrinth.com/project/${projectSlug || projectId}`
    }
    const classPath = projectType === "modpack" ? "modpacks"
      : projectType === "resourcepack" ? "texture-packs"
      : projectType === "shaderpack" ? "shaders"
      : "mc-mods"
    return `https://www.curseforge.com/minecraft/${classPath}/${projectSlug}`
  }

  function openProjectUrl(e: MouseEvent) {
    e.preventDefault()
    invoke("open_url", { url: getProjectUrl() }).catch(() => {})
  }
</script>

<div class="h-full relative overflow-hidden">
  {#if isLoading}
    <div class="flex items-center justify-center h-full">
      <Loader2 size={32} class="animate-spin {spinnerColor}" />
    </div>
  {:else if error}
    <div class="flex items-center justify-center h-full">
      <div class="text-center">
        <p class="text-red-400 mb-2">Failed to load project details</p>
        <p class="text-sm text-[var(--text-muted)]">{error}</p>
      </div>
    </div>
  {:else}
    <div class="h-full max-w-7xl mx-auto grid grid-cols-1 lg:grid-cols-11 gap-2">
      <!-- Left: Header + Description -->
      <div class="{showInstallSection() ? 'lg:col-span-8' : 'lg:col-span-11'} overflow-y-auto pr-2">
      <div class="relative space-y-4 bg-[var(--bg-tertiary)] rounded-md p-4">
        <button
          onclick={onBack}
          class="absolute top-2 right-2 z-20 flex items-center justify-center w-9 h-9 rounded-full bg-[var(--bg-tertiary)] text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-hover)] transition-colors cursor-pointer"
        >
          <X size={18} />
        </button>
        <!-- Header + Stats -->
        <div class="flex gap-4 items-center">
          {#if getIconUrl()}
            <img src={getIconUrl()} alt={getTitle()} class="w-16 h-16 rounded-md object-contain flex-shrink-0 bg-[var(--bg-tertiary)]" />
          {:else}
            <div class="w-16 h-16 rounded-md bg-gradient-to-br from-[#16a34a]/10 to-[#22c55e]/10 flex items-center justify-center flex-shrink-0"></div>
          {/if}
          <div class="flex-1 min-w-0">
            <h1 class="text-3xl font-bold text-[var(--text-primary)] truncate">
              <a href={getProjectUrl()} onclick={openProjectUrl} class="hover:underline">
                {getTitle()}
              </a>
            </h1>
            {#if getAuthor()}
              <p class="text-lg text-[var(--text-muted)]">by {getAuthor()}</p>
            {/if}
          </div>
        </div>

        <!-- Description -->
        {#if bodyHtml}
          <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
          <div class="prose-custom markdown-body" onclick={(e) => {
            const target = e.target as HTMLElement
            const anchor = target.closest('a')
            if (anchor?.href && anchor.href.startsWith('http')) {
              e.preventDefault()
              invoke('open_url', { url: anchor.href }).catch(() => {})
            }
          }}>
            {@html bodyHtml}
          </div>
        {/if}
      </div>
      </div>

      <!-- Right: Versions sidebar -->
      {#if showInstallSection()}
        <div class="lg:col-span-3 overflow-y-auto">
          <div class="mb-3">
            <h2 class="text-lg font-semibold text-[var(--text-primary)] px-3">Versions</h2>
          </div>

          {#if !isModpack && !selectedInstance}
            <p class="text-sm text-[var(--text-muted)]">Select an instance to install</p>
          {:else if isLoadingVersions}
            <div class="flex items-center justify-center py-8">
              <Loader2 size={20} class="animate-spin text-[#16a34a]" />
            </div>
          {:else if source === "modrinth" && versions.length === 0}
            <p class="text-sm text-[var(--text-muted)]">No compatible versions found</p>
          {:else if source === "curseforge" && curseforgeFiles.length === 0}
            <p class="text-sm text-[var(--text-muted)]">No files available</p>
          {:else}
            <div class="space-y-2 pr-2">
              {#if source === "modrinth"}
                {#each versions as version (version.id)}
                  {@const installed = isInstalled(version.files.find(f => f.primary)?.filename || version.files[0]?.filename || "")}
                  {@const downloading = downloadingMap.has(version.id)}
                  {@const done = completedIds.has(version.id)}
                  <div class="bg-[var(--bg-tertiary)] rounded-md p-3 flex items-center justify-between gap-2">
                    <div class="flex-1 min-w-0">
                      <div class="text-sm font-medium text-[var(--text-primary)] truncate">{version.name}</div>
                      <div class="text-xs text-[var(--text-muted)] mt-0.5">
                        {version.version_type === "release" ? "Release" : version.version_type === "beta" ? "Beta" : "Alpha"}
                        {#if version.loaders.filter(l => l !== "minecraft" && l !== "iris" && l !== "optifine").length > 0}
                          &middot; {version.loaders.filter(l => l !== "minecraft" && l !== "iris" && l !== "optifine").join(", ")}
                        {/if}
                        &middot; {formatGameVersions(version.game_versions)}
                      </div>
                    </div>
                    <button
                      onclick={() => handleInstallVersion(version)}
                      disabled={downloading || installed || done || (!isModpack && !selectedInstance)}
                      class="px-3 py-2 bg-[#16a34a] hover:bg-[#22c55e] disabled:opacity-50 disabled:cursor-not-allowed text-white rounded text-xs font-medium whitespace-nowrap transition-all cursor-pointer flex items-center gap-1 flex-shrink-0"
                    >
                      {#if downloading}
                        <Loader2 size={14} class="animate-spin" />
                      {:else if installed || done}
                        Installed
                      {:else}
                        <Download size={14} />Install
                      {/if}
                    </button>
                  </div>
                {/each}
              {:else}
                {#each curseforgeFiles as file (file.id)}
                  {@const installed = isInstalled(file.fileName)}
                  {@const downloading = downloadingMap.has(file.id.toString())}
                  {@const done = completedIds.has(file.id.toString())}
                  <div class="bg-[var(--bg-tertiary)] rounded-md p-3 flex items-center justify-between gap-2">
                    <div class="flex-1 min-w-0">
                      <div class="text-sm font-medium text-[var(--text-primary)] truncate">{file.fileName}</div>
                      <div class="text-xs text-[var(--text-muted)] mt-0.5">
                        {file.releaseType === 1 ? "Release" : file.releaseType === 2 ? "Beta" : "Alpha"}
                        &middot; {(file.fileLength / 1024 / 1024).toFixed(1)} MB
                      </div>
                    </div>
                    <button
                      onclick={() => handleInstallCurseforgeFile(file)}
                      disabled={downloading || installed || done || !file.downloadUrl || (!isModpack && !selectedInstance)}
                      class="px-3 py-2 bg-[#16a34a] hover:bg-[#22c55e] disabled:opacity-50 disabled:cursor-not-allowed text-white rounded text-xs font-medium whitespace-nowrap transition-all cursor-pointer flex items-center gap-1 flex-shrink-0"
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
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>
