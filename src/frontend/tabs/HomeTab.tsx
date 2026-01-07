import { Package, Plus, FolderOpen, Copy, Trash2, Play, ExternalLink, LayoutGrid, LayoutList } from "lucide-react"
import { useState, useEffect } from "react"
import { invoke } from "@tauri-apps/api/core"
import type { Instance } from "../../types"
import { ContextMenu } from "../modals/ContextMenu"

interface HomeTabProps {
  instances: Instance[]
  isAuthenticated: boolean
  launchingInstanceName: string | null
  runningInstances: Set<string>
  onLaunch: (instance: Instance) => void | Promise<void>
  onOpenFolder: () => void
  onDeleteInstance: (name: string) => void
  onCreateNew: () => void
  onShowDetails: (instance: Instance) => void
  onOpenFolderByInstance?: (instance: Instance) => void
  onDuplicateInstance?: (instance: Instance) => void
  onRefreshInstances?: () => void
  onKillInstance?: (instance: Instance) => void
}

interface Snapshot {
  id: string
  title: string
  version: string
  type: string
  date?: string
  image?: {
    title: string
    url: string
  }
  body?: string
  contentPath?: string
  shortText?: string
}

interface SnapshotsResponse {
  version: number
  entries: Snapshot[]
}

export function HomeTab({
  instances,
  isAuthenticated,
  launchingInstanceName,
  runningInstances,
  onLaunch,
  onCreateNew,
  onShowDetails,
  onOpenFolderByInstance,
  onDuplicateInstance,
  onDeleteInstance,
  onKillInstance,
}: HomeTabProps) {
  const [contextMenu, setContextMenu] = useState<{
    x: number
    y: number
    instance: Instance
  } | null>(null)
  const [instanceIcons, setInstanceIcons] = useState<Record<string, string | null>>({})
  const [snapshots, setSnapshots] = useState<Snapshot[]>([])
  const [loadingSnapshots, setLoadingSnapshots] = useState(true)
  const [viewMode, setViewMode] = useState<"grid" | "list">("grid")

  // Get only the 5 most recently played instances
  const recentInstances = instances
    .filter(instance => instance.last_played)
    .sort((a, b) => {
      const timeA = a.last_played ? new Date(a.last_played).getTime() : 0
      const timeB = b.last_played ? new Date(b.last_played).getTime() : 0
      return timeB - timeA
    })
    .slice(0, 5)

  useEffect(() => {
    const loadIcons = async () => {
      const icons: Record<string, string | null> = {}
      for (const instance of recentInstances) {
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

    if (recentInstances.length > 0) {
      loadIcons()
    }
  }, [instances])

  // Load snapshots
  useEffect(() => {
    const loadSnapshots = async () => {
      try {
        const response = await fetch('https://launchercontent.mojang.com/v2/javaPatchNotes.json')
        const data: SnapshotsResponse = await response.json()
        // Get the latest 4 snapshots
        setSnapshots(data.entries.slice(0, 4))
      } catch (error) {
        console.error('Failed to load snapshots:', error)
      } finally {
        setLoadingSnapshots(false)
      }
    }

    loadSnapshots()
  }, [])

  const handleContextMenu = (e: React.MouseEvent, instance: Instance) => {
    e.preventDefault()
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      instance,
    })
  }

  const getMinecraftVersion = (instance: Instance): string => {
    if (instance.loader === "fabric") {
      const parts = instance.version.split('-')
      return parts[parts.length - 1]
    }
    return instance.version
  }

  const formatDate = (dateString: string): string => {
    const date = new Date(dateString)
    const now = new Date()
    const diffTime = now.getTime() - date.getTime()
    const diffDays = Math.floor(diffTime / (1000 * 60 * 60 * 24))
    
    if (diffDays === 0) return 'Today'
    if (diffDays === 1) return 'Yesterday'
    if (diffDays < 7) return `${diffDays} days ago`
    if (diffDays < 30) return `${Math.floor(diffDays / 7)} weeks ago`
    
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
  }

  const cleanVersionName = (version: string): string => {
    // Remove "Minecraft Java Edition" and "| Minecraft" from version string
    return version
      .replace('Minecraft Java Edition', '')
      .replace('| Minecraft', '')
      .replace('Minecraft:', '')
      .replace('Minecraft', '')
      .trim()
  }

  const getVersionUrl = (version: string): string => {
    const cleanVersion = cleanVersionName(version)
    
    // Check if it's a snapshot (contains 'w' pattern like 25w46a or 'snapshot' in name)
    const isSnapshot = /\d+w\d+[a-z]?/.test(cleanVersion) || cleanVersion.toLowerCase().includes('snapshot')
    
    if (isSnapshot) {
      // For snapshots: minecraft-snapshot-25w46a or minecraft-26-1-snapshot-1
      const urlVersion = cleanVersion.replace(/\./g, '-').toLowerCase()
      
      // Check if it's a weekly snapshot (like 25w46a)
      if (/\d+w\d+[a-z]?/.test(cleanVersion)) {
        return `https://www.minecraft.net/en-us/article/minecraft-snapshot-${urlVersion}`
      } else {
        // For numbered snapshots (like 26.1-snapshot-1)
        return `https://www.minecraft.net/en-us/article/minecraft-${urlVersion}`
      }
    } else {
      // For releases: minecraft-java-edition-1-21-4
      const urlVersion = cleanVersion.replace(/\./g, '-').toLowerCase()
      return `https://www.minecraft.net/en-us/article/minecraft-java-edition-${urlVersion}`
    }
  }

  return (
    <div className="p-6 space-y-6">
      <div className="max-w-7xl mx-auto space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <div>
            <h1 className="text-2xl font-semibold text-[#e6edf3] tracking-tight">Home</h1>
            <p className="text-sm text-[#7d8590] mt-0.5">Recently played instances</p>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => setViewMode(viewMode === "grid" ? "list" : "grid")}
              className="w-8.5 h-8.5 hover:bg-[#1a1a1a] text-[#7d8590] hover:text-[#e6edf3] rounded flex items-center justify-center transition-colors cursor-pointer"
            >
              {viewMode === "grid" ? <LayoutList size={22} /> : <LayoutGrid size={22} />}
            </button>
            <button
              onClick={onCreateNew}
              className="px-4 h-8 bg-[#238636] hover:bg-[#2ea043] text-white rounded-md text-sm font-medium flex items-center gap-2 transition-colors cursor-pointer"
            >
              <Plus size={16} />
              New
            </button>
          </div>
        </div>

        {/* Instances Section */}
        <div>
          {recentInstances.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-14">
              <Package size={48} className="text-[#16a34a] mb-3" strokeWidth={1.5} />
              <h3 className="text-base font-semibold text-[#e6edf3] mb-1">No recently played instances</h3>
              <p className="text-sm text-[#7d8590] mb-4">Launch an instance to see it here</p>
              <button
                onClick={onCreateNew}
                className="px-4 py-2 bg-[#238636] hover:bg-[#2ea043] text-white rounded font-medium text-sm flex items-center gap-2 transition-all cursor-pointer"
              >
                <Plus size={16} strokeWidth={2} />
                <span>Create Instance</span>
              </button>
            </div>
          ) : (
            <>
              {viewMode === "grid" ? (
                <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3">
                  {recentInstances.map((instance) => {
                    const icon = instanceIcons[instance.name]
                    const isLaunching = launchingInstanceName === instance.name
                    const isRunning = runningInstances.has(instance.name)
                    return (
                      <div
                        key={instance.name}
                        onClick={() => onShowDetails(instance)}
                        onContextMenu={(e) => handleContextMenu(e, instance)}
                        className="group relative bg-[#141414] rounded-md overflow-hidden cursor-pointer transition-all hover:ring-1 hover:ring-[#2a2a2a] border border-[#2a2a2a]"
                      >
                        {/* Square Image Section */}
                        <div className="aspect-square bg-[#0f0f0f] flex items-center justify-center overflow-hidden">
                          {icon ? (
                            <img
                              src={icon}
                              alt={instance.name}
                              className="w-full h-full object-cover"
                            />
                          ) : (
                            <Package size={88} className="text-[#3a3a3a]" strokeWidth={1.5} />
                          )}
                        </div>
                        
                        {/* Solid Text Section with Play Button */}
                        <div className="bg-[#141414] p-3 flex items-center justify-between gap-2">
                          <div className="flex-1 min-w-0">
                            <h3 className="text-sm font-semibold text-[#e6edf3] truncate mb-0.5">{instance.name}</h3>
                            <div className="flex items-center gap-1.5 text-xs">
                              <span className="text-[#7d8590]">{getMinecraftVersion(instance)}</span>
                              <span className="text-[#3a3a3a]">•</span>
                              {instance.loader === "fabric" ? (
                                <span className="text-[#3b82f6]">Fabric</span>
                              ) : (
                                <span className="text-[#16a34a]">Vanilla</span>
                              )}
                            </div>
                          </div>
                          
                          {/* Play Button */}
                          {isAuthenticated && (
                            <button
                              onClick={(e) => {
                                e.stopPropagation()
                                if (isRunning && onKillInstance) {
                                  onKillInstance(instance)
                                } else {
                                  onLaunch(instance)
                                }
                              }}
                              disabled={launchingInstanceName !== null && !isRunning}
                              className={`flex-shrink-0 w-10 h-10 flex items-center justify-center rounded transition-all cursor-pointer ${
                                isRunning || isLaunching
                                  ? "bg-red-500/10 text-red-400"
                                  : "bg-[#16a34a]/10 hover:bg-[#16a34a]/20 text-[#16a34a]"
                              } disabled:opacity-50`}
                              title={isRunning ? "Stop instance" : "Launch instance"}
                            >
                              {isLaunching || isRunning ? (
                                <div className="w-4 h-4 border-2 border-red-400/30 border-t-red-400 rounded-full animate-spin" />
                              ) : (
                                <Play size={18} fill="currentColor" strokeWidth={0} />
                              )}
                            </button>
                          )}
                        </div>
                      </div>
                    )
                  })}
                </div>
              ) : (
                <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-3">
                  {recentInstances.map((instance) => {
                    const icon = instanceIcons[instance.name]
                    const isLaunching = launchingInstanceName === instance.name
                    const isRunning = runningInstances.has(instance.name)
                    return (
                      <div
                        key={instance.name}
                        onClick={() => onShowDetails(instance)}
                        onContextMenu={(e) => handleContextMenu(e, instance)}
                        className="group relative bg-[#141414] rounded-md overflow-hidden cursor-pointer transition-all hover:ring-1 hover:ring-[#2a2a2a] border border-[#2a2a2a] flex items-center"
                      >
                        {/* Instance Image */}
                        <div className="w-20 h-20 bg-[#0f0f0f] flex items-center justify-center flex-shrink-0">
                          {icon ? (
                            <img
                              src={icon}
                              alt={instance.name}
                              className="w-full h-full object-cover"
                            />
                          ) : (
                            <Package size={32} className="text-[#3a3a3a]" strokeWidth={1.5} />
                          )}
                        </div>
                        
                        {/* Instance Info */}
                        <div className="flex-1 min-w-0 px-4 py-3">
                          <h3 className="text-base font-semibold text-[#e6edf3] truncate mb-1">{instance.name}</h3>
                          <div className="flex items-center gap-2 text-sm">
                            <span className="text-[#7d8590]">{getMinecraftVersion(instance)}</span>
                            <span className="text-[#3a3a3a]">•</span>
                            {instance.loader === "fabric" ? (
                              <span className="text-[#3b82f6]">Fabric</span>
                            ) : (
                              <span className="text-[#16a34a]">Vanilla</span>
                            )}
                          </div>
                        </div>
                        
                        {/* Play Button */}
                        {isAuthenticated && (
                          <div className="px-4">
                            <button
                              onClick={(e) => {
                                e.stopPropagation()
                                if (isRunning && onKillInstance) {
                                  onKillInstance(instance)
                                } else {
                                  onLaunch(instance)
                                }
                              }}
                              disabled={launchingInstanceName !== null && !isRunning}
                              className={`flex-shrink-0 w-10 h-10 flex items-center justify-center rounded transition-all cursor-pointer ${
                                isRunning || isLaunching
                                  ? "bg-red-500/10 text-red-400"
                                  : "bg-[#16a34a]/10 hover:bg-[#16a34a]/20 text-[#16a34a]"
                              } disabled:opacity-50`}
                              title={isRunning ? "Stop instance" : "Launch instance"}
                            >
                              {isLaunching || isRunning ? (
                                <div className="w-4 h-4 border-2 border-red-400/30 border-t-red-400 rounded-full animate-spin" />
                              ) : (
                                <Play size={18} fill="currentColor" strokeWidth={0} />
                              )}
                            </button>
                          </div>
                        )}
                      </div>
                    )
                  })}
                </div>
              )}
            </>
          )}
        </div>

        {/* Snapshots Section */}
        <div className="mt-auto">
          <div className="mb-4">
            <h2 className="text-xl font-semibold text-[#e6edf3] tracking-tight">Latest Snapshots</h2>
            <p className="text-sm text-[#7d8590] mt-0.5">Recent Java Edition snapshots</p>
          </div>

          {loadingSnapshots ? (
            <div className="flex items-center justify-center py-12">
              <div className="w-8 h-8 border-2 border-[#2a2a2a] border-t-[#16a34a] rounded-full animate-spin" />
            </div>
          ) : snapshots.length === 0 ? (
            <div className="bg-[#141414] rounded-md p-8 text-center">
              <p className="text-[#7d8590]">Unable to load snapshots</p>
            </div>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
              {snapshots.map((snapshot) => (
                <div
                  key={snapshot.id}
                  onClick={async () => {
                    try {
                      await invoke('open_url', { url: getVersionUrl(snapshot.version) })
                    } catch (error) {
                      console.error('Failed to open link:', error)
                    }
                  }}
                  className="bg-[#141414] rounded-md overflow-hidden relative group cursor-pointer border border-[#2a2a2a] hover:border-[#3a3a3a] transition-colors flex flex-col"
                >
                  {/* External Link Icon */}
                  <div className="absolute top-2 right-2 z-10 w-7 h-7 flex items-center justify-center bg-[#141414]/90 backdrop-blur-sm rounded border border-[#2a2a2a] opacity-0 group-hover:opacity-100 transition-opacity">
                    <ExternalLink size={14} className="text-[#e6edf3]" />
                  </div>
                  
                  {/* Snapshot Image */}
                  <div className="h-40 bg-[#0f0f0f] overflow-hidden relative flex-shrink-0">
                    {snapshot.image?.url ? (
                      <img
                        src={`https://launchercontent.mojang.com${snapshot.image.url}`}
                        alt={snapshot.title}
                        className="w-full h-full object-cover"
                      />
                    ) : (
                      <div className="w-full h-full flex items-center justify-center">
                        <Package size={48} className="text-[#3a3a3a]" strokeWidth={1.5} />
                      </div>
                    )}
                    {/* Hover Overlay - Gradient from bottom */}
                    <div className="absolute inset-0 bg-gradient-to-t from-black/80 via-black/20 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300" />
                  </div>
                  
                  {/* Content */}
                  <div className="p-4 flex-1 flex flex-col">
                    <div className="flex items-center justify-between gap-2 mb-2">
                      <h3 className="text-sm font-semibold text-[#e6edf3] truncate">
                        {cleanVersionName(snapshot.version)}
                      </h3>
                      {snapshot.date && (
                        <span className="text-xs text-[#7d8590] whitespace-nowrap">
                          {formatDate(snapshot.date)}
                        </span>
                      )}
                    </div>
                    {snapshot.shortText && (
                      <p className="text-xs text-[#7d8590] line-clamp-3 leading-snug">
                        {snapshot.shortText}
                      </p>
                    )}
                  </div>
                </div>
              ))}
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