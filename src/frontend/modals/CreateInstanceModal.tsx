import { useState, useEffect } from "react"
import { invoke } from "@tauri-apps/api/core"
import { open } from '@tauri-apps/plugin-dialog'
import { X, Loader2, Package, AlertCircle, FileDown } from "lucide-react"
import { AlertModal } from "./ConfirmModal"
import type { FabricVersion, Instance } from "../../types"

interface MinecraftVersion {
  id: string
  type: "release" | "snapshot" | "old_beta" | "old_alpha"
  url: string
  time: string
  releaseTime: string
}

interface CreateInstanceModalProps {
  versions: string[]
  instances: Instance[]
  onClose: () => void
  onSuccess: () => void
  onStartCreating: (instanceName: string) => void
}

export function CreateInstanceModal({ versions, instances, onClose, onSuccess, onStartCreating }: CreateInstanceModalProps) {
  const [isCreating, setIsCreating] = useState(false)
  const [selectedVersion, setSelectedVersion] = useState(versions[0] || "1.21.11")
  const [newInstanceName, setNewInstanceName] = useState("")
  const [loaderType, setLoaderType] = useState<"vanilla" | "fabric">("vanilla")
  const [fabricVersions, setFabricVersions] = useState<FabricVersion[]>([])
  const [selectedFabricVersion, setSelectedFabricVersion] = useState<string>("")
  const [isLoadingFabric, setIsLoadingFabric] = useState(false)
  const [isClosing, setIsClosing] = useState(false)
  const [alertModal, setAlertModal] = useState<{
    isOpen: boolean
    title: string
    message: string
    type: "warning" | "danger" | "success" | "info"
  } | null>(null)

  const [versionFilter, setVersionFilter] = useState<"release" | "snapshot">("release")
  const [allVersions, setAllVersions] = useState<MinecraftVersion[]>([])
  const [isLoadingVersions, setIsLoadingVersions] = useState(false)
  const [fabricSupportedVersions, setFabricSupportedVersions] = useState<string[]>([])

  const instanceExists = instances.some(
    instance => instance.name.toLowerCase() === newInstanceName.trim().toLowerCase()
  )

  useEffect(() => {
    loadVersionsWithMetadata()
    loadFabricSupportedVersions()
  }, [])

  const loadVersionsWithMetadata = async () => {
    setIsLoadingVersions(true)
    try {
      const versionsData = await invoke<MinecraftVersion[]>("get_minecraft_versions_with_metadata")
      setAllVersions(versionsData)
      
      const firstRelease = versionsData.find(v => v.type === "release")
      if (firstRelease) {
        setSelectedVersion(firstRelease.id)
      }
    } catch (error) {
      console.error("Failed to load versions:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to load versions: ${error}`,
        type: "danger"
      })
    } finally {
      setIsLoadingVersions(false)
    }
  }

  const loadFabricSupportedVersions = async () => {
    try {
      const supported = await invoke<string[]>("get_supported_game_versions")
      setFabricSupportedVersions(supported)
    } catch (error) {
      console.error("Failed to load Fabric supported versions:", error)
    }
  }

  const getFilteredVersions = () => {
    let filtered: MinecraftVersion[]
    
    if (versionFilter === "snapshot") {
      filtered = allVersions.filter(v => v.type === "snapshot")
    } else {
      filtered = allVersions.filter(v => v.type === "release" || v.type === "old_beta" || v.type === "old_alpha")
    }

    if (loaderType === "fabric" && versionFilter === "release") {
      filtered = filtered.filter(v => fabricSupportedVersions.includes(v.id))
    }

    return filtered
  }

  const filteredVersions = getFilteredVersions()

  useEffect(() => {
    if (loaderType === "fabric") {
      if (versionFilter === "snapshot") {
        setVersionFilter("release")
      }
      
      if (!fabricSupportedVersions.includes(selectedVersion)) {
        const firstSupported = allVersions.find(v => 
          (v.type === "release" || v.type === "old_beta" || v.type === "old_alpha") && 
          fabricSupportedVersions.includes(v.id)
        )
        if (firstSupported) {
          setSelectedVersion(firstSupported.id)
        }
      }
      
      if (fabricVersions.length === 0) {
        loadFabricVersions()
      }
    }
  }, [loaderType, fabricSupportedVersions, allVersions, versionFilter])

  const loadFabricVersions = async () => {
    setIsLoadingFabric(true)
    try {
      const versions = await invoke<FabricVersion[]>("get_fabric_versions")
      setFabricVersions(versions)
      const stableVersion = versions.find(v => v.stable)
      if (stableVersion) {
        setSelectedFabricVersion(stableVersion.version)
      } else if (versions.length > 0) {
        setSelectedFabricVersion(versions[0].version)
      }
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

  const handleClose = () => {
    setIsClosing(true)
    setTimeout(() => {
      setIsClosing(false)
      onClose()
    }, 150)
  }

  const handleImportFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'Instance Files',
          extensions: ['mrpack', 'zip']
        }]
      })

      if (!selected) return

      const filePath = selected as string
      
      handleClose()

      let extractedName = ""
      try {
        extractedName = await invoke<string>("get_modpack_name_from_file", {
          filePath: filePath
        })
      } catch (error) {
        console.error("Failed to extract name from file:", error)
        const fileName = filePath.split(/[/\\]/).pop()?.replace(/\.(mrpack|zip)$/, '') || "Imported Instance"
        extractedName = fileName
      }

      let finalName = extractedName
      let counter = 1
      while (instances.some(i => i.name.toLowerCase() === finalName.toLowerCase())) {
        finalName = `${extractedName} (${counter})`
        counter++
      }

      setIsCreating(true)
      onStartCreating(finalName)

      await invoke("install_modpack_from_file", {
        filePath: filePath,
        instanceName: finalName,
        preferredGameVersion: null,
      })

      onSuccess()
    } catch (error) {
      console.error("Import error:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to import instance: ${error}`,
        type: "danger"
      })
    } finally {
      setIsCreating(false)
    }
  }

  const handleCreateInstance = async () => {
    if (!newInstanceName.trim() || instanceExists) return

    setIsCreating(true)
    const finalName = newInstanceName.trim()

    onStartCreating(finalName)
    handleClose()
    
    try {
      await invoke<string>("create_instance", {
        instanceName: finalName,
        version: selectedVersion,
        loader: loaderType === "vanilla" ? null : loaderType,
        loaderVersion: loaderType === "fabric" ? selectedFabricVersion : null,
      })

      onSuccess()
    } catch (error) {
      console.error("Create instance error:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to create instance: ${error}`,
        type: "danger"
      })
    } finally {
      setIsCreating(false)
    }
  }

  const isCreateDisabled = isCreating || 
    !newInstanceName.trim() || 
    instanceExists ||
    (loaderType === "fabric" && !selectedFabricVersion) ||
    isLoadingVersions ||
    filteredVersions.length === 0

  return (
    <>
      <style>{`
        @keyframes fadeIn {
          from { opacity: 0; }
          to { opacity: 1; }
        }
        @keyframes fadeOut {
          from { opacity: 1; }
          to { opacity: 0; }
        }
        @keyframes scaleIn {
          from { 
            opacity: 0;
            transform: scale(0.95);
          }
          to { 
            opacity: 1;
            transform: scale(1);
          }
        }
        @keyframes scaleOut {
          from { 
            opacity: 1;
            transform: scale(1);
          }
          to { 
            opacity: 0;
            transform: scale(0.95);
          }
        }
        .modal-backdrop {
          animation: fadeIn 0.15s ease-out forwards;
        }
        .modal-backdrop.closing {
          animation: fadeOut 0.15s ease-in forwards;
        }
        .modal-content {
          animation: scaleIn 0.15s ease-out forwards;
        }
        .modal-content.closing {
          animation: scaleOut 0.15s ease-in forwards;
        }
      `}</style>
      <div 
        className={`fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4 modal-backdrop ${isClosing ? 'closing' : ''}`}
        onClick={handleClose}
      >
        <div 
          className={`bg-[#141414] border border-[#2a2a2a] rounded-md w-full max-w-md shadow-2xl modal-content ${isClosing ? 'closing' : ''}`}
          onClick={(e) => e.stopPropagation()}
        >
          <div className="flex items-center justify-between p-5">
            <div className="flex items-center gap-3">
              <Package size={24} className="text-[#16a34a]" strokeWidth={1.5} />
              <div>
                <h2 className="text-base font-semibold text-[#e6edf3] tracking-tight">Create Instance</h2>
                <p className="text-xs text-[#7d8590] mt-0.5">Set up a new Minecraft instance</p>
              </div>
            </div>
            <button 
              onClick={handleClose} 
              className="p-1.5 hover:bg-[#1a1a1a] rounded transition-colors text-[#7d8590] hover:text-[#e6edf3] cursor-pointer"
            >
              <X size={16} strokeWidth={2} />
            </button>
          </div>

          <div className="p-5 space-y-4">
            <div className="grid grid-cols-2 gap-3">
              <button
                type="button"
                onClick={() => {}}
                disabled={isCreating}
                className="px-4 py-3 rounded text-sm font-medium transition-all cursor-pointer bg-[#16a34a]/10 border-2 border-[#16a34a] text-[#e6edf3]"
              >
                <div className="flex items-center justify-center gap-2">
                  <Package size={20} className="text-[#16a34a]" strokeWidth={1.5} />
                  <span>Custom</span>
                </div>
              </button>
              <button
                type="button"
                onClick={handleImportFile}
                disabled={isCreating}
                className="px-4 py-3 rounded text-sm font-medium transition-all cursor-pointer bg-[#0f0f0f] border border-[#2a2a2a] text-[#7d8590] hover:bg-[#1a1a1a] hover:border-[#3a3a3a]"
              >
                <div className="flex items-center justify-center gap-2">
                  <FileDown size={20} className="text-[#3a3a3a]" strokeWidth={1.5} />
                  <span>Import File</span>
                </div>
              </button>
            </div>

            <div>
              <label className="block text-xs font-medium text-[#7d8590] mb-2">Instance Name</label>
              <input
                type="text"
                value={newInstanceName}
                onChange={(e) => setNewInstanceName(e.target.value)}
                placeholder="My Minecraft Instance"
                className={`w-full bg-[#0f0f0f] border rounded px-3 py-2.5 text-sm text-[#e6edf3] placeholder-[#3a3a3a] focus:outline-none transition-colors ${
                  instanceExists && newInstanceName.trim()
                    ? 'border-red-500/50 focus:border-red-500'
                    : 'border-[#2a2a2a] focus:border-[#16a34a]'
                }`}
                disabled={isCreating}
              />
              {instanceExists && newInstanceName.trim() && (
                <div className="flex items-center gap-1.5 mt-2 text-xs text-red-400">
                  <AlertCircle size={12} strokeWidth={2} />
                  <span>An instance with this name already exists</span>
                </div>
              )}
            </div>

            <div>
              <label className="block text-xs font-medium text-[#7d8590] mb-2">Version Type</label>
              <div className="flex gap-2 mb-3">
                <button
                  type="button"
                  onClick={() => {
                    setVersionFilter("release")
                    const releases = allVersions.filter(v => 
                      v.type === "release" || v.type === "old_beta" || v.type === "old_alpha"
                    )
                    let availableReleases = releases
                    
                    if (loaderType === "fabric") {
                      availableReleases = releases.filter(v => fabricSupportedVersions.includes(v.id))
                    }
                    
                    if (availableReleases.length > 0) {
                      setSelectedVersion(availableReleases[0].id)
                    }
                  }}
                  className={`px-6 py-1.5 rounded text-sm font-medium transition-all cursor-pointer border ${
                    versionFilter === "release"
                      ? "bg-[#16a34a]/10 border-[#16a34a] text-[#e6edf3]"
                      : "bg-[#0f0f0f] border-[#2a2a2a] text-[#7d8590] hover:bg-[#1a1a1a] hover:border-[#3a3a3a]"
                  }`}
                >
                  Releases
                </button>
                <button
                  type="button"
                  onClick={() => {
                    if (loaderType !== "vanilla") return
                    setVersionFilter("snapshot")
                    const snapshots = allVersions.filter(v => v.type === "snapshot")
                    if (snapshots.length > 0) {
                      setSelectedVersion(snapshots[0].id)
                    }
                  }}
                  disabled={loaderType !== "vanilla"}
                  className={`px-6 py-1.5 rounded text-sm font-medium transition-all cursor-pointer border ${
                    versionFilter === "snapshot"
                      ? "bg-[#eab308]/10 border-[#eab308] text-[#e6edf3]"
                      : "bg-[#0f0f0f] border-[#2a2a2a] text-[#7d8590] hover:bg-[#1a1a1a] hover:border-[#3a3a3a]"
                  } ${loaderType !== "vanilla" ? "opacity-50 cursor-not-allowed" : ""}`}
                >
                  Snapshots
                </button>
              </div>

              <label className="block text-xs font-medium text-[#7d8590] mb-2">
                Minecraft Version
              </label>
              {isLoadingVersions ? (
                <div className="flex items-center gap-2 text-[#7d8590] text-xs py-2 px-3 bg-[#0f0f0f] border border-[#2a2a2a] rounded">
                  <Loader2 size={14} className="animate-spin" />
                  <span>Loading versions...</span>
                </div>
              ) : filteredVersions.length === 0 ? (
                <div className="flex items-center gap-2 text-[#7d8590] text-xs py-2 px-3 bg-[#0f0f0f] border border-[#2a2a2a] rounded">
                  <AlertCircle size={14} />
                  <span>No compatible versions available</span>
                </div>
              ) : (
                <div className="relative">
                  <select
                    value={selectedVersion}
                    onChange={(e) => setSelectedVersion(e.target.value)}
                    className="w-full bg-[#0f0f0f] border border-[#2a2a2a] rounded px-3 py-2.5 pr-10 text-sm text-[#e6edf3] focus:outline-none focus:border-[#16a34a] transition-colors appearance-none"
                    disabled={isCreating}
                  >
                    {filteredVersions.map((version) => (
                      <option key={version.id} value={version.id}>
                        {version.id}
                      </option>
                    ))}
                  </select>
                  <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#7d8590" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <polyline points="6 9 12 15 18 9"></polyline>
                    </svg>
                  </div>
                </div>
              )}
            </div>

            <div>
              <label className="block text-xs font-medium text-[#7d8590] mb-2">Mod Loader</label>
              <div className="grid grid-cols-2 gap-2">
                <button
                  type="button"
                  onClick={() => setLoaderType("vanilla")}
                  disabled={isCreating}
                  className={`px-3 py-2.5 rounded text-sm font-medium transition-all cursor-pointer border ${
                    loaderType === "vanilla"
                      ? "bg-[#16a34a]/10 border-[#16a34a] text-[#e6edf3]"
                      : "bg-[#0f0f0f] border-[#2a2a2a] text-[#7d8590] hover:bg-[#1a1a1a] hover:border-[#3a3a3a]"
                  }`}
                >
                  <div className="flex items-center justify-center gap-2">
                    <Package size={18} className={loaderType === "vanilla" ? "text-[#16a34a]" : "text-[#3a3a3a]"} strokeWidth={1.5} />
                    <span>Vanilla</span>
                  </div>
                </button>
                <button
                  type="button"
                  onClick={() => setLoaderType("fabric")}
                  disabled={isCreating}
                  className={`px-3 py-2.5 rounded text-sm font-medium transition-all cursor-pointer border ${
                    loaderType === "fabric"
                      ? "bg-[#3b82f6]/10 border-[#3b82f6] text-[#e6edf3]"
                      : "bg-[#0f0f0f] border-[#2a2a2a] text-[#7d8590] hover:bg-[#1a1a1a] hover:border-[#3a3a3a]"
                  }`}
                >
                  <div className="flex items-center justify-center gap-2">
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" className={loaderType === "fabric" ? "text-[#3b82f6]" : "text-[#3a3a3a]"}>
                      <path d="M12 2L2 7L12 12L22 7L12 2Z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
                      <path d="M2 17L12 22L22 17" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
                      <path d="M2 12L12 17L22 12" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
                    </svg>
                    <span>Fabric</span>
                  </div>
                </button>
              </div>
            </div>

            {loaderType === "fabric" && (
              <div>
                <label className="block text-xs font-medium text-[#7d8590] mb-2">Fabric Loader Version</label>
                {isLoadingFabric ? (
                  <div className="flex items-center gap-2 text-[#7d8590] text-xs py-2 px-3 bg-[#0f0f0f] border border-[#2a2a2a] rounded">
                    <Loader2 size={14} className="animate-spin text-[#3b82f6]" />
                    <span>Loading versions...</span>
                  </div>
                ) : (
                  <div className="relative">
                    <select
                      value={selectedFabricVersion}
                      onChange={(e) => setSelectedFabricVersion(e.target.value)}
                      className="w-full bg-[#0f0f0f] border border-[#2a2a2a] rounded px-3 py-2.5 pr-10 text-sm text-[#e6edf3] focus:outline-none focus:border-[#3b82f6] transition-colors appearance-none"
                      disabled={isCreating}
                    >
                      {fabricVersions.map((version) => (
                        <option key={version.version} value={version.version}>
                          {version.version} {version.stable ? "(Stable)" : ""}
                        </option>
                      ))}
                    </select>
                    <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#7d8590" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <polyline points="6 9 12 15 18 9"></polyline>
                      </svg>
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>

          <div className="flex items-center justify-end gap-2 p-5">
            <button
              onClick={handleClose}
              disabled={isCreating}
              className="px-4 py-2 bg-[#1a1a1a] hover:bg-[#2a2a2a] text-[#e6edf3] rounded font-medium text-sm transition-colors disabled:opacity-50 cursor-pointer disabled:cursor-not-allowed border border-[#2a2a2a]"
            >
              Cancel
            </button>
            <button
              onClick={handleCreateInstance}
              disabled={isCreateDisabled}
              className="px-4 py-2 bg-[#16a34a] hover:bg-[#15803d] text-white rounded font-medium text-sm flex items-center gap-2 transition-colors disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer"
            >
              {isCreating ? (
                <>
                  <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  <span>Creating...</span>
                </>
              ) : (
                <span>Create Instance</span>
              )}
            </button>
          </div>
        </div>
      </div>

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