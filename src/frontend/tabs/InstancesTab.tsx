import { Package, Plus, Search, FolderOpen, Copy, Trash2, FileText, Download, ChevronDown, Play, FileDown, FileUp } from "lucide-react"
import { useState, useEffect, useRef } from "react"
import { createPortal } from "react-dom"
import { invoke } from "@tauri-apps/api/core"
import { open, save } from '@tauri-apps/plugin-dialog'
import type { Instance } from "../../types"
import { ContextMenu } from "../modals/ContextMenu"
import { ConfirmModal, AlertModal } from "../modals/ConfirmModal"
import { CreationProgressToast } from "../modals/CreationProgressToast"

interface InstanceTemplate {
  id: string
  name: string
  description: string | null
  created_at: string
  launcher_settings: LauncherSettings | null
  minecraft_options: MinecraftOptions | null
}

interface LauncherSettings {
  java_path: string | null
  memory_mb: number
}

interface MinecraftOptions {
  fov: number | null
  render_distance: number | null
  max_fps: number | null
  fullscreen: boolean | null
  vsync: boolean | null
  gui_scale: number | null
  brightness: number | null
  entity_shadows: boolean | null
  particles: string | null
  graphics: string | null
  smooth_lighting: boolean | null
  biome_blend: number | null
  master_volume: number | null
  music_volume: number | null
  mouse_sensitivity: number | null
  auto_jump: boolean | null
  keybinds: Record<string, string> | null
}

interface InstancesTabProps {
  instances: Instance[]
  selectedInstance: Instance | null
  isAuthenticated: boolean
  launchingInstanceName: string | null
  onSetSelectedInstance: (instance: Instance) => void
  onLaunch: () => void
  onCreateNew: () => void
  onShowDetails: (instance: Instance) => void
  onOpenFolder?: (instance: Instance) => void
  onDuplicateInstance?: (instance: Instance) => void
  onDeleteInstance?: (instanceName: string) => void
}

export function InstancesTab({
  instances,
  isAuthenticated,
  launchingInstanceName,
  onCreateNew,
  onSetSelectedInstance,
  onLaunch,
  onShowDetails,
  onOpenFolder,
  onDuplicateInstance,
  onDeleteInstance,
}: InstancesTabProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [contextMenu, setContextMenu] = useState<{
    x: number
    y: number
    instance: Instance
  } | null>(null)
  const [instanceIcons, setInstanceIcons] = useState<Record<string, string | null>>({})
  const [showTemplateMenu, setShowTemplateMenu] = useState(false)
  const [showApplyMenu, setShowApplyMenu] = useState(false)
  const [templates, setTemplates] = useState<InstanceTemplate[]>([])
  const [selectedTemplateId, setSelectedTemplateId] = useState<string | null>(null)
  const [creatingTemplate, setCreatingTemplate] = useState<string | null>(null)
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
  const templateButtonRef = useRef<HTMLButtonElement>(null)
  const applyButtonRef = useRef<HTMLButtonElement>(null)
  const templateMenuRef = useRef<HTMLDivElement>(null)
  const applyMenuRef = useRef<HTMLDivElement>(null)

  const getMinecraftVersion = (instance: Instance): string => {
    if (instance.loader === "fabric") {
      const parts = instance.version.split('-')
      return parts[parts.length - 1]
    }
    return instance.version
  }

  const handleQuickLaunch = (instance: Instance) => {
    onSetSelectedInstance(instance)
    onLaunch()
  }

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

  // Load templates
  const loadTemplates = async () => {
    try {
      const templateList = await invoke<InstanceTemplate[]>("get_templates")
      setTemplates(templateList)
    } catch (error) {
      console.error("Failed to load templates:", error)
    }
  }

  useEffect(() => {
    loadTemplates()
  }, [])

  // Close menus on outside click
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        templateMenuRef.current &&
        !templateMenuRef.current.contains(event.target as Node) &&
        !templateButtonRef.current?.contains(event.target as Node)
      ) {
        setShowTemplateMenu(false)
      }
      if (
        applyMenuRef.current &&
        !applyMenuRef.current.contains(event.target as Node) &&
        !applyButtonRef.current?.contains(event.target as Node)
      ) {
        setShowApplyMenu(false)
        setSelectedTemplateId(null)
      }
    }

    document.addEventListener("mousedown", handleClickOutside)
    return () => document.removeEventListener("mousedown", handleClickOutside)
  }, [])

  const handleContextMenu = (e: React.MouseEvent, instance: Instance) => {
    e.preventDefault()
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      instance,
    })
  }

  const handleCreateTemplate = async (instanceName: string) => {
    setCreatingTemplate(instanceName)
    setShowTemplateMenu(false)
    
    try {
      await invoke("create_template_from_instance", { 
        instanceName,
        templateName: `${instanceName} Template`,
        description: `Template created from ${instanceName}`
      })
      await loadTemplates()
    } catch (error) {
      console.error("Failed to create template:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to create template: ${error}`,
        type: "danger"
      })
      setCreatingTemplate(null)
    }
  }

  const handleExportTemplate = async (templateId: string, templateName: string) => {
    try {
      const filePath = await save({
        defaultPath: `${templateName}.json`,
        filters: [{
          name: 'Template',
          extensions: ['json']
        }]
      })

      if (filePath) {
        await invoke("export_template", { 
          templateId, 
          exportPath: filePath 
        })
        setAlertModal({
          isOpen: true,
          title: "Success",
          message: `Template "${templateName}" exported successfully!`,
          type: "success"
        })
      }
    } catch (error) {
      console.error("Failed to export template:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to export template: ${error}`,
        type: "danger"
      })
    }
  }

  const handleImportTemplate = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'Template',
          extensions: ['json']
        }]
      })

      if (selected && typeof selected === 'string') {
        await invoke("import_template", { importPath: selected })
        await loadTemplates()
        setAlertModal({
          isOpen: true,
          title: "Success",
          message: "Template imported successfully!",
          type: "success"
        })
      }
    } catch (error) {
      console.error("Failed to import template:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to import template: ${error}`,
        type: "danger"
      })
    }
  }

  const handleDeleteTemplate = async (templateId: string, templateName: string) => {
    setShowApplyMenu(false)
    setSelectedTemplateId(null)
    
    setConfirmModal({
      isOpen: true,
      title: "Delete Template",
      message: `Are you sure you want to delete "${templateName}"?\n\nThis action cannot be undone.`,
      type: "danger",
      onConfirm: async () => {
        try {
          await invoke("delete_template", { templateId })
          await loadTemplates()
          setConfirmModal(null)
        } catch (error) {
          console.error("Failed to delete template:", error)
          setConfirmModal(null)
          setAlertModal({
            isOpen: true,
            title: "Error",
            message: `Failed to delete template: ${error}`,
            type: "danger"
          })
        }
      }
    })
  }

  const handleApplyTemplate = async (templateId: string, instanceName: string) => {
    try {
      await invoke("apply_template_to_instance", { templateId, instanceName })
      setShowApplyMenu(false)
      setSelectedTemplateId(null)
      setAlertModal({
        isOpen: true,
        title: "Success",
        message: `Template applied to ${instanceName} successfully!`,
        type: "success"
      })
    } catch (error) {
      console.error("Failed to apply template:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to apply template: ${error}`,
        type: "danger"
      })
    }
  }

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString()
  }

  const filteredInstances = instances.filter(instance =>
    instance.name.toLowerCase().includes(searchQuery.toLowerCase())
  )

  return (
    <>
      <div className="p-6 space-y-4">
        <div className="max-w-7xl mx-auto">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h1 className="text-2xl font-semibold text-[#e8e8e8] tracking-tight">Instances</h1>
              <p className="text-sm text-[#808080] mt-0.5">Manage all your instances</p>
            </div>
            <div className="flex items-center gap-2">
              <button
                ref={templateButtonRef}
                onClick={() => {
                  setShowTemplateMenu(!showTemplateMenu)
                  setShowApplyMenu(false)
                }}
                className="w-10 h-10 hover:bg-[#1a1a1a] text-[#e8e8e8] rounded-lg flex items-center justify-center transition-all cursor-pointer"
                title="Templates"
              >
                <FileText size={24} strokeWidth={2} />
              </button>
              <button
                onClick={onCreateNew}
                className="w-10 h-10 hover:bg-[#1a1a1a] text-[#e8e8e8] rounded-lg flex items-center justify-center transition-all cursor-pointer"
                title="New Instance"
              >
                <Plus size={28} strokeWidth={2} />
              </button>
            </div>
          </div>

          {instances.length > 0 && (
            <div className="relative mb-4">
              <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-[#4a4a4a]" strokeWidth={2} />
              <input
                type="text"
                placeholder="Search instances..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full bg-[#1a1a1a] rounded-lg pl-10 pr-4 py-2.5 text-sm text-[#e8e8e8] placeholder-[#4a4a4a] focus:outline-none focus:ring-2 focus:ring-[#16a34a] transition-all"
              />
            </div>
          )}

          {instances.length === 0 ? (
            <div className="flex flex-col items-center justify-center min-h-[calc(100vh-300px)]">
              <Package size={64} className="text-[#16a34a] mb-4" strokeWidth={1.5} />
              <h3 className="text-lg font-semibold text-[#e8e8e8] mb-1">No instances yet</h3>
              <p className="text-sm text-[#808080] mb-4">Create your first instance to get started</p>
              <button
                onClick={onCreateNew}
                className="px-4 py-2 bg-[#16a34a] hover:bg-[#15803d] text-white rounded-lg font-medium text-sm flex items-center gap-2 transition-all cursor-pointer"
              >
                <Plus size={16} strokeWidth={2} />
                <span>Create Instance</span>
              </button>
            </div>
          ) : filteredInstances.length === 0 ? (
            <div className="bg-[#1a1a1a] rounded-xl p-12 flex flex-col items-center justify-center">
              <Search size={64} className="text-[#16a34a] mb-4" strokeWidth={1.5} />
              <h3 className="text-lg font-semibold text-[#e8e8e8] mb-1">No instances found</h3>
              <p className="text-sm text-[#808080]">Try adjusting your search query</p>
            </div>
          ) : (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 2xl:grid-cols-7 gap-3">
              {filteredInstances.map((instance) => {
                const icon = instanceIcons[instance.name]
                const isLaunching = launchingInstanceName === instance.name
                return (
                  <div
                    key={instance.name}
                    onClick={() => {
                      onSetSelectedInstance(instance)
                      onShowDetails(instance)
                    }}
                    onContextMenu={(e) => handleContextMenu(e, instance)}
                    className="group relative bg-[#1a1a1a] rounded-xl overflow-hidden cursor-pointer transition-all hover:ring-2 hover:ring-[#2a2a2a]"
                  >
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
                      
                      {isAuthenticated && (
                        <button
                          onClick={(e) => {
                            e.stopPropagation()
                            handleQuickLaunch(instance)
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

      {/* Template Menu Dropdown */}
      {showTemplateMenu && !showApplyMenu && createPortal(
        <div
          ref={templateMenuRef}
          className="fixed w-80 bg-[#1a1a1a] rounded-lg z-[9999] overflow-hidden"
          style={{
            top: `${(templateButtonRef.current?.getBoundingClientRect().bottom || 0) + 8}px`,
            right: `${window.innerWidth - (templateButtonRef.current?.getBoundingClientRect().right || 0)}px`
          }}
        >
          <div className="p-4 pb-3">
            <h3 className="text-base font-semibold text-[#e8e8e8]">Templates</h3>
            <p className="text-xs text-[#808080] mt-1">Manage instance templates</p>
          </div>
          <div className="h-px bg-[#2a2a2a]" />
          
          <button
            onClick={() => {
              setShowTemplateMenu(false)
              setShowApplyMenu(true)
            }}
            disabled={templates.length === 0 || instances.length === 0}
            className="w-full p-3 hover:bg-[#2a2a2a] transition-colors text-left cursor-pointer flex items-center gap-3 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <FileText size={22} className="text-[#16a34a]" strokeWidth={2} />
            <span className="text-sm font-medium text-[#e8e8e8]">Apply Template</span>
          </button>
          <div className="h-px bg-[#2a2a2a]" />

            <div className="px-3 pt-3 pb-2">
              <p className="text-xs text-[#808080]">Create from instance:</p>
            </div>

            <div className="bg-[#0f0f0f]">
              <div className="max-h-64 overflow-y-auto">
                {instances.map((instance) => (
                  <button
                    key={instance.name}
                    onClick={() => handleCreateTemplate(instance.name)}
                    className="w-full p-3 hover:bg-[#1a1a1a] transition-colors text-left cursor-pointer flex items-center justify-between group"
                  >
                    <div className="flex items-center gap-3">
                      <Download size={22} className="text-[#16a34a]" strokeWidth={2} />
                      <div>
                        <div className="text-sm font-medium text-[#e8e8e8]">{instance.name}</div>
                        <div className="text-xs text-[#808080]">{instance.version}</div>
                      </div>
                    </div>
                  </button>
                ))}
              </div>
            </div>

          <div className="h-px bg-[#2a2a2a]" />
          
          <div className="p-3">
            <button
              onClick={handleImportTemplate}
              className="w-full h-12 hover:bg-[#2a2a2a] text-[#16a34a] rounded-lg flex items-center justify-center gap-3 transition-all cursor-pointer"
            >
              <FileDown size={22} strokeWidth={2} />
              <span className="text-sm font-medium text-[#e8e8e8]">Import Template</span>
            </button>
          </div>
        </div>,
        document.body
      )}

      {/* Apply Template Menu */}
      {showApplyMenu && createPortal(
        <div
          ref={applyMenuRef}
          className="fixed w-80 bg-[#1a1a1a] rounded-lg z-[9999] overflow-hidden"
          style={{
            top: `${(templateButtonRef.current?.getBoundingClientRect().bottom || 0) + 8}px`,
            right: `${window.innerWidth - (templateButtonRef.current?.getBoundingClientRect().right || 0)}px`
          }}
        >
          <div className="p-4 pb-3">
            <h3 className="text-base font-semibold text-[#e8e8e8]">Apply Template</h3>
            <p className="text-xs text-[#808080] mt-1">Select a template and instance</p>
          </div>
          <div className="h-px bg-[#2a2a2a]" />
          
          <div className="max-h-64 overflow-y-auto">
            {templates.map((template) => (
              <div key={template.id} className="last:border-0">
                <button
                  onClick={() => setSelectedTemplateId(selectedTemplateId === template.id ? null : template.id)}
                  className="w-full p-3 hover:bg-[#2a2a2a] transition-colors text-left cursor-pointer"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <FileText size={24} className="text-[#16a34a]" strokeWidth={2} />
                      <div>
                        <div className="text-sm font-medium text-[#e8e8e8]">{template.name}</div>
                        <div className="text-xs text-[#808080]">{formatDate(template.created_at)}</div>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <ChevronDown 
                        size={16} 
                        className={`text-[#808080] transition-transform ${selectedTemplateId === template.id ? 'rotate-180' : ''}`} 
                      />
                      <button
                        onClick={(e) => {
                          e.stopPropagation()
                          handleExportTemplate(template.id, template.name)
                        }}
                        className="p-1 hover:bg-[#1f1f1f] text-[#808080] hover:text-[#16a34a] rounded transition-all cursor-pointer"
                        title="Export Template"
                      >
                        <FileUp size={14} strokeWidth={2} />
                      </button>
                      <button
                        onClick={(e) => {
                          e.stopPropagation()
                          handleDeleteTemplate(template.id, template.name)
                        }}
                        className="p-1 hover:bg-[#1f1f1f] text-[#808080] hover:text-red-400 rounded transition-all cursor-pointer"
                        title="Delete Template"
                      >
                        <Trash2 size={14} strokeWidth={2} />
                      </button>
                    </div>
                  </div>
                </button>
                
                {selectedTemplateId === template.id && (
                  <div className="bg-[#0f0f0f]">
                    {instances.map((instance) => {
                      const icon = instanceIcons[instance.name]
                      return (
                        <button
                          key={instance.name}
                          onClick={() => handleApplyTemplate(template.id, instance.name)}
                          className="w-full p-2.5 pl-3 pr-3 hover:bg-[#1a1a1a] transition-colors text-left cursor-pointer flex items-center gap-3 group"
                        >
                          <div className="w-10 h-10 bg-[#1a1a1a] rounded-lg flex items-center justify-center flex-shrink-0 overflow-hidden">
                            {icon ? (
                              <img
                                src={icon}
                                alt={instance.name}
                                className="w-full h-full object-contain p-1"
                              />
                            ) : (
                              <Package size={20} className="text-[#4a4a4a]" strokeWidth={1.5} />
                            )}
                          </div>
                          <div className="flex-1">
                            <div className="text-sm text-[#e8e8e8]">{instance.name}</div>
                            <div className="text-xs text-[#808080]">{instance.version}</div>
                          </div>
                          <Download size={16} className="text-[#808080] group-hover:text-[#16a34a] transition-colors flex-shrink-0" />
                        </button>
                      )
                    })}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>,
        document.body
      )}

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
                if (onOpenFolder) {
                  onOpenFolder(contextMenu.instance)
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
                if (onDeleteInstance) {
                  onDeleteInstance(contextMenu.instance.name)
                }
              },
              danger: true,
            },
          ]}
        />
      )}

      {/* Template Creation Toast */}
      {creatingTemplate && (
        <CreationProgressToast
          instanceName={`Template: ${creatingTemplate}`}
          onComplete={() => setCreatingTemplate(null)}
          onError={() => setCreatingTemplate(null)}
          onDismiss={() => setCreatingTemplate(null)}
        />
      )}

      {/* Confirmation Modal */}
      {confirmModal && (
        <ConfirmModal
          isOpen={confirmModal.isOpen}
          title={confirmModal.title}
          message={confirmModal.message}
          type={confirmModal.type}
          confirmText="Delete"
          onConfirm={confirmModal.onConfirm}
          onCancel={() => setConfirmModal(null)}
        />
      )}

      {/* Alert Modal */}
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