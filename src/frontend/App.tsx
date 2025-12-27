import { useState, useEffect } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { LogOut, Settings, LogIn, Home, Package, Search, Terminal, Minus, Square, X, Server, Play, ChevronUp, ChevronDown } from "lucide-react"
import { HomeTab } from "./tabs/HomeTab"
import { InstancesTab } from "./tabs/InstancesTab"
import { ModsTab } from "./tabs/ModsTab"
import { ConsoleTab } from "./tabs/ConsoleTab"
import { SettingsTab } from "./tabs/SettingsTab"
import { ServersTab } from "./tabs/ServersTab"
import { CreateInstanceModal } from "./modals/CreateInstanceModal"
import { CreationProgressToast } from "./modals/CreationProgressToast"
import { InstanceDetailsTab } from "./modals/InstanceDetailsTab"
import { ConfirmModal, AlertModal } from "./modals/ConfirmModal"
import type { Instance, LauncherSettings, ConsoleLog } from "../types"

interface AccountInfo {
  uuid: string
  username: string
  is_active: boolean
  added_at: string
  last_used: string | null
}

function App() {
  const [isReady, setIsReady] = useState(false)
  const [isAuthenticated, setIsAuthenticated] = useState(false)
  const [activeAccount, setActiveAccount] = useState<AccountInfo | null>(null)
  const [accounts, setAccounts] = useState<AccountInfo[]>([])
  const [showAccountDropdown, setShowAccountDropdown] = useState(false)
  const [isLaunching, setIsLaunching] = useState(false)
  const [isLoggingIn, setIsLoggingIn] = useState(false)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [versions, setVersions] = useState<string[]>([])
  const [instances, setInstances] = useState<Instance[]>([])
  const [selectedInstance, setSelectedInstance] = useState<Instance | null>(null)
  const [launcherDirectory, setLauncherDirectory] = useState("")
  const [settings, setSettings] = useState<LauncherSettings | null>(null)
  const [activeTab, setActiveTab] = useState<"home" | "instances" | "browse" | "console" | "settings" | "servers">("home")
  const [consoleLogs, setConsoleLogs] = useState<ConsoleLog[]>([])
  const [showInstanceDetails, setShowInstanceDetails] = useState(false)
  const [creatingInstanceName, setCreatingInstanceName] = useState<string | null>(null)
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
  const [instanceIcons, setInstanceIcons] = useState<Record<string, string | null>>({})

  const appWindow = getCurrentWindow()

  // Get recently played instances (sorted by lastPlayed, limit to 3)
  const recentInstances = [...instances]
    .filter(inst => inst.last_played)
    .sort((a, b) => {
      const timeA = a.last_played ? new Date(a.last_played).getTime() : 0
      const timeB = b.last_played ? new Date(b.last_played).getTime() : 0
      return timeB - timeA
    })
    .slice(0, 3)

  const getMinecraftVersion = (instance: Instance): string => {
    if (instance.loader === "fabric") {
      const parts = instance.version.split('-')
      return parts[parts.length - 1]
    }
    return instance.version
  }

  const formatLastPlayed = (timestamp: string): string => {
    const seconds = Math.floor((Date.now() - new Date(timestamp).getTime()) / 1000)
    const minutes = Math.floor(seconds / 60)
    const hours = Math.floor(minutes / 60)
    const days = Math.floor(hours / 24)

    if (days > 0) return `${days}d ago`
    if (hours > 0) return `${hours}h ago`
    if (minutes > 0) return `${minutes}m ago`
    return 'Just now'
  }

  const formatLastUsed = (timestamp: string | null): string => {
    if (!timestamp) return 'Never'
    
    const seconds = Math.floor((Date.now() - new Date(timestamp).getTime()) / 1000)
    const minutes = Math.floor(seconds / 60)
    const hours = Math.floor(minutes / 60)
    const days = Math.floor(hours / 24)

    if (days > 0) return `${days}d ago`
    if (hours > 0) return `${hours}h ago`
    if (minutes > 0) return `${minutes}m ago`
    return 'Just now'
  }

  // Initialize app and handle splashscreen
  useEffect(() => {
    const initializeApp = async () => {
      setIsReady(true)
      await new Promise(resolve => setTimeout(resolve, 100))
      await invoke('frontend_ready')
    }
    initializeApp()
  }, [])

  useEffect(() => {
    if (!isReady) return
    loadVersions()
    loadInstances()
    loadLauncherDirectory()
    loadSettings()
    loadAccounts()
    const unlisten = listen<ConsoleLog>("console-log", (event) => {
      setConsoleLogs((prev) => [...prev, event.payload])
    })
    return () => {
      unlisten.then((fn) => fn())
    }
  }, [isReady])

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

  const loadVersions = async () => {
    try {
      const versionList = await invoke<string[]>("get_minecraft_versions")
      setVersions(versionList)
    } catch (error) {
      console.error("Failed to load versions:", error)
    }
  }

  const loadInstances = async () => {
    try {
      const instanceList = await invoke<Instance[]>("get_instances")
      setInstances(instanceList)
      if (selectedInstance) {
        const updated = instanceList.find(i => i.name === selectedInstance.name)
        if (updated) {
          setSelectedInstance(updated)
        } else {
          setSelectedInstance(instanceList[0] || null)
        }
      } else if (instanceList.length > 0 && !selectedInstance) {
        setSelectedInstance(instanceList[0])
      }
    } catch (error) {
      console.error("Failed to load instances:", error)
    }
  }

  const loadLauncherDirectory = async () => {
    try {
      const dir = await invoke<string>("get_launcher_directory")
      setLauncherDirectory(dir)
    } catch (error) {
      console.error("Failed to get launcher directory:", error)
    }
  }

  const loadSettings = async () => {
    try {
      const s = await invoke<LauncherSettings>("get_settings")
      setSettings(s)
    } catch (e) {
      console.error("Failed to load settings", e)
    }
  }

  const loadAccounts = async () => {
    try {
      const accountList = await invoke<AccountInfo[]>("get_accounts")
      setAccounts(accountList)
      
      const active = accountList.find(acc => acc.is_active)
      setActiveAccount(active || null)
      setIsAuthenticated(!!active)
    } catch (error) {
      console.error("Failed to load accounts:", error)
    }
  }

  const handleAddAccount = async () => {
    setIsLoggingIn(true)
    setShowAccountDropdown(false)
    try {
      await invoke<AccountInfo>("microsoft_login_and_store")
      await loadAccounts()
    } catch (error) {
      console.error("Login error:", error)
    } finally {
      setIsLoggingIn(false)
    }
  }

  const handleSwitchAccount = async (uuid: string) => {
    try {
      await invoke("switch_account", { uuid })
      await loadAccounts()
      setShowAccountDropdown(false)
    } catch (error) {
      console.error("Failed to switch account:", error)
    }
  }

  const handleRemoveAccount = async (uuid: string) => {
    try {
      await invoke("remove_account", { uuid })
      await loadAccounts()
      setShowAccountDropdown(false)
    } catch (error) {
      console.error("Failed to remove account:", error)
    }
  }

  const handleLaunch = async () => {
    if (!activeAccount || !selectedInstance) return
    setIsLaunching(true)
    setConsoleLogs([])
    setActiveTab("console")
    setShowInstanceDetails(false)
    try {
      await invoke<string>("launch_instance_with_active_account", {
        instanceName: selectedInstance.name,
        appHandle: appWindow,
      })
      await loadInstances()
      setTimeout(() => {
        setIsLaunching(false)
      }, 2000)
    } catch (error) {
      console.error("Launch error:", error)
      setIsLaunching(false)
    }
  }

  const handleDeleteInstance = async (instanceName: string) => {
    setConfirmModal({
      isOpen: true,
      title: "Delete Instance",
      message: `Are you sure you want to delete "${instanceName}"?\n\nThis action cannot be undone.`,
      type: "danger",
      onConfirm: async () => {
        try {
          await invoke<string>("delete_instance", { instanceName })
          await loadInstances()
          if (selectedInstance?.name === instanceName) {
            setSelectedInstance(instances.length > 1 ? instances.find(i => i.name !== instanceName) || null : null)
          }
          setConfirmModal(null)
        } catch (error) {
          console.error("Delete error:", error)
          setConfirmModal(null)
        }
      }
    })
  }

  const handleDuplicateInstance = async (instance: Instance) => {
    let baseName = instance.name
    let counter = 1
    let newName = `${baseName} (Copy)`
    
    while (instances.some(i => i.name === newName)) {
      counter++
      newName = `${baseName} (Copy ${counter})`
    }
    
    setCreatingInstanceName(newName)
    
    try {
      await invoke("duplicate_instance", {
        instanceName: instance.name,
        newName: newName,
        appHandle: appWindow,
      })
      
      await loadInstances()
    } catch (error) {
      console.error("Duplicate error:", error)
      setCreatingInstanceName(null)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to duplicate instance: ${error}`,
        type: "danger"
      })
    }
  }

  const handleOpenInstanceFolder = async () => {
    if (!selectedInstance) return
    try {
      await invoke("open_instance_folder", { instanceName: selectedInstance.name })
    } catch (error) {
      console.error("Failed to open folder:", error)
    }
  }

  const handleOpenInstanceFolderByInstance = async (instance: Instance) => {
    try {
      await invoke("open_instance_folder", { instanceName: instance.name })
    } catch (error) {
      console.error("Failed to open folder:", error)
    }
  }

  const handleShowDetails = (instance: Instance) => {
    setSelectedInstance(instance)
    setShowInstanceDetails(true)
  }

  const handleCloseDetails = () => {
    setShowInstanceDetails(false)
  }

  const handleStartCreating = (instanceName: string) => {
    setCreatingInstanceName(instanceName)
  }

  const handleCreationComplete = () => {
    setCreatingInstanceName(null)
    loadInstances()
  }

  const handleCreationError = () => {
    setCreatingInstanceName(null)
  }

  const handleQuickLaunch = async (instance: Instance) => {
    if (!activeAccount) return
    
    setSelectedInstance(instance)
    setIsLaunching(true)
    setConsoleLogs([])
    setActiveTab("console")
    
    try {
      await invoke<string>("launch_instance_with_active_account", {
        instanceName: instance.name,
        appHandle: appWindow,
      })
      await loadInstances()
      setTimeout(() => {
        setIsLaunching(false)
      }, 2000)
    } catch (error) {
      console.error("Launch error:", error)
      setIsLaunching(false)
    }
  }

  return (
    <div className={`flex flex-col h-screen bg-[#0d0d0d] text-[#e8e8e8] overflow-hidden font-sans ${!isReady ? 'opacity-0' : 'opacity-100 transition-opacity duration-300'}`}>
      {/* Custom Title Bar */}
      <div 
        data-tauri-drag-region
        style={{ userSelect: 'none', WebkitAppRegion: 'drag' } as any}
        className="h-10 bg-[#1a1a1a] flex-shrink-0 fixed top-0 left-0 right-0 z-50 flex items-center px-4 border-b border-[#2a2a2a]"
      >
        <div className="flex items-center gap-2 mr-23">
          <img src="/logo.png" alt="Atomic Launcher" className="h-5 w-5" />
          <span className="text-sm font-semibold text-[#e8e8e8]">Atomic Launcher</span>
        </div>
        <div className="flex items-center gap-2 text-sm flex-1">
          <button
            onClick={() => {
              setActiveTab("home")
              setShowInstanceDetails(false)
            }}
            className={`transition-colors cursor-pointer ${activeTab === "home" && !showInstanceDetails ? "text-[#e8e8e8] font-semibold" : "text-[#808080] hover:text-[#e8e8e8]"}`}
            style={{ WebkitAppRegion: 'no-drag' } as any}
          >
            Home
          </button>
          {activeTab !== "home" && (
            <>
              <span className="text-[#4a4a4a]">›</span>
              <button
                onClick={() => setShowInstanceDetails(false)}
                className={`transition-colors capitalize cursor-pointer ${!showInstanceDetails ? "text-[#e8e8e8] font-semibold" : "text-[#808080] hover:text-[#e8e8e8]"}`}
                style={{ WebkitAppRegion: 'no-drag' } as any}
              >
                {activeTab === "browse" ? "Mods" : activeTab}
              </button>
            </>
          )}
          {showInstanceDetails && selectedInstance && (
            <>
              <span className="text-[#4a4a4a]">›</span>
              <span className="text-[#e8e8e8] font-semibold">{selectedInstance.name}</span>
            </>
          )}
        </div>

        {/* Window Controls */}
        <div className="flex items-center ml-auto" style={{ WebkitAppRegion: 'no-drag' } as any}>
          <button
            onClick={() => appWindow.minimize()}
            className="h-10 w-12 flex items-center justify-center text-[#808080] hover:text-[#e8e8e8] hover:bg-[#2a2a2a] transition-colors"
          >
            <Minus size={16} />
          </button>
          <button
            onClick={() => appWindow.toggleMaximize()}
            className="h-10 w-12 flex items-center justify-center text-[#808080] hover:text-[#e8e8e8] hover:bg-[#2a2a2a] transition-colors"
          >
            <Square size={14} />
          </button>
          <button
            onClick={() => appWindow.close()}
            className="h-10 w-12 flex items-center justify-center text-[#808080] hover:text-[#e8e8e8] hover:bg-red-500 transition-colors"
          >
            <X size={16} />
          </button>
        </div>
      </div>

      {/* Creation Progress Toast */}
      {creatingInstanceName && (
        <CreationProgressToast
          instanceName={creatingInstanceName}
          onComplete={handleCreationComplete}
          onError={handleCreationError}
          onDismiss={() => setCreatingInstanceName(null)}
        />
      )}

      {/* Confirmation Modal */}
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

      {/* Main Content Container */}
      <div className="flex flex-1 overflow-hidden mt-10">
        {/* Sidebar */}
        <aside className="sidebar-bg w-58 bg-[#1a1a1a] border-r border-[#2a2a2a] flex flex-col">
          <div className="flex-1 flex flex-col overflow-hidden">
            {/* Navigation */}
            <nav className="flex-shrink-0 px-2 py-3 space-y-1">
              <button
                onClick={() => {
                  setActiveTab("home")
                  setShowInstanceDetails(false)
                }}
                className={`w-full flex items-center gap-3 px-3.5 py-2.5 rounded-md text-[15px] font-medium transition-all cursor-pointer ${
                  activeTab === "home"
                    ? "bg-[#2a2a2a] text-[#e8e8e8] shadow-sm"
                    : "text-[#808080] hover:text-[#e8e8e8] hover:bg-[#1f1f1f]"
                }`}
              >
                <Home size={19} strokeWidth={2} />
                <span>Home</span>
              </button>
              <button
                onClick={() => {
                  setActiveTab("instances")
                  setShowInstanceDetails(false)
                }}
                className={`w-full flex items-center gap-3 px-3.5 py-2.5 rounded-md text-[15px] font-medium transition-all cursor-pointer ${
                  activeTab === "instances"
                    ? "bg-[#2a2a2a] text-[#e8e8e8] shadow-sm"
                    : "text-[#808080] hover:text-[#e8e8e8] hover:bg-[#1f1f1f]"
                }`}
              >
                <Package size={19} strokeWidth={2} />
                <span>Instances</span>
              </button>
              <button
                onClick={() => {
                  setActiveTab("browse")
                  setShowInstanceDetails(false)
                }}
                className={`w-full flex items-center gap-3 px-3.5 py-2.5 rounded-md text-[15px] font-medium transition-all cursor-pointer ${
                  activeTab === "browse"
                    ? "bg-[#2a2a2a] text-[#e8e8e8] shadow-sm"
                    : "text-[#808080] hover:text-[#e8e8e8] hover:bg-[#1f1f1f]"
                }`}
              >
                <Search size={19} strokeWidth={2} />
                <span>Mods</span>
              </button>
              <button
                onClick={() => {
                  setActiveTab("servers")
                  setShowInstanceDetails(false)
                }}
                className={`w-full flex items-center gap-3 px-3.5 py-2.5 rounded-md text-[15px] font-medium transition-all cursor-pointer ${
                  activeTab === "servers"
                    ? "bg-[#2a2a2a] text-[#e8e8e8] shadow-sm"
                    : "text-[#808080] hover:text-[#e8e8e8] hover:bg-[#1f1f1f]"
                }`}
              >
                <Server size={19} strokeWidth={2} />
                <span>Servers</span>
              </button>
              <button
                onClick={() => {
                  setActiveTab("console")
                  setShowInstanceDetails(false)
                }}
                className={`w-full flex items-center gap-3 px-3.5 py-2.5 rounded-md text-[15px] font-medium transition-all cursor-pointer ${
                  activeTab === "console"
                    ? "bg-[#2a2a2a] text-[#e8e8e8] shadow-sm"
                    : "text-[#808080] hover:text-[#e8e8e8] hover:bg-[#1f1f1f]"
                }`}
              >
                <Terminal size={19} strokeWidth={2} />
                <span>Console</span>
              </button>
            </nav>

            {/* Recent Instances Section */}
            {recentInstances.length > 0 && (
              <div className="flex-1 overflow-y-auto px-2 pb-3">
                <div className="py-2">
                  <h3 className="text-xs font-semibold text-[#808080] uppercase tracking-wider mb-2 px-2">
                    Recently Played
                  </h3>
                  <div className="space-y-1">
                    {recentInstances.map((instance) => {
                      const icon = instanceIcons[instance.name]
                      return (
                        <button
                          key={instance.name}
                          onClick={() => {
                            setSelectedInstance(instance)
                            setActiveTab("instances")
                            setShowInstanceDetails(true)
                          }}
                          className="group w-full flex items-center gap-2 rounded-md cursor-pointer transition-all text-[#808080] hover:text-[#e8e8e8] hover:bg-[#1f1f1f] px-1.5 py-1.5"
                        >
                          {icon ? (
                            <img
                              src={icon}
                              alt={instance.name}
                              className="w-9 h-9 rounded-md object-cover flex-shrink-0"
                            />
                          ) : (
                            <div className="w-9 h-9 flex items-center justify-center flex-shrink-0 bg-[#0d0d0d] rounded-md">
                              <Package size={24} className="text-[#4a4a4a]" strokeWidth={1.5} />
                            </div>
                          )}
                          <div className="flex-1 min-w-0 text-left">
                            <div className="text-sm font-medium text-[#e8e8e8] truncate leading-tight">
                              {instance.name}
                            </div>
                            <div className="flex items-center gap-1 text-xs text-[#808080] leading-tight mt-0.5">
                              <span className="truncate">{getMinecraftVersion(instance)}</span>
                              <span>•</span>
                              <span className="truncate">{formatLastPlayed(instance.last_played!)}</span>
                            </div>
                          </div>
                          {isAuthenticated && (
                            <button
                              onClick={(e) => {
                                e.stopPropagation()
                                handleQuickLaunch(instance)
                              }}
                              disabled={isLaunching}
                              className={`opacity-0 group-hover:opacity-100 flex-shrink-0 w-8 h-8 flex items-center justify-center rounded-md transition-all cursor-pointer mr-1 ${
                                isLaunching && selectedInstance?.name === instance.name
                                  ? "bg-red-500/10 text-red-400"
                                  : "bg-[#16a34a]/10 hover:bg-[#16a34a]/20 text-[#16a34a]"
                              } disabled:opacity-50`}
                            >
                              {isLaunching && selectedInstance?.name === instance.name ? (
                                <div className="w-3.5 h-3.5 border-2 border-red-400/30 border-t-red-400 rounded-full animate-spin" />
                              ) : (
                                <Play size={16} fill="currentColor" strokeWidth={0} />
                              )}
                            </button>
                          )}
                        </button>
                      )
                    })}
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Bottom Section */}
          <div className="p-2 space-y-1 border-t border-[#2a2a2a]">
            {isAuthenticated && activeAccount ? (
              <>
                {/* Account Dropdown Container */}
                <div className="relative py-1 mb-0.5">
                  <button
                    onClick={() => setShowAccountDropdown(!showAccountDropdown)}
                    className="w-full flex items-center gap-2.5 p-2 cursor-pointer hover:bg-[#1f1f1f] rounded-md transition-colors"
                  >
                    <div className="relative">
                      <img
                        src={`https://cravatar.eu/avatar/${activeAccount.username}/32`}
                        alt={activeAccount.username}
                        className="w-8 h-8 rounded-md"
                      />
                    </div>
                    <div className="flex-1 min-w-0 text-left">
                      <div className="text-xs text-[#808080]">Welcome back,</div>
                      <div className="text-sm font-medium text-[#e8e8e8] truncate">{activeAccount.username}</div>
                    </div>
                    <div className="flex flex-col text-[#808080]">
                      <ChevronUp size={14} strokeWidth={2.5} />
                      <ChevronDown size={14} strokeWidth={2.5} />
                    </div>
                  </button>

                  {/* Account Dropdown Menu */}
                  {showAccountDropdown && (
                    <>
                      <div 
                        className="fixed inset-0 z-40"
                        onClick={() => setShowAccountDropdown(false)}
                      />
                      
                      {/* Dropdown */}
                      <div className="absolute bottom-1 left-0 right-0 bg-[#1a1a1a] border border-[#2a2a2a] rounded-lg shadow-xl z-50 overflow-hidden">
                        {/* Add Account Button */}
                        <button
                          onClick={handleAddAccount}
                          disabled={isLoggingIn}
                          className="w-full flex items-center gap-2 p-2 text-sm text-[#e8e8e8] hover:bg-[#1f1f1f] rounded-md transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                          <LogIn size={18} className="text-[#16a34a]" />
                          <span>{isLoggingIn ? 'Authenticating...' : 'Add another account'}</span>
                        </button>

                        {/* Other Accounts List */}
                        {accounts.filter(acc => !acc.is_active).length > 0 && (
                          <>
                            <div className="border-t border-[#2a2a2a]" />
                            <div className="max-h-60 overflow-y-auto">
                              {accounts
                                .filter(acc => !acc.is_active)
                                .map((account) => (
                                  <div
                                    key={account.uuid}
                                    className="flex items-center gap-2 p-2.5 hover:bg-[#2a2a2a] transition-colors group"
                                  >
                                    <button
                                      onClick={() => handleSwitchAccount(account.uuid)}
                                      className="flex-1 flex items-center gap-2.5 cursor-pointer"
                                    >
                                      <img
                                        src={`https://cravatar.eu/avatar/${account.username}/32`}
                                        alt={account.username}
                                        className="w-8 h-8 rounded-md"
                                      />
                                      <div className="flex-1 min-w-0 text-left">
                                        <div className="text-sm font-medium text-[#e8e8e8] truncate">
                                          {account.username}
                                        </div>
                                        <div className="text-xs text-[#808080]">
                                          Last used: {formatLastUsed(account.last_used)}
                                        </div>
                                      </div>
                                    </button>
                                    
                                    {/* Remove Account Button */}
                                    <button
                                      onClick={() => handleRemoveAccount(account.uuid)}
                                      className="opacity-0 group-hover:opacity-100 p-1.5 hover:bg-red-500/10 rounded transition-all cursor-pointer"
                                    >
                                      <LogOut size={16} className="text-red-400" />
                                    </button>
                                  </div>
                                ))}
                            </div>
                          </>
                        )}

                        {/* Divider */}
                        <div className="border-t border-[#2a2a2a]" />

                        {/* Current Active Account */}
                        <div className="flex items-center gap-2.5 p-2 bg-[#2a2a2a] group">
                          <div className="flex-1 flex items-center gap-2.5">
                            <img
                              src={`https://cravatar.eu/avatar/${activeAccount.username}/32`}
                              alt={activeAccount.username}
                              className="w-8 h-8 rounded-md"
                            />
                            <div className="flex-1 min-w-0 text-left">
                              <div className="text-xs text-[#808080]">Welcome back,</div>
                              <div className="text-sm font-medium text-[#e8e8e8] truncate">{activeAccount.username}</div>
                            </div>
                          </div>
                          
                          {/* Logout Current Account Button */}
                          <button
                            onClick={() => handleRemoveAccount(activeAccount.uuid)}
                            className="opacity-0 group-hover:opacity-100 p-1.5 hover:bg-red-500/10 rounded transition-all cursor-pointer"
                          >
                            <LogOut size={16} className="text-red-400" />
                          </button>
                        </div>
                      </div>
                    </>
                  )}
                </div>
              </>
            ) : (
              <button
                onClick={handleAddAccount}
                disabled={isLoggingIn}
                className="w-full flex items-center gap-3 px-4 py-2.5 rounded-md text-base font-medium transition-all cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed mb-2 text-[#808080] hover:text-[#e8e8e8] hover:bg-[#1f1f1f]"
              >
                <LogIn size={20} className="text-[#16a34a]" strokeWidth={2} />
                <span>{isLoggingIn ? 'Authenticating...' : 'Sign In'}</span>
              </button>
            )}
            
            <button
              onClick={() => {
                setActiveTab("settings")
                setShowInstanceDetails(false)
              }}
              className={`w-full flex items-center gap-3 px-4 py-2.5 rounded-md text-base font-medium transition-all cursor-pointer mb-1 ${
                activeTab === "settings"
                  ? "bg-[#2a2a2a] text-[#e8e8e8] shadow-sm"
                  : "text-[#808080] hover:text-[#e8e8e8] hover:bg-[#1f1f1f]"
              }`}
            >
              <Settings size={18} strokeWidth={2} />
              <span>Settings</span>
            </button>
          </div>
        </aside>

        {/* Main Content */}
        <main className="flex-1 overflow-y-auto bg-[#0d0d0d]">
          {showInstanceDetails && selectedInstance ? (
            <InstanceDetailsTab
              instance={selectedInstance}
              isAuthenticated={isAuthenticated}
              isLaunching={isLaunching}
              onLaunch={handleLaunch}
              onBack={handleCloseDetails}
              onInstanceUpdated={loadInstances}
            />
          ) : (
            <>
              {activeTab === "home" && (
                <HomeTab
                  selectedInstance={selectedInstance}
                  instances={instances}
                  isAuthenticated={isAuthenticated}
                  isLaunching={isLaunching}
                  onSetSelectedInstance={setSelectedInstance}
                  onLaunch={handleLaunch}
                  onOpenFolder={handleOpenInstanceFolder}
                  onDeleteInstance={handleDeleteInstance}
                  onCreateNew={() => setShowCreateModal(true)}
                  onShowDetails={handleShowDetails}
                  onOpenFolderByInstance={handleOpenInstanceFolderByInstance}
                  onDuplicateInstance={handleDuplicateInstance}
                  onRefreshInstances={loadInstances}
                />
              )}

              {activeTab === "instances" && (
                <InstancesTab
                  instances={instances}
                  selectedInstance={selectedInstance}
                  isAuthenticated={isAuthenticated}
                  onSetSelectedInstance={setSelectedInstance}
                  onLaunch={handleLaunch}
                  onCreateNew={() => setShowCreateModal(true)}
                  onShowDetails={handleShowDetails}
                  onOpenFolder={handleOpenInstanceFolderByInstance}
                  onDuplicateInstance={handleDuplicateInstance}
                  onDeleteInstance={handleDeleteInstance}
                />
              )}

              {activeTab === "browse" && (
                <ModsTab
                  selectedInstance={selectedInstance}
                  instances={instances}
                  onSetSelectedInstance={setSelectedInstance}
                />
              )}

              {activeTab === "servers" && (
                <ServersTab />
              )}

              {activeTab === "console" && (
                <ConsoleTab
                  consoleLogs={consoleLogs}
                  onClearConsole={() => setConsoleLogs([])}
                />
              )}

              {activeTab === "settings" && (
                <SettingsTab
                  settings={settings}
                  launcherDirectory={launcherDirectory}
                  onSettingsChange={setSettings}
                />
              )}
            </>
          )}
        </main>
      </div>

      {showCreateModal && (
        <CreateInstanceModal
          versions={versions}
          instances={instances}
          onClose={() => setShowCreateModal(false)}
          onSuccess={handleCreationComplete}
          onStartCreating={handleStartCreating}
        />
      )}
    </div>
  )
}

export default App