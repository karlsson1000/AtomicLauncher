import { useState, useEffect } from "react"
import { Play, FolderOpen, Package, Loader2, ExternalLink, Globe, Settings, Trash2, RefreshCw } from "lucide-react"
import { invoke } from "@tauri-apps/api/core"
import { ConfirmModal, AlertModal } from "./ConfirmModal"
import { InstanceSettingsModal } from "./InstanceSettingsModal"
import type { Instance } from "../../types"

interface InstalledMod {
  filename: string
  size: number
  name?: string
  description?: string
  icon_url?: string
  downloads?: number
  author?: string
  disabled?: boolean
  project_id?: string
  current_version_id?: string
}

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

interface InstanceDetailsTabProps {
  instance: Instance
  isAuthenticated: boolean
  isLaunching: boolean
  isRunning: boolean
  onLaunch: () => void
  onBack: () => void
  onInstanceUpdated: () => void
}

export function InstanceDetailsTab({
  instance,
  isAuthenticated,
  isLaunching,
  isRunning,
  onLaunch,
  onBack,
  onInstanceUpdated,
}: InstanceDetailsTabProps) {
  const [installedMods, setInstalledMods] = useState<InstalledMod[]>([])
  const [worlds, setWorlds] = useState<World[]>([])
  const [isLoadingMods, setIsLoadingMods] = useState(true)
  const [isLoadingWorlds, setIsLoadingWorlds] = useState(true)
  const [instanceIcon, setInstanceIcon] = useState<string | null>(null)
  const [isSettingsOpen, setIsSettingsOpen] = useState(false)
  const [availableUpdates, setAvailableUpdates] = useState<ModUpdate[]>([])
  const [isCheckingUpdates, setIsCheckingUpdates] = useState(false)
  const [isUpdatingMods, setIsUpdatingMods] = useState(false)
  const [confirmModal, setConfirmModal] = useState<{
    isOpen: boolean
    title: string
    message: string
    type: "warning" | "danger" | "success" | "info"
    onConfirm: () => void
  } | null>(null)
  const [alertModal, setAlertModal] = useState<{
    isOpen: boolean
    title: string
    message: string
    type: "warning" | "danger" | "success" | "info"
  } | null>(null)

  useEffect(() => {
    loadInstalledMods()
    loadWorlds()
    loadInstanceIcon()
  }, [instance.name])

  useEffect(() => {
    loadInstanceIcon()
  }, [instance])

  const loadInstanceIcon = async () => {
    try {
      const icon = await invoke<string | null>("get_instance_icon", {
        instanceName: instance.name
      })
      setInstanceIcon(icon)
    } catch (error) {
      console.error("Failed to load instance icon:", error)
      setInstanceIcon(null)
    }
  }

  const loadWorlds = async () => {
    setIsLoadingWorlds(true)
    try {
      const worldsList = await invoke<World[]>("get_instance_worlds", {
        instanceName: instance.name
      })
      setWorlds(worldsList)
    } catch (error) {
      console.error("Failed to load worlds:", error)
      setWorlds([])
    } finally {
      setIsLoadingWorlds(false)
    }
  }

  const loadInstalledMods = async () => {
    setIsLoadingMods(true)
    try {
      const mods = await invoke<InstalledMod[]>("get_installed_mods", {
        instanceName: instance.name
      })
      
      const modsWithMetadata = await Promise.all(
        mods.map(async (mod) => {
          const isDisabled = mod.filename.endsWith('.disabled')
          const actualFilename = isDisabled ? mod.filename.replace('.disabled', '') : mod.filename
          
          try {
            let slug = actualFilename
              .replace(/\.jar$/i, '')
              .toLowerCase()
            
            slug = slug
              .replace(/[-_]((\d+\.)+\d+)[-_+]?.*$/i, '')
              .replace(/[-_]v?((\d+\.)+\d+)$/i, '')
              .replace(/[-_]mc((\d+\.)+\d+)$/i, '')
              .replace(/[-_](forge|fabric|quilt|neoforge)$/i, '')
              .replace(/[-_]\d+$/i, '')
              .trim()
            
            if (!slug) {
              return { ...mod, disabled: isDisabled }
            }
            
            const facets = JSON.stringify([["project_type:mod"]])
            let result = await invoke<any>("search_mods", {
              query: slug,
              facets,
              index: "relevance",
              offset: 0,
              limit: 20,
            })
            
            if (result.hits && result.hits.length > 0) {
              const slugNormalized = slug.replace(/[-_\s]/g, '').toLowerCase()
              let bestMatch = result.hits.find((hit: any) => {
                const hitSlug = hit.slug.toLowerCase().replace(/[-_\s]/g, '')
                const hitTitle = hit.title.toLowerCase().replace(/[-_\s]/g, '')
                return hitSlug === slugNormalized || hitTitle === slugNormalized
              })
              
              if (bestMatch) {
                const mcVersion = getMinecraftVersion(instance)
                try {
                  const versions = await invoke<any[]>("get_mod_versions", {
                    idOrSlug: bestMatch.project_id,
                    loaders: [instance.loader],
                    gameVersions: [mcVersion],
                  })
                  
                  if (versions && versions.length > 0) {
                    const currentVersion = versions.find((v: any) => 
                      v.files.some((f: any) => f.filename === actualFilename)
                    )
                    
                    return {
                      ...mod,
                      disabled: isDisabled,
                      name: bestMatch.title,
                      description: bestMatch.description,
                      icon_url: bestMatch.icon_url,
                      downloads: bestMatch.downloads,
                      author: bestMatch.author,
                      project_id: bestMatch.project_id,
                      current_version_id: currentVersion?.id,
                    }
                  }
                } catch (versionError) {
                  console.error(`Failed to fetch versions for ${bestMatch.title}:`, versionError)
                }
                
                return {
                  ...mod,
                  disabled: isDisabled,
                  name: bestMatch.title,
                  description: bestMatch.description,
                  icon_url: bestMatch.icon_url,
                  downloads: bestMatch.downloads,
                  author: bestMatch.author,
                  project_id: bestMatch.project_id,
                }
              }
            }
          } catch (error) {
            console.error(`Failed to fetch metadata for ${mod.filename}:`, error)
          }
          return { ...mod, disabled: isDisabled }
        })
      )
      
      setInstalledMods(modsWithMetadata)
    } catch (error) {
      console.error("Failed to load installed mods:", error)
      setInstalledMods([])
    } finally {
      setIsLoadingMods(false)
    }
  }

  const checkForUpdates = async () => {
    if (!instance || instance.loader !== "fabric") return
    
    setIsCheckingUpdates(true)
    const updates: ModUpdate[] = []
    
    try {
      const mcVersion = getMinecraftVersion(instance)
      
      for (const mod of installedMods) {
        if (!mod.project_id || !mod.current_version_id || mod.disabled) continue
        
        try {
          const versions = await invoke<any[]>("get_mod_versions", {
            idOrSlug: mod.project_id,
            loaders: [instance.loader],
            gameVersions: [mcVersion],
          })
          
          if (versions && versions.length > 0) {
            const latestVersion = versions[0]
            
            if (latestVersion.id !== mod.current_version_id) {
              const primaryFile = latestVersion.files.find((f: any) => f.primary) || latestVersion.files[0]
              
              if (primaryFile) {
                updates.push({
                  filename: mod.filename,
                  projectId: mod.project_id,
                  currentVersionId: mod.current_version_id,
                  latestVersion: {
                    id: latestVersion.id,
                    name: latestVersion.name,
                    version_number: latestVersion.version_number,
                    downloadUrl: primaryFile.url,
                    filename: primaryFile.filename,
                  }
                })
              }
            }
          }
        } catch (error) {
          console.error(`Failed to check updates for ${mod.name || mod.filename}:`, error)
        }
      }
      
      setAvailableUpdates(updates)
    } catch (error) {
      console.error("Failed to check for updates:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to check for updates: ${error}`,
        type: "danger"
      })
    } finally {
      setIsCheckingUpdates(false)
    }
  }

  const updateAllMods = async () => {
    if (availableUpdates.length === 0) return
    
    setIsUpdatingMods(true)
    let successCount = 0
    let failCount = 0
    
    for (const update of availableUpdates) {
      try {
        await invoke("download_mod", {
          instanceName: instance.name,
          downloadUrl: update.latestVersion.downloadUrl,
          filename: update.latestVersion.filename,
        })
        
        if (update.filename !== update.latestVersion.filename) {
          try {
            await invoke("delete_mod", {
              instanceName: instance.name,
              filename: update.filename
            })
          } catch (deleteError) {
            console.error(`Failed to delete old version ${update.filename}:`, deleteError)
          }
        }
        
        successCount++
      } catch (error) {
        console.error(`Failed to update mod ${update.filename}:`, error)
        failCount++
      }
    }
    
    setIsUpdatingMods(false)
    setAvailableUpdates([])
    
    await loadInstalledMods()
  }

  const formatFileSize = (bytes: number): string => {
    if (bytes >= 1024 * 1024) {
      return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
    }
    if (bytes >= 1024) {
      return `${(bytes / 1024).toFixed(1)} KB`
    }
    return `${bytes} B`
  }

  const formatPlaytime = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600)
    const minutes = Math.floor((seconds % 3600) / 60)
    
    if (hours > 0) {
      return `${hours}h ${minutes}m played`
    }
    if (minutes > 0) {
      return `${minutes}m played`
    }
    return `${seconds}s played`
  }

  const formatDate = (timestamp?: number): string => {
    if (!timestamp) return "Unknown"
    const date = new Date(timestamp * 1000)
    
    const year = date.getFullYear()
    const month = String(date.getMonth() + 1).padStart(2, '0')
    const day = String(date.getDate()).padStart(2, '0')
    
    let hours = date.getHours()
    const minutes = String(date.getMinutes()).padStart(2, '0')
    const ampm = hours >= 12 ? 'PM' : 'AM'
    hours = hours % 12
    hours = hours ? hours : 12
    
    return `${year}-${month}-${day} at ${hours}:${minutes} ${ampm}`
  }

  const getMinecraftVersion = (instance: Instance): string => {
    if (instance.loader === "fabric") {
      const parts = instance.version.split('-')
      return parts[parts.length - 1]
    }
    return instance.version
  }

  const getFabricLoaderVersion = (instance: Instance): string | null => {
    if (instance.loader === "fabric") {
      const parts = instance.version.split('-')
      if (parts.length >= 2) {
        return parts[parts.length - 2]
      }
    }
    return null
  }

  const handleOpenFolder = async () => {
    try {
      await invoke("open_instance_folder", { instanceName: instance.name })
    } catch (error) {
      console.error("Failed to open folder:", error)
    }
  }

  const handleDeleteMod = async (filename: string) => {
    try {
      await invoke("delete_mod", {
        instanceName: instance.name,
        filename
      })
      await loadInstalledMods()
      setAvailableUpdates([])
    } catch (error) {
      console.error("Failed to delete mod:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to delete mod: ${error}`,
        type: "danger"
      })
    }
  }

  const handleDeleteWorld = async (folderName: string, worldName: string) => {
    setConfirmModal({
      isOpen: true,
      title: "Delete World",
      message: `Are you sure you want to delete "${worldName}"?\n\nThis action cannot be undone.`,
      type: "danger",
      onConfirm: async () => {
        setConfirmModal(null)
        try {
          await invoke("delete_world", {
            instanceName: instance.name,
            folderName
          })
          await loadWorlds()
        } catch (error) {
          console.error("Failed to delete world:", error)
          setAlertModal({
            isOpen: true,
            title: "Error",
            message: `Failed to delete world: ${error}`,
            type: "danger"
          })
        }
      }
    })
  }

  const handleToggleMod = async (mod: InstalledMod) => {
    try {
      await invoke("toggle_mod", {
        instanceName: instance.name,
        filename: mod.filename,
        disable: !mod.disabled
      })
      await loadInstalledMods()
    } catch (error) {
      console.error("Failed to toggle mod:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to ${mod.disabled ? 'enable' : 'disable'} mod: ${error}`,
        type: "danger"
      })
    }
  }

  const handleOpenWorldsFolder = async () => {
    try {
      await invoke("open_worlds_folder", { instanceName: instance.name })
    } catch (error) {
      console.error("Failed to open worlds folder:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to open worlds folder: ${error}`,
        type: "danger"
      })
    }
  }

  const handleOpenWorldFolder = async (folderName: string) => {
    try {
      await invoke("open_world_folder", {
        instanceName: instance.name,
        folderName
      })
    } catch (error) {
      console.error("Failed to open world folder:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to open world folder: ${error}`,
        type: "danger"
      })
    }
  }

  const handleInstanceUpdated = () => {
    loadInstanceIcon()
    onInstanceUpdated()
  }

  const handleInstanceDeleted = () => {
    onBack()
    onInstanceUpdated()
  }

  const fabricLoaderVersion = getFabricLoaderVersion(instance)
  const modsWithProjectId = installedMods.filter(mod => mod.project_id && !mod.disabled).length

  return (
    <>
      <div className="p-6 space-y-4">
        <div className="max-w-7xl mx-auto">
          <div className="flex items-center gap-4 mb-6">
            <div className="flex-shrink-0">
              {instanceIcon ? (
                <div className="w-16 h-16 rounded-md overflow-hidden border border-[#2a2a2a]">
                  <img
                    src={instanceIcon}
                    alt={instance.name}
                    className="w-full h-full object-cover"
                  />
                </div>
              ) : (
                <div className="w-16 h-16 border border-[#2a2a2a] rounded-md flex items-center justify-center bg-[#0f0f0f]">
                  <Package size={24} className="text-[#3a3a3a]" strokeWidth={1.5} />
                </div>
              )}
            </div>

            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-3">
                    <h1 className="text-2xl font-semibold text-[#e6edf3] tracking-tight leading-tight">{instance.name}</h1>
                    {(instance.total_playtime_seconds ?? 0) > 0 && (
                      <span className="px-2 py-0.5 bg-[#141414] border border-[#2a2a2a] text-[#7d8590] text-xs rounded">
                        {formatPlaytime(instance.total_playtime_seconds ?? 0)}
                      </span>
                    )}
                  </div>
                  <p className="text-sm text-[#7d8590] mt-1">
                    Minecraft {getMinecraftVersion(instance)}
                    {" • "}
                    {instance.loader === "fabric" ? (
                      <>
                        <span className="text-[#3b82f6]">Fabric Loader</span>
                        {fabricLoaderVersion && (
                          <span className="text-[#7d8590]"> {fabricLoaderVersion}</span>
                        )}
                      </>
                    ) : (
                      <span className="text-[#238636]">Vanilla</span>
                    )}
                  </p>
                </div>
                <div className="flex gap-2">
                  <button
                    onClick={onLaunch}
                    disabled={!isAuthenticated || isLaunching || isRunning}
                    className={`px-6 py-2.5 rounded-md font-medium text-sm flex items-center gap-2 transition-all cursor-pointer ${
                      isLaunching || isRunning
                        ? "bg-red-500/10 text-red-400"
                        : "bg-[#238636]/15 hover:bg-[#238636]/25 text-[#2ea043]"
                    } disabled:opacity-50 disabled:cursor-not-allowed`}
                  >
                    {isLaunching || isRunning ? (
                      <>
                        <div className="w-4 h-4 border-2 border-red-400/30 border-t-red-400 rounded-full animate-spin" />
                        <span>Running...</span>
                      </>
                    ) : (
                      <>
                        <Play size={18} fill="currentColor" strokeWidth={0} />
                        <span>Play</span>
                      </>
                    )}
                  </button>
                  <button
                    onClick={handleOpenFolder}
                    className="px-4 py-2.5 bg-[#141414] hover:bg-[#1a1a1a] border border-[#2a2a2a] text-[#e6edf3] rounded-md font-medium text-sm flex items-center gap-2 transition-all cursor-pointer"
                  >
                    <FolderOpen size={16} />
                    <span>Open Folder</span>
                  </button>
                  <button
                    onClick={() => setIsSettingsOpen(true)}
                    className="px-4 py-2.5 bg-[#141414] hover:bg-[#1a1a1a] border border-[#2a2a2a] text-[#e6edf3] rounded-md font-medium text-sm flex items-center gap-2 transition-all cursor-pointer"
                    title="Instance Settings"
                  >
                    <Settings size={18} />
                  </button>
                </div>
              </div>
            </div>
          </div>

          <div className="border-t border-[#2a2a2a] my-6"></div>

          <div className="grid grid-cols-2 gap-0 relative">
            <div className="pr-6 border-r border-[#2a2a2a]">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-3">
                  <h2 className="text-lg font-semibold text-[#e6edf3] tracking-tight">Installed Mods</h2>
                  <span className="px-2 py-0.5 bg-[#141414] border border-[#2a2a2a] text-[#7d8590] text-xs rounded">
                    {installedMods.length} {installedMods.length === 1 ? 'mod' : 'mods'}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  {instance.loader === "fabric" && modsWithProjectId > 0 && (
                    <>
                      {availableUpdates.length > 0 ? (
                        <button
                          onClick={updateAllMods}
                          disabled={isUpdatingMods}
                          className="flex items-center gap-1.5 px-3 py-1.5 bg-[#3b82f6] hover:bg-[#2563eb] disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-md text-sm font-medium transition-colors cursor-pointer"
                        >
                          {isUpdatingMods ? (
                            <>
                              <Loader2 size={14} className="animate-spin" />
                              <span>Updating...</span>
                            </>
                          ) : (
                            <>
                              <RefreshCw size={14} />
                              <span>Update All ({availableUpdates.length})</span>
                            </>
                          )}
                        </button>
                      ) : (
                        <button
                          onClick={checkForUpdates}
                          disabled={isCheckingUpdates}
                          className="flex items-center gap-1.5 px-3 py-1.5 bg-[#141414] hover:bg-[#1a1a1a] border border-[#2a2a2a] disabled:opacity-50 text-[#7d8590] hover:text-[#e6edf3] rounded-md text-sm transition-colors cursor-pointer"
                        >
                          {isCheckingUpdates ? (
                            <>
                              <Loader2 size={14} className="animate-spin" />
                              <span>Checking...</span>
                            </>
                          ) : (
                            <>
                              <RefreshCw size={14} />
                              <span>Check for Updates</span>
                            </>
                          )}
                        </button>
                      )}
                    </>
                  )}
                </div>
              </div>
              
              {isLoadingMods ? (
                <div className="text-center py-16">
                  <Loader2 size={32} className="animate-spin text-[#238636] mx-auto" />
                </div>
              ) : installedMods.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-16">
                  <Package size={48} className="text-[#238636] mb-3" strokeWidth={1.5} />
                  <h3 className="text-base font-semibold text-[#e6edf3] mb-1">No mods installed</h3>
                  <p className="text-sm text-[#7d8590]">Browse the mods tab to add mods</p>
                </div>
              ) : (
                <div className="space-y-3">
                  {installedMods.map((mod) => {
                    const hasUpdate = availableUpdates.some(u => u.filename === mod.filename)
                    
                    return (
                      <div
                        key={mod.filename}
                        className={`bg-[#141414] border border-[#2a2a2a] hover:ring-1 hover:ring-[#2a2a2a] rounded-md overflow-hidden transition-all ${
                          mod.disabled ? 'opacity-60' : ''
                        } ${hasUpdate ? 'ring-2 ring-[#3b82f6]/50' : ''}`}
                      >
                        <div className="flex min-h-0">
                          {mod.icon_url ? (
                            <div className="w-22 bg-[#0f0f0f] flex items-center justify-center flex-shrink-0 self-stretch">
                              <img
                                src={mod.icon_url}
                                alt={mod.name || mod.filename}
                                className={`w-full h-full object-contain ${
                                  mod.disabled ? 'grayscale' : ''
                                }`}
                              />
                            </div>
                          ) : (
                            <div className="w-22 bg-[#0f0f0f] flex items-center justify-center flex-shrink-0 self-stretch">
                              <Package size={32} className="text-[#3a3a3a]" strokeWidth={1.5} />
                            </div>
                          )}
                          <div className="flex-1 min-w-0 py-2 px-3 flex items-center gap-3">
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2">
                                <h3 className="font-semibold text-base text-[#e6edf3] truncate">
                                  {mod.name || mod.filename}
                                </h3>
                                {hasUpdate && (
                                  <span className="px-1.5 py-0.5 bg-[#3b82f6] text-white text-xs rounded font-medium">
                                    Update
                                  </span>
                                )}
                              </div>
                              <p className="text-sm text-[#7d8590] truncate">{mod.filename}</p>
                              <p className="text-sm text-[#3a3a3a] mt-0.5">{formatFileSize(mod.size)}</p>
                            </div>
                            <div className="flex flex-col items-end gap-1">
                              <button
                                onClick={() => handleToggleMod(mod)}
                                className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors cursor-pointer ${
                                  mod.disabled ? 'bg-red-500/80' : 'bg-[#238636]'
                                }`}
                              >
                                <span
                                  className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                                    mod.disabled ? 'translate-x-1' : 'translate-x-6'
                                  }`}
                                />
                              </button>
                              <button
                                onClick={() => handleDeleteMod(mod.filename)}
                                className="p-1.5 hover:bg-red-500/10 text-[#7d8590] hover:text-red-400 rounded-md transition-all cursor-pointer mt-1"
                              >
                                <Trash2 size={16} />
                              </button>
                            </div>
                          </div>
                        </div>
                      </div>
                    )
                  })}
                </div>
              )}
            </div>

            <div className="pl-6">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-3">
                  <h2 className="text-lg font-semibold text-[#e6edf3] tracking-tight">Worlds</h2>
                  <span className="px-2 py-0.5 bg-[#141414] border border-[#2a2a2a] text-[#7d8590] text-xs rounded">
                    {worlds.length} {worlds.length === 1 ? 'world' : 'worlds'}
                  </span>
                </div>
                <button
                  onClick={handleOpenWorldsFolder}
                  className="flex items-center gap-1.5 text-sm text-[#7d8590] hover:text-[#e6edf3] transition-colors cursor-pointer"
                >
                  <ExternalLink size={14} />
                  <span>Open Folder</span>
                </button>
              </div>
              
              {isLoadingWorlds ? (
                <div className="text-center py-16">
                  <Loader2 size={32} className="animate-spin text-[#238636] mx-auto" />
                </div>
              ) : worlds.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-16">
                  <Globe size={48} className="text-[#238636] mb-3" strokeWidth={1.5} />
                  <h3 className="text-base font-semibold text-[#e6edf3] mb-1">No worlds yet</h3>
                  <p className="text-sm text-[#7d8590]">Launch the game to create a world</p>
                </div>
              ) : (
                <div className="space-y-3">
                  {worlds.map((world) => (
                    <div
                      key={world.folder_name}
                      className="bg-[#141414] border border-[#2a2a2a] hover:ring-1 hover:ring-[#2a2a2a] rounded-md overflow-hidden transition-all"
                    >
                      <div className="flex min-h-0">
                        {world.icon ? (
                          <div className="w-22 bg-[#0f0f0f] flex items-center justify-center flex-shrink-0 self-stretch">
                            <img 
                              src={world.icon} 
                              alt={world.name} 
                              className="w-full h-full object-contain"
                            />
                          </div>
                        ) : (
                          <div className="w-22 bg-[#0f0f0f] flex items-center justify-center flex-shrink-0 self-stretch">
                            <Globe size={32} className="text-[#3a3a3a]" strokeWidth={1.5} />
                          </div>
                        )}
                        <div className="flex-1 min-w-0 py-2 px-3 flex items-center gap-3">
                          <div className="flex-1 min-w-0">
                            <h3 className="font-semibold text-base text-[#e6edf3] truncate">
                              {world.name}
                            </h3>
                            <p className="text-xs text-[#7d8590] mt-0.5">
                              Created {formatDate(world.created)}
                            </p>
                            <div className="flex items-center gap-2 text-sm text-[#7d8590] mt-0.5">
                              <span>{formatFileSize(world.size)}</span>
                              {world.game_mode && (
                                <>
                                  <span>•</span>
                                  <span className="capitalize">{world.game_mode}</span>
                                </>
                              )}
                            </div>
                          </div>
                          <div className="flex flex-col items-end gap-1">
                            <button
                              onClick={() => handleOpenWorldFolder(world.folder_name)}
                              className="p-1.5 hover:bg-[#1a1a1a] text-[#7d8590] hover:text-[#e6edf3] rounded-md transition-all cursor-pointer"
                            >
                              <FolderOpen size={16} />
                            </button>
                            <button
                              onClick={() => handleDeleteWorld(world.folder_name, world.name)}
                              className="p-1.5 hover:bg-red-500/10 text-[#7d8590] hover:text-red-400 rounded-md transition-all cursor-pointer"
                              title="Delete world"
                            >
                              <Trash2 size={16} />
                            </button>
                          </div>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      <InstanceSettingsModal
        isOpen={isSettingsOpen}
        instance={instance}
        instanceIcon={instanceIcon}
        onClose={() => setIsSettingsOpen(false)}
        onInstanceUpdated={handleInstanceUpdated}
        onInstanceDeleted={handleInstanceDeleted}
      />

      {confirmModal && (
        <ConfirmModal
          isOpen={confirmModal.isOpen}
          title={confirmModal.title}
          message={confirmModal.message}
          type={confirmModal.type}
          confirmText={confirmModal.type === "danger" ? "Delete" : "Confirm"}
          onConfirm={confirmModal.onConfirm}
          onCancel={() => setConfirmModal(null)}
        />
      )}

      {alertModal && (
        <AlertModal
          isOpen={alertModal.isOpen}
          title={alertModal.title}
          message={alertModal.message}
          type={alertModal.type}
          onClose={() => setAlertModal(null)}
        />
      )}
    </>
  )
}