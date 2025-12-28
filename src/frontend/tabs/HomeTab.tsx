import { Package, Plus, FolderOpen, Copy, Trash2 } from "lucide-react"
import { useState, useEffect, useRef } from "react"
import { invoke } from "@tauri-apps/api/core"
import type { Instance } from "../../types"
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
}: HomeTabProps) {
  const [showInstanceDropdown, setShowInstanceDropdown] = useState(false)
  const [contextMenu, setContextMenu] = useState<{
    x: number
    y: number
    instance: Instance
  } | null>(null)
  const [instanceIcons, setInstanceIcons] = useState<Record<string, string | null>>({})
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
            <p className="text-sm text-[#808080] mt-0.5">Your Minecraft instances</p>
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