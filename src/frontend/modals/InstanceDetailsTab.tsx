import { useState, useEffect } from "react"
import { Play, FolderOpen, Package, Loader2, ExternalLink, Globe, Settings, Trash2 } from "lucide-react"
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
  onLaunch: () => void
  onBack: () => void
  onInstanceUpdated: () => void
}

export function InstanceDetailsTab({
  instance,
  isAuthenticated,
  isLaunching,
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
                return {
                  ...mod,
                  disabled: isDisabled,
                  name: bestMatch.title,
                  description: bestMatch.description,
                  icon_url: bestMatch.icon_url,
                  downloads: bestMatch.downloads,
                  author: bestMatch.author,
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

  const formatFileSize = (bytes: number): string => {
    if (bytes >= 1024 * 1024) {
      return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
    }
    if (bytes >= 1024) {
      return `${(bytes / 1024).toFixed(1)} KB`
    }
    return `${bytes} B`
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

  const handleOpenModsFolder = async () => {
    try {
      await invoke("open_mods_folder", { instanceName: instance.name })
    } catch (error) {
      console.error("Failed to open mods folder:", error)
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

  return (
    <>
      <div className="p-6 space-y-4">
        <div className="max-w-7xl mx-auto">
          <div className="flex items-start gap-4 mb-6">
            <div className="flex-shrink-0">
              {instanceIcon ? (
                <div className="w-16 h-16 rounded-xl overflow-hidden">
                  <img
                    src={instanceIcon}
                    alt={instance.name}
                    className="w-full h-full object-cover"
                  />
                </div>
              ) : (
                <div className="w-16 h-16 border-2 border-dashed border-[#2a2a2a] rounded-xl flex items-center justify-center bg-transparent">
                  <Package size={24} className="text-[#4a4a4a]" />
                </div>
              )}
            </div>

            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between mb-2">
                <div className="flex-1 min-w-0">
                  <h1 className="text-2xl font-semibold text-[#e8e8e8] tracking-tight">{instance.name}</h1>
                  <p className="text-sm text-[#808080] mt-0.5">
                    Minecraft {getMinecraftVersion(instance)}
                    {" • "}
                    {instance.loader === "fabric" ? (
                      <>
                        <span className="text-[#3b82f6]">Fabric Loader</span>
                        {fabricLoaderVersion && (
                          <span className="text-[#808080]"> {fabricLoaderVersion}</span>
                        )}
                      </>
                    ) : (
                      <span className="text-[#16a34a]">Vanilla</span>
                    )}
                  </p>
                </div>
                <div className="flex gap-2">
                  <button
                    onClick={onLaunch}
                    disabled={!isAuthenticated}
                    className={`px-6 py-2.5 rounded-lg font-medium text-sm flex items-center gap-2 transition-all cursor-pointer ${
                      isLaunching
                        ? "bg-red-500/10 text-red-400"
                        : "bg-[#16a34a]/10 hover:bg-[#16a34a]/20 text-[#16a34a]"
                    } disabled:opacity-50 disabled:cursor-not-allowed`}
                  >
                    {isLaunching ? (
                      <>
                        <div className="w-4 h-4 border-2 border-red-400/30 border-t-red-400 rounded-full animate-spin" />
                        <span>Launching...</span>
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
                    className="px-4 py-2.5 bg-[#1a1a1a] hover:bg-[#1f1f1f] text-[#e8e8e8] rounded-lg font-medium text-sm flex items-center gap-2 transition-all cursor-pointer"
                  >
                    <FolderOpen size={16} />
                    <span>Open Folder</span>
                  </button>
                  <button
                    onClick={() => setIsSettingsOpen(true)}
                    className="px-4 py-2.5 bg-[#1a1a1a] hover:bg-[#1f1f1f] text-[#e8e8e8] rounded-lg font-medium text-sm flex items-center gap-2 transition-all cursor-pointer"
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
                  <h2 className="text-lg font-semibold text-[#e8e8e8] tracking-tight">Installed Mods</h2>
                  <span className="px-2 py-0.5 bg-[#1a1a1a] text-[#808080] text-xs rounded">
                    {installedMods.length} {installedMods.length === 1 ? 'mod' : 'mods'}
                  </span>
                </div>
                <button
                  onClick={handleOpenModsFolder}
                  className="flex items-center gap-1.5 text-sm text-[#808080] hover:text-[#e8e8e8] transition-colors cursor-pointer"
                >
                  <ExternalLink size={14} />
                  <span>Open Folder</span>
                </button>
              </div>
              
              {isLoadingMods ? (
                <div className="text-center py-16">
                  <Loader2 size={32} className="animate-spin text-[#16a34a] mx-auto" />
                </div>
              ) : installedMods.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-16">
                  <Package size={48} className="text-[#16a34a] mb-3" strokeWidth={1.5} />
                  <h3 className="text-base font-semibold text-[#e8e8e8] mb-1">No mods installed</h3>
                  <p className="text-sm text-[#808080]">Browse the mods tab to add mods</p>
                </div>
              ) : (
                <div className="space-y-3">
                  {installedMods.map((mod) => (
                    <div
                      key={mod.filename}
                      className={`bg-[#1a1a1a] hover:bg-[#1f1f1f] rounded-xl overflow-hidden transition-all ${
                        mod.disabled ? 'opacity-60' : ''
                      }`}
                    >
                      <div className="flex min-h-0">
                        {mod.icon_url ? (
                          <div className="w-22 bg-[#1a1a1a] flex items-center justify-center flex-shrink-0 self-stretch">
                            <img
                              src={mod.icon_url}
                              alt={mod.name || mod.filename}
                              className={`w-full h-full object-contain ${
                                mod.disabled ? 'grayscale' : ''
                              }`}
                            />
                          </div>
                        ) : (
                          <div className="w-22 bg-gradient-to-br from-[#16a34a]/10 to-[#15803d]/10 flex items-center justify-center flex-shrink-0 self-stretch">
                            <Package size={32} className="text-[#16a34a]" />
                          </div>
                        )}
                        <div className="flex-1 min-w-0 py-2 px-3 flex items-center gap-3">
                          <div className="flex-1 min-w-0">
                            <h3 className="font-semibold text-base text-[#e8e8e8] truncate">
                              {mod.name || mod.filename}
                            </h3>
                            <p className="text-sm text-[#808080] truncate">{mod.filename}</p>
                            <p className="text-sm text-[#4a4a4a] mt-0.5">{formatFileSize(mod.size)}</p>
                          </div>
                          <div className="flex flex-col items-end gap-1">
                            <button
                              onClick={() => handleToggleMod(mod)}
                              className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors cursor-pointer ${
                                mod.disabled ? 'bg-red-500/80' : 'bg-[#16a34a]'
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
                              className="p-1.5 hover:bg-red-500/10 text-[#808080] hover:text-red-400 rounded-md transition-all cursor-pointer mt-1"
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

            <div className="pl-6">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-3">
                  <h2 className="text-lg font-semibold text-[#e8e8e8] tracking-tight">Worlds</h2>
                  <span className="px-2 py-0.5 bg-[#1a1a1a] text-[#808080] text-xs rounded">
                    {worlds.length} {worlds.length === 1 ? 'world' : 'worlds'}
                  </span>
                </div>
                <button
                  onClick={handleOpenWorldsFolder}
                  className="flex items-center gap-1.5 text-sm text-[#808080] hover:text-[#e8e8e8] transition-colors cursor-pointer"
                >
                  <ExternalLink size={14} />
                  <span>Open Folder</span>
                </button>
              </div>
              
              {isLoadingWorlds ? (
                <div className="text-center py-16">
                  <Loader2 size={32} className="animate-spin text-[#16a34a] mx-auto" />
                </div>
              ) : worlds.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-16">
                  <Globe size={48} className="text-[#16a34a] mb-3" strokeWidth={1.5} />
                  <h3 className="text-base font-semibold text-[#e8e8e8] mb-1">No worlds yet</h3>
                  <p className="text-sm text-[#808080]">Launch the game to create a world</p>
                </div>
              ) : (
                <div className="space-y-3">
                  {worlds.map((world) => (
                    <div
                      key={world.folder_name}
                      className="bg-[#1a1a1a] hover:bg-[#1f1f1f] rounded-xl overflow-hidden transition-all"
                    >
                      <div className="flex min-h-0">
                        {world.icon ? (
                          <div className="w-22 bg-[#1a1a1a] flex items-center justify-center flex-shrink-0 self-stretch">
                            <img 
                              src={world.icon} 
                              alt={world.name} 
                              className="w-full h-full object-contain"
                            />
                          </div>
                        ) : (
                          <div className="w-22 bg-gradient-to-br from-[#16a34a]/10 to-[#15803d]/10 flex items-center justify-center flex-shrink-0 self-stretch">
                            <Globe size={32} className="text-[#16a34a]" />
                          </div>
                        )}
                        <div className="flex-1 min-w-0 py-2 px-3 flex items-center gap-3">
                          <div className="flex-1 min-w-0">
                            <h3 className="font-semibold text-base text-[#e8e8e8] truncate">
                              {world.name}
                            </h3>
                            <p className="text-xs text-[#808080] mt-0.5">
                              Created {formatDate(world.created)}
                            </p>
                            <div className="flex items-center gap-2 text-sm text-[#808080] mt-0.5">
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
                              className="p-1.5 hover:bg-[#2a2a2a] text-[#808080] hover:text-[#e8e8e8] rounded-md transition-all cursor-pointer"
                            >
                              <FolderOpen size={16} />
                            </button>
                            <button
                              onClick={() => handleDeleteWorld(world.folder_name, world.name)}
                              className="p-1.5 hover:bg-red-500/10 text-[#808080] hover:text-red-400 rounded-md transition-all cursor-pointer"
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