import { useState, useRef, useEffect } from "react"
import { X, Trash2, Camera, ImagePlus, Loader2, Settings } from "lucide-react"
import { invoke } from "@tauri-apps/api/core"
import { ConfirmModal, AlertModal } from "./ConfirmModal"
import type { Instance, FabricVersion } from "../../types"

interface InstanceSettingsModalProps {
  isOpen: boolean
  instance: Instance
  instanceIcon: string | null
  onClose: () => void
  onInstanceUpdated: () => void
  onInstanceDeleted: () => void
}

export function InstanceSettingsModal({
  isOpen,
  instance,
  instanceIcon,
  onClose,
  onInstanceUpdated,
  onInstanceDeleted,
}: InstanceSettingsModalProps) {
  const [isDeleting, setIsDeleting] = useState(false)
  const [newName, setNewName] = useState(instance.name)
  const [renameError, setRenameError] = useState<string | null>(null)
  const [isRenamingInstance, setIsRenamingInstance] = useState(false)
  const [isUploadingIcon, setIsUploadingIcon] = useState(false)
  const [localIcon, setLocalIcon] = useState<string | null>(instanceIcon)
  const fileInputRef = useRef<HTMLInputElement>(null)
  
  // Fabric loader state
  const [fabricVersions, setFabricVersions] = useState<FabricVersion[]>([])
  const [selectedFabricVersion, setSelectedFabricVersion] = useState<string>(instance.loader_version || "")
  const [isLoadingFabric, setIsLoadingFabric] = useState(false)
  const [isUpdatingFabric, setIsUpdatingFabric] = useState(false)
  
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

  const isFabricInstance = instance.loader === "fabric"

  useEffect(() => {
    if (isOpen) {
      setLocalIcon(instanceIcon)
      setNewName(instance.name)
      setSelectedFabricVersion(instance.loader_version || "")
      
      if (isFabricInstance && fabricVersions.length === 0) {
        loadFabricVersions()
      }
    }
  }, [isOpen, instanceIcon, instance.name, instance.loader_version, isFabricInstance])

  const loadFabricVersions = async () => {
    setIsLoadingFabric(true)
    try {
      const versions = await invoke<FabricVersion[]>("get_fabric_versions")
      setFabricVersions(versions)
    } catch (error) {
      console.error("Failed to load Fabric versions:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to load Fabric versions: ${error}`,
        type: "danger"
      })
    } finally {
      setIsLoadingFabric(false)
    }
  }

  const handleUpdateFabricLoader = async (newVersion: string) => {
    if (!newVersion || newVersion === instance.loader_version) {
      return
    }

    setIsUpdatingFabric(true)
    try {
      await invoke("update_instance_fabric_loader", {
        instanceName: instance.name,
        fabricVersion: newVersion
      })
      
      onInstanceUpdated()
    } catch (error) {
      console.error("Failed to update Fabric loader:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to update Fabric loader: ${error}`,
        type: "danger"
      })
      // Reset selection on error
      setSelectedFabricVersion(instance.loader_version || "")
    } finally {
      setIsUpdatingFabric(false)
    }
  }

  if (!isOpen) return null

  const handleIconClick = () => {
    fileInputRef.current?.click()
  }

  const handleIconChange = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    if (!file) return

    if (!file.type.startsWith('image/')) {
      setAlertModal({
        isOpen: true,
        title: "Invalid File",
        message: "Please select an image file (PNG, JPEG, or WebP)",
        type: "danger"
      })
      return
    }

    if (file.size > 5 * 1024 * 1024) {
      setAlertModal({
        isOpen: true,
        title: "File Too Large",
        message: "Image must be smaller than 5MB",
        type: "danger"
      })
      return
    }

    setIsUploadingIcon(true)

    try {
      const reader = new FileReader()
      reader.onload = async () => {
        const base64 = (reader.result as string).split(',')[1]
        
        try {
          await invoke("set_instance_icon", {
            instanceName: instance.name,
            imageData: base64
          })
          
          const newIcon = await invoke<string | null>("get_instance_icon", {
            instanceName: instance.name
          })
          setLocalIcon(newIcon)
          onInstanceUpdated()
        } catch (error) {
          console.error("Failed to set icon:", error)
          setAlertModal({
            isOpen: true,
            title: "Error",
            message: `Failed to set icon: ${error}`,
            type: "danger"
          })
        } finally {
          setIsUploadingIcon(false)
        }
      }
      reader.readAsDataURL(file)
    } catch (error) {
      console.error("Failed to read file:", error)
      setIsUploadingIcon(false)
    }

    if (event.target) {
      event.target.value = ''
    }
  }

  const handleRemoveIcon = async () => {
    setConfirmModal({
      isOpen: true,
      title: "Remove Icon",
      message: "Are you sure you want to remove this instance icon?",
      type: "warning",
      onConfirm: async () => {
        setConfirmModal(null)
        try {
          await invoke("remove_instance_icon", {
            instanceName: instance.name
          })
          setLocalIcon(null)
          onInstanceUpdated()
        } catch (error) {
          console.error("Failed to remove icon:", error)
          setAlertModal({
            isOpen: true,
            title: "Error",
            message: `Failed to remove icon: ${error}`,
            type: "danger"
          })
        }
      }
    })
  }

  const handleRename = async (trimmedName: string) => {
    if (!trimmedName) {
      setRenameError("Instance name cannot be empty")
      return
    }

    if (trimmedName === instance.name) {
      setRenameError(null)
      return
    }

    setIsRenamingInstance(true)
    try {
      await invoke("rename_instance", {
        oldName: instance.name,
        newName: trimmedName
      })
      setRenameError(null)
      onInstanceUpdated()
      onClose()
    } catch (error) {
      setRenameError(error as string)
      setNewName(instance.name)
    } finally {
      setIsRenamingInstance(false)
    }
  }

  const handleDelete = async () => {
    setConfirmModal({
      isOpen: true,
      title: "Delete Instance",
      message: `Are you sure you want to delete "${instance.name}"?\n\nThis action cannot be undone.`,
      type: "danger",
      onConfirm: async () => {
        setIsDeleting(true)
        setConfirmModal(null)
        try {
          await invoke("delete_instance", { instanceName: instance.name })
          onInstanceDeleted()
          onClose()
        } catch (error) {
          console.error("Failed to delete instance:", error)
          setAlertModal({
            isOpen: true,
            title: "Error",
            message: `Failed to delete instance: ${error}`,
            type: "danger"
          })
        } finally {
          setIsDeleting(false)
        }
      }
    })
  }

  return (
    <>
      <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4">
        <div className="bg-[#1a1a1a] rounded-xl w-full max-w-md shadow-2xl">
          {/* Header */}
          <div className="flex items-center justify-between p-5">
            <div className="flex items-center gap-3">
              <Settings size={32} className="text-[#16a34a]" strokeWidth={1.5} />
              <div>
                <h2 className="text-base font-semibold text-[#e8e8e8] tracking-tight">Instance Settings</h2>
                <p className="text-xs text-[#808080] mt-0.5">Manage your instance configuration</p>
              </div>
            </div>
            <button
              onClick={onClose}
              className="p-1.5 hover:bg-[#0d0d0d] rounded-lg transition-colors text-[#808080] hover:text-[#e8e8e8] cursor-pointer"
            >
              <X size={16} strokeWidth={2} />
            </button>
          </div>

          {/* Content */}
          <div className="p-5 space-y-4">
            {/* Instance Icon Section */}
            <div>
              <label className="block text-xs font-medium text-[#808080] mb-2">Instance Icon</label>
              <div className="flex items-center gap-4">
                <input
                  ref={fileInputRef}
                  type="file"
                  accept="image/*"
                  onChange={handleIconChange}
                  className="hidden"
                />
                
                {localIcon ? (
                  <div className="relative group">
                    <button
                      onClick={handleIconClick}
                      disabled={isUploadingIcon}
                      className="w-20 h-20 rounded-xl overflow-hidden relative cursor-pointer bg-transparent"
                    >
                      <img
                        src={localIcon}
                        alt={instance.name}
                        className="w-full h-full object-cover"
                      />
                      <div className="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center">
                        <Camera size={24} className="text-white" />
                      </div>
                    </button>
                  </div>
                ) : (
                  <button
                    onClick={handleIconClick}
                    disabled={isUploadingIcon}
                    className="w-20 h-20 border-2 border-dashed border-[#2a2a2a] hover:border-[#16a34a]/50 rounded-xl flex items-center justify-center transition-all bg-transparent cursor-pointer"
                  >
                    {isUploadingIcon ? (
                      <Loader2 size={28} className="text-[#16a34a] animate-spin" />
                    ) : (
                      <ImagePlus size={28} className="text-[#4a4a4a] hover:text-[#16a34a] transition-colors" />
                    )}
                  </button>
                )}
                
                <div className="flex-1 space-y-2">
                  <button
                    onClick={handleIconClick}
                    disabled={isUploadingIcon}
                    className="w-full px-4 py-2 bg-[#0d0d0d] hover:bg-[#2a2a2a] text-[#e8e8e8] rounded-lg text-sm font-medium transition-all disabled:opacity-50 cursor-pointer"
                  >
                    {localIcon ? "Change Icon" : "Upload Icon"}
                  </button>
                  {localIcon && (
                    <button
                      onClick={handleRemoveIcon}
                      disabled={isUploadingIcon}
                      className="w-full px-4 py-2 bg-[#0d0d0d] hover:bg-red-500/10 text-[#808080] hover:text-red-400 rounded-lg text-sm font-medium transition-all disabled:opacity-50 cursor-pointer"
                    >
                      Remove Icon
                    </button>
                  )}
                </div>
              </div>
            </div>

            {/* Instance Name Section */}
            <div>
              <label className="block text-xs font-medium text-[#808080] mb-2">Instance Name</label>
              <div className="relative">
                <input
                  type="text"
                  value={newName}
                  onChange={(e) => {
                    setNewName(e.target.value)
                    setRenameError(null)
                  }}
                  onBlur={(e) => {
                    const trimmedName = e.target.value.trim()
                    if (trimmedName && trimmedName !== instance.name) {
                      handleRename(trimmedName)
                    }
                  }}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.currentTarget.blur()
                    }
                  }}
                  className="w-full bg-[#0d0d0d] rounded-lg px-3 py-2.5 pr-10 text-sm text-[#e8e8e8] placeholder-[#4a4a4a] focus:outline-none focus:ring-1 focus:ring-[#16a34a] transition-colors"
                  placeholder="Enter instance name"
                  disabled={isRenamingInstance}
                />
                {isRenamingInstance && (
                  <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">
                    <Loader2 size={14} className="animate-spin text-[#16a34a]" />
                  </div>
                )}
              </div>
              {renameError && (
                <p className="text-xs text-red-400 mt-2">{renameError}</p>
              )}
            </div>

            {/* Fabric Loader Section */}
            {isFabricInstance && (
              <div>
                <label className="block text-xs font-medium text-[#808080] mb-2">Fabric Loader Version</label>
                {isLoadingFabric ? (
                  <div className="flex items-center gap-2 text-[#808080] text-xs py-2 px-3 bg-[#0d0d0d] rounded-lg">
                    <Loader2 size={14} className="animate-spin text-[#3b82f6]" />
                    <span>Loading versions...</span>
                  </div>
                ) : (
                  <>
                    <div className="relative">
                      <select
                        value={selectedFabricVersion}
                        onChange={(e) => {
                          const newVersion = e.target.value
                          setSelectedFabricVersion(newVersion)
                          handleUpdateFabricLoader(newVersion)
                        }}
                        className="w-full bg-[#0d0d0d] rounded-lg px-3 py-2.5 pr-10 text-sm text-[#e8e8e8] focus:outline-none focus:ring-1 focus:ring-[#3b82f6] transition-colors appearance-none"
                        disabled={isUpdatingFabric}
                      >
                        {fabricVersions.map((version) => (
                          <option key={version.version} value={version.version}>
                            {version.version} {version.stable ? "(Stable)" : ""}
                          </option>
                        ))}
                      </select>
                      <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">
                        {isUpdatingFabric ? (
                          <Loader2 size={14} className="animate-spin text-[#3b82f6]" />
                        ) : (
                          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#808080" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                            <polyline points="6 9 12 15 18 9"></polyline>
                          </svg>
                        )}
                      </div>
                    </div>
                  </>
                )}
              </div>
            )}

            {/* Danger Zone */}
            <div className="pt-4 border-t border-[#2a2a2a]">
              <button
                onClick={handleDelete}
                disabled={isDeleting}
                className="w-full px-4 py-2.5 bg-red-500/10 hover:bg-red-500/20 text-red-400 rounded-lg font-medium text-sm flex items-center justify-center gap-2 transition-all disabled:opacity-50 cursor-pointer"
              >
                {isDeleting ? (
                  <>
                    <Loader2 size={16} className="animate-spin" />
                    <span>Deleting...</span>
                  </>
                ) : (
                  <>
                    <Trash2 size={16} />
                    <span>Delete Instance</span>
                  </>
                )}
              </button>
            </div>
          </div>
        </div>
      </div>

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