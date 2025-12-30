import { useState, useEffect, useRef } from "react"
import { invoke } from "@tauri-apps/api/core"
import { Loader2, Coffee, Cpu, FolderOpen, ImagePlus, X } from "lucide-react"
import { AlertModal } from "../modals/ConfirmModal"
import type { LauncherSettings } from "../../types"

interface SystemInfo {
  total_memory_mb: number
  available_memory_mb: number
  recommended_max_memory_mb: number
}

interface SettingsTabProps {
  settings: LauncherSettings | null
  launcherDirectory: string
  onSettingsChange: (settings: LauncherSettings) => void
  onBackgroundChanged?: () => void
}

export function SettingsTab({ settings, launcherDirectory, onSettingsChange, onBackgroundChanged }: SettingsTabProps) {
  const [javaInstallations, setJavaInstallations] = useState<string[]>([])
  const [isLoadingJava, setIsLoadingJava] = useState(false)
  const [showCustomPath, setShowCustomPath] = useState(false)
  const [customPathValue, setCustomPathValue] = useState("")
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null)
  const [sidebarBgPreview, setSidebarBgPreview] = useState<string | null>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)
  const [alertModal, setAlertModal] = useState<{
    isOpen: boolean
    title: string
    message: string
    type: "warning" | "danger" | "success" | "info"
  } | null>(null)

  useEffect(() => {
    loadSystemInfo()
    loadSidebarBackground()
  }, [])

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

    // Validate file type
    if (!file.type.startsWith('image/')) {
      setAlertModal({
        isOpen: true,
        title: "Invalid File",
        message: "Please select an image file (PNG, JPG, etc.)",
        type: "warning"
      })
      return
    }

    // Validate file size (max 10MB)
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
      // Convert to base64
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

    // Reset input
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

  const getMemoryRecommendation = (mb: number) => {
    if (!systemInfo) {
      if (mb < 2048) return { text: "Low - May struggle with modded", color: "text-yellow-500" }
      if (mb < 4096) return { text: "Good for vanilla", color: "text-[#16a34a]" }
      if (mb <= 12288) return { text: "Good for modded", color: "text-[#16a34a]" }
      if (mb <= 16384) return { text: "High allocation", color: "text-yellow-500" }
      return { text: "Excessive - More isn't always better", color: "text-red-500" }
    }
    
    const maxRecommended = systemInfo.recommended_max_memory_mb
    
    if (mb < 2048) return { text: "Low - May struggle with modded", color: "text-yellow-500" }
    if (mb < 4096) return { text: "Good for vanilla", color: "text-[#16a34a]" }
    if (mb < 8192) return { text: "Good for modded", color: "text-[#16a34a]" }
    if (mb > maxRecommended) return { 
      text: `Exceeds recommended (${(maxRecommended / 1024).toFixed(1)} GB max)`, 
      color: "text-red-500" 
    }
    return { text: "High allocation", color: "text-yellow-500" }
  }

  if (!settings) {
    return (
      <div className="p-6 flex items-center justify-center h-full">
        <div className="flex items-center gap-2 text-[#808080] text-base">
          <Loader2 size={20} className="animate-spin" />
          <span>Loading settings...</span>
        </div>
      </div>
    )
  }

  const memoryRec = getMemoryRecommendation(settings.memory_mb)

  return (
    <>
      <div className="p-6 space-y-4">
        <div className="max-w-3xl mx-auto">
          <div className="mb-6">
            <h1 className="text-2xl font-semibold text-[#e8e8e8] tracking-tight">Settings</h1>
            <p className="text-sm text-[#808080] mt-0.5">Configure your launcher preferences</p>
          </div>

          <div className="space-y-4">
            {/* Appearance */}
            <div className="bg-[#1a1a1a] rounded-xl p-6">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-3">
                  <ImagePlus size={28} className="text-[#16a34a] flex-shrink-0" strokeWidth={1.5} />
                  <div>
                    <h2 className="text-lg font-semibold text-[#e8e8e8]">Appearance</h2>
                    <p className="text-sm text-[#808080]">Customize the launcher look</p>
                  </div>
                </div>

                {/* Compact image controls */}
                <div className="flex items-center gap-2">
                  {sidebarBgPreview ? (
                    <>
                      <button
                        onClick={() => fileInputRef.current?.click()}
                        className="relative h-16 rounded-lg overflow-hidden bg-[#0d0d0d] border border-[#2a2a2a] hover:border-[#16a34a] transition-colors cursor-pointer"
                      >
                        <img
                          src={sidebarBgPreview}
                          alt="Preview"
                          className="h-full w-auto object-contain"
                        />
                      </button>
                      <button
                        onClick={handleRemoveBackground}
                        className="p-2 bg-red-500/10 hover:bg-red-500/20 rounded-lg transition-colors cursor-pointer"
                      >
                        <X size={16} className="text-red-400" />
                      </button>
                    </>
                  ) : (
                    <button
                      onClick={() => fileInputRef.current?.click()}
                      className="flex items-center gap-2 px-3 py-2 bg-[#0d0d0d] hover:bg-[#0a0a0a] border border-dashed border-[#2a2a2a] hover:border-[#16a34a] rounded-lg transition-colors cursor-pointer group"
                    >
                      <ImagePlus size={16} className="text-[#4a4a4a] group-hover:text-[#16a34a] transition-colors" />
                      <span className="text-xs font-medium text-[#e8e8e8]">Add Image</span>
                    </button>
                  )}
                </div>
              </div>

              <input
                ref={fileInputRef}
                type="file"
                accept="image/*"
                onChange={handleFileSelect}
                className="hidden"
              />
            </div>

            {/* Java Configuration */}
            <div className="bg-[#1a1a1a] rounded-xl p-6">
              <div className="flex items-center gap-3 mb-4">
                <Coffee size={28} className="text-[#16a34a] flex-shrink-0" strokeWidth={1.5} />
                <div>
                  <h2 className="text-lg font-semibold text-[#e8e8e8]">Java Configuration</h2>
                  <p className="text-sm text-[#808080]">Select Java installation for Minecraft</p>
                </div>
              </div>

              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-[#e8e8e8] mb-2">Java Installation</label>
                  <div className="flex gap-2">
                    <select
                      className="flex-1 bg-[#0d0d0d] rounded-lg px-3 py-2.5 pr-8 text-sm text-[#e8e8e8] focus:outline-none focus:ring-2 focus:ring-[#16a34a] transition-all cursor-pointer appearance-none bg-[length:16px] bg-[right_0.75rem_center] bg-no-repeat"
                      style={{
                        backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%23808080' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E")`
                      }}
                      value={
                        showCustomPath
                          ? "custom"
                          : settings.java_path || "auto"
                      }
                      onChange={(e) => {
                        const value = e.target.value
                        if (value === "custom") {
                          setShowCustomPath(true)
                          setCustomPathValue(settings.java_path || "")
                          return
                        }
                        setShowCustomPath(false)
                        handleSettingChange({
                          ...settings,
                          java_path: value === "auto" ? null : value
                        })
                      }}
                    >
                      <option value="auto">Auto-detect Java</option>
                      {javaInstallations.map((path) => (
                        <option key={path} value={path}>{path}</option>
                      ))}
                      <option value="custom">Custom Path...</option>
                    </select>
                    <button
                      onClick={loadJavaInstallations}
                      disabled={isLoadingJava}
                      className="px-4 py-2.5 bg-[#0d0d0d] hover:bg-[#0a0a0a] disabled:opacity-50 rounded-lg text-sm font-medium transition-all text-[#e8e8e8] whitespace-nowrap cursor-pointer"
                    >
                      {isLoadingJava ? (
                        <div className="flex items-center gap-2">
                          <Loader2 size={14} className="animate-spin" />
                          <span>Scanning...</span>
                        </div>
                      ) : (
                        "Scan"
                      )}
                    </button>
                  </div>
                  <p className="text-xs text-[#4a4a4a] mt-2">Java 17+ required for Minecraft 1.18 and newer</p>
                </div>

                {showCustomPath && (
                  <div>
                    <label className="block text-sm font-medium text-[#e8e8e8] mb-2">Custom Java Path</label>
                    <input
                      type="text"
                      className="w-full bg-[#0d0d0d] rounded-lg px-3 py-2.5 text-sm text-[#e8e8e8] focus:outline-none focus:ring-2 focus:ring-[#16a34a] transition-all font-mono"
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
                    />
                    <p className="text-xs text-[#4a4a4a] mt-2">Enter the full path to your Java executable</p>
                  </div>
                )}
              </div>
            </div>

            {/* Memory Allocation */}
            <div className="bg-[#1a1a1a] rounded-xl p-6">
              <div className="flex items-center gap-3 mb-4">
                <Cpu size={28} className="text-[#16a34a] flex-shrink-0" strokeWidth={1.5} />
                <div>
                  <h2 className="text-lg font-semibold text-[#e8e8e8]">Memory Allocation</h2>
                  <p className="text-sm text-[#808080]">Configure RAM for Minecraft</p>
                </div>
              </div>

              <div>
                <div className="flex items-center justify-between mb-3">
                  <label className="text-sm font-medium text-[#e8e8e8]">
                    {(settings.memory_mb / 1024).toFixed(1)} GB
                  </label>
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
                  className="w-full h-2 bg-[#2a2a2a] rounded-full appearance-none cursor-pointer accent-[#16a34a]"
                  style={{
                    background: `linear-gradient(to right, #16a34a 0%, #16a34a ${((settings.memory_mb - 1024) / ((systemInfo?.total_memory_mb || 32768) - 1024)) * 100}%, #2a2a2a ${((settings.memory_mb - 1024) / ((systemInfo?.total_memory_mb || 32768) - 1024)) * 100}%, #2a2a2a 100%)`
                  }}
                />
                <div className="flex justify-between text-xs text-[#4a4a4a] mt-2">
                  <span>1 GB</span>
                  <span>{systemInfo ? `${(systemInfo.total_memory_mb / 4 / 1024).toFixed(0)} GB` : '8 GB'}</span>
                  <span>{systemInfo ? `${(systemInfo.total_memory_mb / 2 / 1024).toFixed(0)} GB` : '16 GB'}</span>
                  <span>{systemInfo ? `${(systemInfo.total_memory_mb / 1024).toFixed(0)} GB` : '32 GB'}</span>
                </div>
              </div>
            </div>

            {/* Launcher Information */}
            <div className="bg-[#1a1a1a] rounded-xl p-6">
              <div className="flex items-center gap-3 mb-4">
                <FolderOpen size={28} className="text-[#16a34a] flex-shrink-0" strokeWidth={1.5} />
                <div>
                  <h2 className="text-lg font-semibold text-[#e8e8e8]">Launcher Directory</h2>
                  <p className="text-sm text-[#808080]">Where launcher files are stored</p>
                </div>
              </div>

              <div className="bg-[#0d0d0d] rounded-lg p-3">
                <p className="text-sm text-[#e8e8e8] font-mono break-all">
                  {launcherDirectory || 'Loading...'}
                </p>
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