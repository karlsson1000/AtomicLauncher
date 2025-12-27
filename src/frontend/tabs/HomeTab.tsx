import { Package, Plus, FolderOpen, Copy, Trash2, Download, Loader2, CheckCircle, AlertCircle } from "lucide-react"
import { useState, useEffect, useRef } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import type { Instance, ModrinthSearchResult, ModrinthProject, ModrinthVersion } from "../../types"
import { ContextMenu } from "../modals/ContextMenu"

interface HomeTabProps {
  selectedInstance: Instance | null
  instances: Instance[]
  isAuthenticated: boolean
  isLaunching: boolean
  onSetSelectedInstance: (instance: Instance) => void
  onLaunch: () => void
  onOpenFolder: () => void
  onDeleteInstance: (name: string) => void
  onCreateNew: () => void
  onShowDetails: (instance: Instance) => void
  onOpenFolderByInstance?: (instance: Instance) => void
  onDuplicateInstance?: (instance: Instance) => void
  onRefreshInstances?: () => void
}

interface ModpackInstallProgress {
  instance: string
  progress: number
  stage: string
}

export function HomeTab({
  selectedInstance,
  instances,
  isAuthenticated,
  isLaunching,
  onSetSelectedInstance,
  onLaunch,
  onCreateNew,
  onShowDetails,
  onOpenFolderByInstance,
  onDuplicateInstance,
  onDeleteInstance,
  onRefreshInstances,
}: HomeTabProps) {
  const [showInstanceDropdown, setShowInstanceDropdown] = useState(false)
  const [contextMenu, setContextMenu] = useState<{
    x: number
    y: number
    instance: Instance
  } | null>(null)
  const [instanceIcons, setInstanceIcons] = useState<Record<string, string | null>>({})
  const [popularModpacks, setPopularModpacks] = useState<ModrinthProject[]>([])
  const [isLoadingModpacks, setIsLoadingModpacks] = useState(false)
  const [installingModpacks, setInstallingModpacks] = useState<Set<string>>(new Set())
  const [modpackProgress, setModpackProgress] = useState<Record<string, ModpackInstallProgress>>({})
  const [installationStatus, setInstallationStatus] = useState<Record<string, 'success' | 'error'>>({})
  const [selectedVersions, setSelectedVersions] = useState<Record<string, string>>({})
  const [modpackVersions, setModpackVersions] = useState<Record<string, ModrinthVersion[]>>({})
  const [loadingVersions, setLoadingVersions] = useState<Set<string>>(new Set())
  const dropdownRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setShowInstanceDropdown(false)
      }
    }

    if (showInstanceDropdown) {
      document.addEventListener('mousedown', handleClickOutside)
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [showInstanceDropdown])

  // Listen for modpack installation progress
  useEffect(() => {
    const unlisten = listen<ModpackInstallProgress>('modpack-install-progress', (event) => {
      const progress = event.payload
      setModpackProgress(prev => ({
        ...prev,
        [progress.instance]: progress
      }))
    })

    return () => {
      unlisten.then(fn => fn())
    }
  }, [])

  // Load icons for all instances
  useEffect(() => {
    const loadIcons = async () => {
      const icons: Record<string, string | null> = {}
      for (const instance of instances) {
        try {
          const icon = await invoke<string | null>("get_instance_icon", {
            instanceName: instance.name
          })
          icons[instance.name] = icon
        } catch (error) {
          console.error(`Failed to load icon for ${instance.name}:`, error)
          icons[instance.name] = null
        }
      }
      setInstanceIcons(icons)
    }

    if (instances.length > 0) {
      loadIcons()
    }
  }, [instances])

  // Load popular modpacks on mount
  useEffect(() => {
    const loadPopularModpacks = async () => {
      setIsLoadingModpacks(true)
      try {
        const facets = JSON.stringify([["project_type:modpack"]])
        const result = await invoke<ModrinthSearchResult>("search_mods", {
          query: "",
          facets,
          index: "downloads",
          offset: 0,
          limit: 6,
        })
        setPopularModpacks(result.hits)
      } catch (error) {
        console.error("Failed to load popular modpacks:", error)
      } finally {
        setIsLoadingModpacks(false)
      }
    }

    loadPopularModpacks()
  }, [])

  const handleOpenInstanceFolder = async () => {
    if (!selectedInstance) return
    try {
      await invoke("open_instance_folder", { instanceName: selectedInstance.name })
    } catch (error) {
      console.error("Failed to open instance folder:", error)
    }
  }

  const handleContextMenu = (e: React.MouseEvent, instance: Instance) => {
    e.preventDefault()
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      instance,
    })
  }

  const loadModpackVersions = async (modpack: ModrinthProject) => {
    const projectId = modpack.project_id
    if (modpackVersions[projectId] || loadingVersions.has(projectId)) {
      return // Already loaded or loading
    }

    setLoadingVersions(prev => new Set(prev).add(projectId))
    try {
      const versions = await invoke<ModrinthVersion[]>("get_modpack_versions", {
        idOrSlug: modpack.slug,
        gameVersion: null,
      })
      setModpackVersions(prev => ({ ...prev, [projectId]: versions }))
      // Set default selected version to latest
      if (versions.length > 0) {
        setSelectedVersions(prev => ({ ...prev, [projectId]: versions[0].id }))
      }
    } catch (error) {
      console.error("Failed to load versions:", error)
    } finally {
      setLoadingVersions(prev => {
        const newSet = new Set(prev)
        newSet.delete(projectId)
        return newSet
      })
    }
  }

  const handleInstallModpack = async (modpack: ModrinthProject) => {
    try {
      const projectId = modpack.project_id
      let versions = modpackVersions[projectId]
      
      // If versions aren't loaded yet, load them first
      if (!versions) {
        setLoadingVersions(prev => new Set(prev).add(projectId))
        versions = await invoke<ModrinthVersion[]>("get_modpack_versions", {
          idOrSlug: modpack.slug,
          gameVersion: null,
        })
        setModpackVersions(prev => ({ ...prev, [projectId]: versions }))
        setLoadingVersions(prev => {
          const newSet = new Set(prev)
          newSet.delete(projectId)
          return newSet
        })
      }

      if (versions.length === 0) {
        alert("No versions available for this modpack")
        return
      }

      // Use selected version or default to latest
      const versionId = selectedVersions[projectId] || versions[0].id
      const instanceName = modpack.title

      // Check if instance already exists
      const existingInstance = instances.find(i => i.name === instanceName)
      if (existingInstance) {
        const timestamp = Date.now()
        const newName = `${instanceName}-${timestamp}`
        
        setInstallingModpacks(prev => new Set(prev).add(modpack.project_id))
        
        await invoke("install_modpack", {
          modpackSlug: modpack.slug,
          instanceName: newName,
          versionId: versionId,
        })
        
        setInstallationStatus(prev => ({ ...prev, [modpack.project_id]: 'success' }))
      } else {
        setInstallingModpacks(prev => new Set(prev).add(modpack.project_id))
        
        await invoke("install_modpack", {
          modpackSlug: modpack.slug,
          instanceName: instanceName,
          versionId: versionId,
        })
        
        setInstallationStatus(prev => ({ ...prev, [modpack.project_id]: 'success' }))
      }

      // Refresh instances list after successful installation
      if (onRefreshInstances) {
        setTimeout(() => {
          onRefreshInstances()
        }, 500)
      }

      // Clear status after 3 seconds
      setTimeout(() => {
        setInstallingModpacks(prev => {
          const newSet = new Set(prev)
          newSet.delete(modpack.project_id)
          return newSet
        })
        setInstallationStatus(prev => {
          const newStatus = { ...prev }
          delete newStatus[modpack.project_id]
          return newStatus
        })
        setModpackProgress(prev => {
          const newProgress = { ...prev }
          delete newProgress[instanceName]
          return newProgress
        })
      }, 3000)

    } catch (error) {
      console.error("Failed to install modpack:", error)
      setInstallationStatus(prev => ({ ...prev, [modpack.project_id]: 'error' }))
      
      setInstallingModpacks(prev => {
        const newSet = new Set(prev)
        newSet.delete(modpack.project_id)
        return newSet
      })

      // Clear error after 5 seconds
      setTimeout(() => {
        setInstallationStatus(prev => {
          const newStatus = { ...prev }
          delete newStatus[modpack.project_id]
          return newStatus
        })
      }, 5000)
    }
  }

  const getMinecraftVersion = (instance: Instance): string => {
    if (instance.loader === "fabric") {
      const parts = instance.version.split('-')
      return parts[parts.length - 1]
    }
    return instance.version
  }

  const formatDownloads = (downloads: number): string => {
    if (downloads >= 1000000) return `${(downloads / 1000000).toFixed(1)}M`
    if (downloads >= 1000) return `${(downloads / 1000).toFixed(1)}K`
    return downloads.toString()
  }

  return (
    <div className="p-6 space-y-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <div>
            <h1 className="text-2xl font-semibold text-[#e8e8e8] tracking-tight">Home</h1>
            <p className="text-sm text-[#808080] mt-0.5">Recently played instances</p>
          </div>
          <button
            onClick={onCreateNew}
            className="w-10 h-10 hover:bg-[#1a1a1a] text-[#e8e8e8] rounded-lg flex items-center justify-center transition-all cursor-pointer"
            title="New Instance"
          >
            <Plus size={28} strokeWidth={2} />
          </button>
        </div>

        {/* Instances Section */}
        <div className="mb-8">
          {instances.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-18">
              <Package size={48} className="text-[#16a34a] mb-3" strokeWidth={1.5} />
              <h3 className="text-base font-semibold text-[#e8e8e8] mb-1">No instances yet</h3>
              <p className="text-sm text-[#808080] mb-4">Create your first instance to get started</p>
              <button
                onClick={onCreateNew}
                className="px-4 py-2 bg-[#16a34a] hover:bg-[#15803d] text-white rounded-lg font-medium text-sm flex items-center gap-2 transition-all cursor-pointer"
              >
                <Plus size={16} strokeWidth={2} />
                <span>Create Instance</span>
              </button>
            </div>
          ) : (
            <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3">
              {instances.map((instance) => {
                const icon = instanceIcons[instance.name]
                return (
                  <div
                    key={instance.name}
                    onClick={() => {
                      onSetSelectedInstance(instance)
                    }}
                    onContextMenu={(e) => handleContextMenu(e, instance)}
                    className={`group relative aspect-[3/4] bg-[#1a1a1a] rounded-xl overflow-hidden cursor-pointer transition-all ${
                      selectedInstance?.name === instance.name
                        ? instance.loader === "fabric"
                          ? "ring-2 ring-[#3b82f6]"
                          : "ring-2 ring-[#16a34a]"
                        : "hover:ring-2 hover:ring-[#2a2a2a]"
                    }`}
                  >
                    {icon ? (
                      <img
                        src={icon}
                        alt={instance.name}
                        className="absolute inset-0 w-full h-full object-contain p-4"
                      />
                    ) : (
                      <div className="absolute inset-0 flex items-center justify-center">
                        <Package size={88} className="text-[#4a4a4a]" strokeWidth={1.5} />
                      </div>
                    )}
                    <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/90 via-black/70 to-transparent p-3">
                      <h3 className="text-sm font-semibold text-[#e8e8e8] truncate mb-0.5">{instance.name}</h3>
                      <div className="flex items-center gap-1.5 text-xs">
                        <span className="text-[#808080]">{getMinecraftVersion(instance)}</span>
                        <span className="text-[#4a4a4a]">•</span>
                        {instance.loader === "fabric" ? (
                          <span className="text-[#3b82f6]">Fabric</span>
                        ) : (
                          <span className="text-[#16a34a]">Vanilla</span>
                        )}
                      </div>
                    </div>
                  </div>
                )
              })}
            </div>
          )}
        </div>

        {/* Popular Modpacks Section */}
        <div>
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-lg font-semibold text-[#e8e8e8]">Popular Modpacks</h2>
            <span className="text-sm text-[#808080]">From Modrinth</span>
          </div>

          {isLoadingModpacks ? (
            <div className="flex items-center justify-center py-12">
              <div className="text-center">
                <div className="w-10 h-10 border-4 border-[#16a34a]/20 border-t-[#16a34a] rounded-full animate-spin mx-auto mb-3" />
                <p className="text-sm text-[#808080]">Loading popular modpacks...</p>
              </div>
            </div>
          ) : popularModpacks.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12">
              <Package size={48} className="text-[#808080] mb-3" strokeWidth={1.5} />
              <h3 className="text-base font-semibold text-[#e8e8e8] mb-1">No modpacks found</h3>
              <p className="text-sm text-[#808080]">Try again later</p>
            </div>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
              {popularModpacks.map((modpack) => {
                const isInstalling = installingModpacks.has(modpack.project_id)
                const status = installationStatus[modpack.project_id]
                const progress = modpackProgress[modpack.title]
                const versions = modpackVersions[modpack.project_id] || []
                const isLoadingVersionsForThis = loadingVersions.has(modpack.project_id)
                const selectedVersion = selectedVersions[modpack.project_id]
                
                return (
                  <div
                    key={modpack.project_id}
                    className="bg-[#1a1a1a] rounded-xl overflow-hidden transition-all group flex flex-col"
                  >
                    <div className="flex">
                      {/* Modpack Image */}
                      <div className="relative w-32 h-32 flex-shrink-0 bg-[#1a1a1a]">
                        {modpack.icon_url ? (
                          <img
                            src={modpack.icon_url}
                            alt={modpack.title}
                            className="w-full h-full object-cover"
                          />
                        ) : (
                          <div className="w-full h-full flex items-center justify-center">
                            <Package size={48} className="text-[#16a34a]/60" strokeWidth={1.5} />
                          </div>
                        )}
                      </div>

                      {/* Modpack Info */}
                      <div className="p-3 flex-1 flex flex-col min-w-0">
                        <h3 className="text-sm font-semibold text-[#e8e8e8] mb-0.5 truncate">
                          {modpack.title}
                        </h3>
                        <p className="text-xs text-[#808080] mb-2 truncate">by {modpack.author}</p>

                        {/* Stats */}
                        <div className="flex items-center gap-3 text-xs text-[#4a4a4a] mb-auto">
                          <span className="flex items-center gap-1">
                            <Download size={12} />
                            {formatDownloads(modpack.downloads)}
                          </span>
                          <span className="flex items-center gap-1">
                            <Package size={12} />
                            {modpack.versions?.length || 0} versions
                          </span>
                        </div>

                        {/* Install Button and Version Selector */}
                        <div className="flex gap-2 mt-2">
                          {/* Version Selector */}
                          <select
                            value={selectedVersion || ''}
                            onChange={(e) => setSelectedVersions(prev => ({ ...prev, [modpack.project_id]: e.target.value }))}
                            onFocus={() => loadModpackVersions(modpack)}
                            disabled={isInstalling || isLoadingVersionsForThis}
                            className="flex-1 min-w-0 px-2 py-2 bg-[#0d0d0d] text-[#e8e8e8] rounded-lg text-xs border-none outline-none cursor-pointer hover:bg-[#2a2a2a] transition-colors disabled:opacity-50 disabled:cursor-not-allowed truncate"
                          >
                            {isLoadingVersionsForThis ? (
                              <option>Loading...</option>
                            ) : versions.length === 0 ? (
                              <option>Select version</option>
                            ) : (
                              versions.map((version) => (
                                <option key={version.id} value={version.id}>
                                  {version.name}
                                </option>
                              ))
                            )}
                          </select>

                          {/* Install Button */}
                          <button
                            onClick={() => handleInstallModpack(modpack)}
                            disabled={isInstalling || isLoadingVersionsForThis}
                            className={`flex-shrink-0 w-20 px-1.5 py-2 rounded-lg text-xs font-medium transition-all cursor-pointer flex items-center justify-center gap-1.5 ${
                              status === 'success'
                                ? 'bg-green-600 hover:bg-green-700 text-white'
                                : status === 'error'
                                ? 'bg-red-600 hover:bg-red-700 text-white'
                                : isInstalling || isLoadingVersionsForThis
                                ? 'bg-[#0d0d0d] text-[#808080] cursor-not-allowed'
                                : 'bg-[#16a34a] hover:bg-[#15803d] text-white'
                            }`}
                          >
                            {status === 'success' ? (
                              <>
                                <CheckCircle size={14} />
                                <span>Done</span>
                              </>
                            ) : status === 'error' ? (
                              <>
                                <AlertCircle size={14} />
                                <span>Error</span>
                              </>
                            ) : isInstalling ? (
                              <>
                                <Loader2 size={14} className="animate-spin" />
                                <span>Installing</span>
                              </>
                            ) : isLoadingVersionsForThis ? (
                              <>
                                <Loader2 size={14} className="animate-spin" />
                              </>
                            ) : (
                              <>
                                <Download size={14} />
                                <span>Install</span>
                              </>
                            )}
                          </button>
                        </div>
                      </div>
                    </div>

                    {/* Progress Bar - Full Width */}
                    {isInstalling && progress && (
                      <div className="px-3 pb-3 pt-2 w-full">
                        <div className="flex items-center justify-between mb-1">
                          <span className="text-xs text-[#e8e8e8] truncate">{progress.stage}</span>
                          <span className="text-xs text-[#808080]">{progress.progress}%</span>
                        </div>
                        <div className="w-full bg-[#0d0d0d] rounded-full h-1.5">
                          <div
                            className="bg-[#16a34a] h-1.5 rounded-full transition-all duration-300"
                            style={{ width: `${progress.progress}%` }}
                          />
                        </div>
                      </div>
                    )}
                  </div>
                )
              })}
            </div>
          )}
        </div>
      </div>

      {/* Context Menu */}
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={() => setContextMenu(null)}
          items={[
            {
              label: "Open",
              icon: <Package size={16} />,
              onClick: () => {
                onSetSelectedInstance(contextMenu.instance)
                onShowDetails(contextMenu.instance)
              },
            },
            {
              label: "Open Folder",
              icon: <FolderOpen size={16} />,
              onClick: () => {
                if (onOpenFolderByInstance) {
                  onOpenFolderByInstance(contextMenu.instance)
                }
              },
            },
            {
              label: "Duplicate",
              icon: <Copy size={16} />,
              onClick: () => {
                if (onDuplicateInstance) {
                  onDuplicateInstance(contextMenu.instance)
                }
              },
            },
            { separator: true },
            {
              label: "Delete",
              icon: <Trash2 size={16} />,
              onClick: () => {
                onDeleteInstance(contextMenu.instance.name)
              },
              danger: true,
            },
          ]}
        />
      )}
    </div>
  )
}