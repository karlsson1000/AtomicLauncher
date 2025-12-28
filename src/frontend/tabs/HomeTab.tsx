import { Package, Plus, FolderOpen, Copy, Trash2, Play } from "lucide-react"
import { useState, useEffect } from "react"
import { invoke } from "@tauri-apps/api/core"
import type { Instance } from "../../types"
import { ContextMenu } from "../modals/ContextMenu"

interface HomeTabProps {
  instances: Instance[]
  isAuthenticated: boolean
  launchingInstanceName: string | null
  onLaunch: (instance: Instance) => void | Promise<void>
  onOpenFolder: () => void
  onDeleteInstance: (name: string) => void
  onCreateNew: () => void
  onShowDetails: (instance: Instance) => void
  onOpenFolderByInstance?: (instance: Instance) => void
  onDuplicateInstance?: (instance: Instance) => void
  onRefreshInstances?: () => void
}

export function HomeTab({
  instances,
  isAuthenticated,
  launchingInstanceName,
  onLaunch,
  onCreateNew,
  onShowDetails,
  onOpenFolderByInstance,
  onDuplicateInstance,
  onDeleteInstance,
}: HomeTabProps) {
  const [contextMenu, setContextMenu] = useState<{
    x: number
    y: number
    instance: Instance
  } | null>(null)
  const [instanceIcons, setInstanceIcons] = useState<Record<string, string | null>>({})

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
        <div>
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
                const isLaunching = launchingInstanceName === instance.name
                return (
                  <div
                    key={instance.name}
                    onClick={() => onShowDetails(instance)}
                    onContextMenu={(e) => handleContextMenu(e, instance)}
                    className="group relative bg-[#1a1a1a] rounded-xl overflow-hidden cursor-pointer transition-all hover:ring-2 hover:ring-[#2a2a2a]"
                  >
                    {/* Square Image Section */}
                    <div className="aspect-square bg-[#141414] flex items-center justify-center overflow-hidden">
                      {icon ? (
                        <img
                          src={icon}
                          alt={instance.name}
                          className="w-full h-full object-cover"
                        />
                      ) : (
                        <Package size={88} className="text-[#4a4a4a]" strokeWidth={1.5} />
                      )}
                    </div>
                    
                    {/* Solid Text Section with Play Button */}
                    <div className="bg-[#1a1a1a] p-3 flex items-center justify-between gap-2">
                      <div className="flex-1 min-w-0">
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
                      
                      {/* Play Button */}
                      {isAuthenticated && (
                        <button
                          onClick={(e) => {
                            e.stopPropagation()
                            onLaunch(instance)
                          }}
                          disabled={launchingInstanceName !== null}
                          className={`opacity-0 group-hover:opacity-100 flex-shrink-0 w-10 h-10 flex items-center justify-center rounded-md transition-all cursor-pointer ${
                            isLaunching
                              ? "bg-red-500/10 text-red-400"
                              : "bg-[#16a34a]/10 hover:bg-[#16a34a]/20 text-[#16a34a]"
                          } disabled:opacity-50`}
                        >
                          {isLaunching ? (
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