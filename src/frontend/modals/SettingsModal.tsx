import { useState, useEffect, useRef } from "react"
import { invoke } from "@tauri-apps/api/core"
import { Loader2, Coffee, Cpu, ImagePlus, FolderOpen, ChevronDown, X } from "lucide-react"
import { AlertModal } from "./ConfirmModal"
import type { LauncherSettings } from "../../types"

interface SystemInfo {
  total_memory_mb: number
  available_memory_mb: number
  recommended_max_memory_mb: number
}

interface SettingsModalProps {
  isOpen: boolean
  settings: LauncherSettings | null
  launcherDirectory: string
  onClose: () => void
  onSettingsChange: (settings: LauncherSettings) => void
  onBackgroundChanged?: () => void
}

export function SettingsModal({ 
  isOpen, 
  settings, 
  launcherDirectory, 
  onClose, 
  onSettingsChange, 
  onBackgroundChanged 
}: SettingsModalProps) {
  const [javaInstallations, setJavaInstallations] = useState<string[]>([])
  const [isLoadingJava, setIsLoadingJava] = useState(false)
  const [showCustomPath, setShowCustomPath] = useState(false)
  const [customPathValue, setCustomPathValue] = useState("")
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null)
  const [sidebarBgPreview, setSidebarBgPreview] = useState<string | null>(null)
  const [appVersion, setAppVersion] = useState<string>("")
  const fileInputRef = useRef<HTMLInputElement>(null)
  const [alertModal, setAlertModal] = useState<{
    isOpen: boolean
    title: string
    message: string
    type: "warning" | "danger" | "success" | "info"
  } | null>(null)
  const [isClosing, setIsClosing] = useState(false)

  useEffect(() => {
    if (isOpen) {
      loadSystemInfo()
      loadSidebarBackground()
      loadJavaInstallations()
      loadAppVersion()
    }
  }, [isOpen])

  useEffect(() => {
    if (!settings?.java_path) {
      setShowCustomPath(false)
      setCustomPathValue("")
      return
    }

    if (javaInstallations.length > 0) {
      const isCustomPath = !javaInstallations.includes(settings.java_path)
      setShowCustomPath(isCustomPath)
      if (isCustomPath) {
        setCustomPathValue(settings.java_path)
      } else {
        setCustomPathValue("")
      }
    }
  }, [settings?.java_path, javaInstallations])

  const loadAppVersion = async () => {
    try {
      const version = await invoke<string>("get_app_version")
      setAppVersion(version)
    } catch (error) {
      console.error("Failed to get app version:", error)
    }
  }

  const loadSystemInfo = async () => {
    try {
      const info = await invoke<SystemInfo>("get_system_info")
      setSystemInfo(info)
    } catch (error) {
      console.error("Failed to get system info:", error)
    }
  }

  const loadSidebarBackground = async () => {
    try {
      const bg = await invoke<string | null>("get_sidebar_background")
      setSidebarBgPreview(bg)
    } catch (error) {
      console.error("Failed to load sidebar background:", error)
    }
  }

  const loadJavaInstallations = async () => {
    setIsLoadingJava(true)
    try {
      const installations = await invoke<string[]>("detect_java_installations")
      setJavaInstallations(installations)
    } catch (error) {
      console.error("Failed to detect Java installations:", error)
    } finally {
      setIsLoadingJava(false)
    }
  }

  const handleSettingChange = async (newSettings: LauncherSettings) => {
    try {
      await invoke("save_settings", { settings: newSettings })
      await invoke("update_discord_rpc_mode")
      onSettingsChange(newSettings)
    } catch (error) {
      console.error("Failed to save settings:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to save settings: ${error}`,
        type: "danger"
      })
    }
  }

  const handleFileSelect = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    if (!file) return

    if (!file.type.startsWith('image/')) {
      setAlertModal({
        isOpen: true,
        title: "Invalid File",
        message: "Please select an image file (PNG, JPG, etc.)",
        type: "warning"
      })
      return
    }

    if (file.size > 10 * 1024 * 1024) {
      setAlertModal({
        isOpen: true,
        title: "File Too Large",
        message: "Image must be smaller than 10MB",
        type: "warning"
      })
      return
    }

    try {
      const reader = new FileReader()
      reader.onload = async (e) => {
        const base64 = e.target?.result as string
        
        try {
          await invoke("set_sidebar_background", { imageData: base64 })
          setSidebarBgPreview(base64)
          onBackgroundChanged?.()
        } catch (error) {
          console.error("Failed to save sidebar background:", error)
          setAlertModal({
            isOpen: true,
            title: "Error",
            message: `Failed to save background: ${error}`,
            type: "danger"
          })
        }
      }
      reader.readAsDataURL(file)
    } catch (error) {
      console.error("Failed to read file:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: "Failed to read image file",
        type: "danger"
      })
    }

    if (fileInputRef.current) {
      fileInputRef.current.value = ''
    }
  }

  const handleRemoveBackground = async () => {
    try {
      await invoke("remove_sidebar_background")
      setSidebarBgPreview(null)
      onBackgroundChanged?.()
    } catch (error) {
      console.error("Failed to remove sidebar background:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to remove background: ${error}`,
        type: "danger"
      })
    }
  }

  const handleClose = () => {
    setIsClosing(true)
    setTimeout(() => {
      setIsClosing(false)
      onClose()
    }, 150)
  }

  const getMemoryRecommendation = (mb: number) => {
    if (!systemInfo) {
      if (mb < 2048) return { text: "Low - May struggle with modded", color: "text-yellow-500" }
      if (mb < 4096) return { text: "Good for vanilla", color: "text-[#4572e3]" }
      if (mb <= 12288) return { text: "Good for modded", color: "text-[#4572e3]" }
      if (mb <= 16384) return { text: "High allocation", color: "text-yellow-500" }
      return { text: "Excessive - More isn't always better", color: "text-red-500" }
    }
    
    const maxRecommended = systemInfo.recommended_max_memory_mb
    
    if (mb < 2048) return { text: "Low - May struggle with modded", color: "text-yellow-500" }
    if (mb < 4096) return { text: "Good for vanilla", color: "text-[#4572e3]" }
    if (mb < 8192) return { text: "Good for modded", color: "text-[#4572e3]" }
    if (mb > maxRecommended) return { 
      text: `Exceeds recommended (${(maxRecommended / 1024).toFixed(1)} GB max)`, 
      color: "text-red-500" 
    }
    return { text: "High allocation", color: "text-yellow-500" }
  }

  if (!isOpen) return null

  if (!settings) {
    return (
      <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50">
        <div className="bg-[#101010] border border-[#2a2a2a] rounded-lg p-8">
          <div className="flex items-center gap-2 text-[#7d8590] text-base">
            <Loader2 size={20} className="animate-spin" />
            <span>Loading settings...</span>
          </div>
        </div>
      </div>
    )
  }

  const memoryRec = getMemoryRecommendation(settings.memory_mb)

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
        className={`fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 p-4 modal-backdrop ${isClosing ? 'closing' : ''}`}
        onClick={handleClose}
      >
        <div 
          className={`bg-[#101010] rounded-lg w-full max-w-5xl max-h-[90vh] overflow-hidden flex flex-col border border-[#2a2a2a] modal-content ${isClosing ? 'closing' : ''}`}
          onClick={(e) => e.stopPropagation()}
        >
          {/* Header */}
          <div className="flex items-center justify-between p-6 border-b border-[#2a2a2a]">
            <div>
              <h2 className="text-xl font-semibold text-[#e6edf3]">Settings</h2>
              <p className="text-sm text-[#7d8590] mt-0.5">
                Configure launcher and game settings
              </p>
            </div>
            <div className="flex items-center gap-3">
              {appVersion && (
                <span className="bg-[#141414] border border-[#2a2a2a] px-3 py-1.5 rounded text-sm text-[#7d8590]">
                  Build {appVersion.split('-')[1] || appVersion}
                </span>
              )}
              <button
                onClick={handleClose}
                className="p-2 hover:bg-[#171717] rounded transition-colors cursor-pointer"
              >
                <X size={20} className="text-[#7d8590]" />
              </button>
            </div>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto p-6">
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              {/* Left Column */}
              <div className="space-y-6">
                {/* Memory Allocation */}
                <div className="bg-[#141414] border border-[#2a2a2a] rounded-md p-6">
                  <div className="flex items-center gap-3 mb-5">
                    <Cpu size={24} className="text-[#4572e3]" strokeWidth={2} />
                    <div>
                      <h2 className="text-base font-semibold text-[#e6edf3]">Memory Allocation</h2>
                      <p className="text-xs text-[#7d8590]">RAM allocated to Minecraft</p>
                    </div>
                  </div>

                  <div className="space-y-4">
                    <div className="flex items-baseline justify-between">
                      <span className="text-2xl font-bold text-[#e6edf3]">
                        {(settings.memory_mb / 1024).toFixed(1)} GB
                      </span>
                      <span className={`text-xs font-medium ${memoryRec.color}`}>
                        {memoryRec.text}
                      </span>
                    </div>

                    <input
                      type="range"
                      min="1024"
                      max={systemInfo?.total_memory_mb || 32768}
                      step="512"
                      value={settings.memory_mb}
                      onChange={(e) => handleSettingChange({
                        ...settings,
                        memory_mb: parseInt(e.target.value)
                      })}
                      className="w-full h-2 bg-[#2a2a2a] rounded-full appearance-none cursor-pointer"
                      style={{
                        background: `linear-gradient(to right, #4572e3 0%, #4572e3 ${((settings.memory_mb - 1024) / ((systemInfo?.total_memory_mb || 32768) - 1024)) * 100}%, #2a2a2a ${((settings.memory_mb - 1024) / ((systemInfo?.total_memory_mb || 32768) - 1024)) * 100}%, #2a2a2a 100%)`
                      }}
                    />

                    <div className="flex justify-between text-xs text-[#7d8590]">
                      <span>1 GB</span>
                      <span>{systemInfo ? `${(systemInfo.total_memory_mb / 4 / 1024).toFixed(0)} GB` : '8 GB'}</span>
                      <span>{systemInfo ? `${(systemInfo.total_memory_mb / 2 / 1024).toFixed(0)} GB` : '16 GB'}</span>
                      <span>{systemInfo ? `${(systemInfo.total_memory_mb / 1024).toFixed(0)} GB` : '32 GB'}</span>
                    </div>

                    {systemInfo && (
                      <div className="pt-2 border-t border-[#2a2a2a]">
                        <div className="flex justify-between text-xs">
                          <span className="text-[#7d8590]">System Total</span>
                          <span className="text-[#e6edf3] font-medium">
                            {(systemInfo.total_memory_mb / 1024).toFixed(1)} GB
                          </span>
                        </div>
                        <div className="flex justify-between text-xs mt-1">
                          <span className="text-[#7d8590]">Available</span>
                          <span className="text-[#e6edf3] font-medium">
                            {(systemInfo.available_memory_mb / 1024).toFixed(1)} GB
                          </span>
                        </div>
                      </div>
                    )}
                  </div>
                </div>

                {/* Java Configuration */}
                <div className="bg-[#141414] border border-[#2a2a2a] rounded-md p-6">
                  <div className="flex items-center gap-3 mb-5">
                    <Coffee size={24} className="text-[#4572e3]" strokeWidth={2} />
                    <div>
                      <h2 className="text-base font-semibold text-[#e6edf3]">Java Runtime</h2>
                      <p className="text-xs text-[#7d8590]">Java installation for game</p>
                    </div>
                  </div>

                  <div className="space-y-4">
                    <div className="flex gap-2">
                      <div className="relative flex-1">
                        <select
                          className="w-full bg-[#1a1a1a] border border-[#2a2a2a] rounded-lg px-4 py-3 text-sm text-[#e6edf3] focus:outline-none focus:ring-2 focus:ring-[#4572e3] transition-all cursor-pointer appearance-none pr-10"
                          value={showCustomPath ? "custom" : settings.java_path || "auto"}
                          onChange={(e) => {
                            const value = e.target.value
                            if (value === "custom") {
                              setShowCustomPath(true)
                              setCustomPathValue(settings.java_path || "")
                              return
                            }
                            setShowCustomPath(false)
                            setCustomPathValue("")
                            handleSettingChange({
                              ...settings,
                              java_path: value === "auto" ? null : value
                            })
                          }}
                        >
                          <option value="auto">Auto-detect (Recommended)</option>
                          {javaInstallations.map((path) => (
                            <option key={path} value={path}>{path}</option>
                          ))}
                          <option value="custom">Custom Path...</option>
                        </select>
                        <ChevronDown size={16} className="absolute right-3 top-1/2 -translate-y-1/2 text-[#7d8590] pointer-events-none" />
                      </div>
                      
                      <button
                        onClick={loadJavaInstallations}
                        disabled={isLoadingJava}
                        className="px-4 py-3 bg-[#1a1a1a] border border-[#2a2a2a] hover:bg-[#171717] disabled:opacity-50 rounded-lg text-sm font-medium transition-all text-[#e6edf3] cursor-pointer flex-shrink-0"
                      >
                        {isLoadingJava ? (
                          <Loader2 size={16} className="animate-spin" />
                        ) : (
                          "Scan"
                        )}
                      </button>
                    </div>

                    {showCustomPath && (
                      <div className="space-y-2">
                        <input
                          type="text"
                          className="w-full bg-[#1a1a1a] border border-[#2a2a2a] rounded-lg px-4 py-3 text-sm text-[#e6edf3] focus:outline-none focus:ring-2 focus:ring-[#4572e3] transition-all font-mono"
                          placeholder="C:\Program Files\Java\jdk-21\bin\javaw.exe"
                          value={customPathValue}
                          onChange={(e) => setCustomPathValue(e.target.value)}
                          onBlur={() => {
                            if (customPathValue.trim()) {
                              handleSettingChange({
                                ...settings,
                                java_path: customPathValue.trim()
                              })
                            }
                          }}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter' && customPathValue.trim()) {
                              handleSettingChange({
                                ...settings,
                                java_path: customPathValue.trim()
                              })
                              e.currentTarget.blur()
                            }
                          }}
                        />
                      </div>
                    )}

                    <p className="text-xs text-[#7d8590]">
                      Java 17+ required for Minecraft 1.18 and newer versions
                    </p>
                  </div>
                </div>
              </div>

              {/* Right Column */}
              <div className="space-y-6">
                {/* Appearance */}
                <div className="bg-[#141414] border border-[#2a2a2a] rounded-md p-6">
                  <div className="flex items-center gap-3 mb-5">
                    <ImagePlus size={24} className="text-[#4572e3]" strokeWidth={2} />
                    <div>
                      <h2 className="text-base font-semibold text-[#e6edf3]">Sidebar Background</h2>
                      <p className="text-xs text-[#7d8590]">Customize launcher appearance</p>
                    </div>
                  </div>

                  <div className="space-y-4">
                    {sidebarBgPreview ? (
                      <div className="relative group">
                        <div className="h-20 rounded-lg overflow-hidden bg-[#1a1a1a] border border-[#2a2a2a]">
                          <img
                            src={sidebarBgPreview}
                            alt="Background preview"
                            className="w-full h-full object-cover"
                          />
                        </div>
                        <div className="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity rounded-lg flex items-center justify-center gap-3">
                          <button
                            onClick={() => fileInputRef.current?.click()}
                            className="px-4 py-2 bg-[#4572e3] hover:bg-[#3461d9] text-white rounded-lg text-sm font-medium transition-all cursor-pointer"
                          >
                            Change
                          </button>
                          <button
                            onClick={handleRemoveBackground}
                            className="px-4 py-2 bg-red-500 hover:bg-red-600 text-white rounded-lg text-sm font-medium transition-all cursor-pointer"
                          >
                            Remove
                          </button>
                        </div>
                      </div>
                    ) : (
                      <button
                        onClick={() => fileInputRef.current?.click()}
                        className="w-full h-36 bg-[#1a1a1a] hover:bg-[#171717] border-2 border-dashed border-[#2a2a2a] hover:border-[#4572e3] rounded-lg transition-all cursor-pointer flex flex-col items-center justify-center gap-3 group"
                      >
                        <ImagePlus size={32} className="text-[#7d8590] group-hover:text-[#4572e3] transition-colors" strokeWidth={1.5} />
                        <div className="text-center">
                          <p className="text-sm font-medium text-[#e6edf3]">Add Background Image</p>
                          <p className="text-xs text-[#7d8590] mt-1">PNG, JPG up to 10MB</p>
                        </div>
                      </button>
                    )}

                    <input
                      ref={fileInputRef}
                      type="file"
                      accept="image/*"
                      onChange={handleFileSelect}
                      className="hidden"
                    />
                  </div>
                </div>

                {/* Launcher Directory */}
                <div className="bg-[#141414] border border-[#2a2a2a] rounded-md p-6">
                  <div className="flex items-center gap-3 mb-5">
                    <FolderOpen size={24} className="text-[#4572e3]" strokeWidth={2} />
                    <div>
                      <h2 className="text-base font-semibold text-[#e6edf3]">Game Directory</h2>
                      <p className="text-xs text-[#7d8590]">Instance and asset storage</p>
                    </div>
                  </div>

                  <div className="bg-[#1a1a1a] border border-[#2a2a2a] rounded-lg p-4">
                    <p className="text-xs text-[#7d8590] font-mono break-all leading-relaxed">
                      {launcherDirectory || 'Loading...'}
                    </p>
                  </div>
                </div>

                <div className="bg-[#141414] border border-[#2a2a2a] rounded-md p-6">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <svg className="w-6 h-6 text-[#5865F2]" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"/>
                      </svg>
                      <div>
                        <h2 className="text-base font-semibold text-[#e6edf3]">Discord Presence</h2>
                        <p className="text-xs text-[#7d8590]">
                          {settings.discord_rpc_enabled 
                            ? "Shows 'Playing Atomic Launcher' on Discord" 
                            : "Disabled"}
                        </p>
                      </div>
                    </div>
                    <button
                      onClick={() => handleSettingChange({
                        ...settings,
                        discord_rpc_enabled: !settings.discord_rpc_enabled
                      })}
                      className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors cursor-pointer ${
                        settings.discord_rpc_enabled ? 'bg-[#4572e3]' : 'bg-[#2a2a2a]'
                      }`}
                    >
                      <span
                        className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                          settings.discord_rpc_enabled ? 'translate-x-6' : 'translate-x-1'
                        }`}
                      />
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

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