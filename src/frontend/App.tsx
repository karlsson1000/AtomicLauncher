import { useState, useEffect } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { Minus, Square, X, ChevronLeft, ChevronRight, Home, Package, Puzzle, Server, HatGlasses, Telescope, Terminal, Settings } from "lucide-react"
import { HomeTab } from "./tabs/HomeTab"
import { InstancesTab } from "./tabs/InstancesTab"
import { BrowseTab } from "./tabs/BrowseTab"
import { ConsoleTab } from "./tabs/ConsoleTab"
import { SettingsModal } from "./modals/SettingsModal"
import { ServersTab } from "./tabs/ServersTab"
import { SkinsTab } from "./tabs/SkinsTab"
import { CreateInstanceModal } from "./modals/CreateInstanceModal"
import { CreationProgressToast } from "./modals/CreationProgressToast"
import { UpdateNotificationToast } from "./modals/UpdateNotificationToast"
import { InstanceDetailsTab } from "./modals/InstanceDetailsTab"
import { ConfirmModal, AlertModal } from "./modals/ConfirmModal"
import { MapTab } from "./tabs/MapTab"
import { Sidebar } from "./components/Sidebar"
import type { Instance, LauncherSettings, ConsoleLog } from "../types"

interface AccountInfo {
  uuid: string
  username: string
  is_active: boolean
  added_at: string
  last_used: string | null
}

interface UpdateInfo {
  current_version: string
  new_version: string
}

function App() {
  const [isReady, setIsReady] = useState(false)
  const [isAuthenticated, setIsAuthenticated] = useState(false)
  const [activeAccount, setActiveAccount] = useState<AccountInfo | null>(null)
  const [accounts, setAccounts] = useState<AccountInfo[]>([])
  const [launchingInstanceName, setLaunchingInstanceName] = useState<string | null>(null)
  const [runningInstances, setRunningInstances] = useState<Set<string>>(new Set())
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [versions, setVersions] = useState<string[]>([])
  const [instances, setInstances] = useState<Instance[]>([])
  const [selectedInstance, setSelectedInstance] = useState<Instance | null>(null)
  const [launcherDirectory, setLauncherDirectory] = useState("")
  const [settings, setSettings] = useState<LauncherSettings | null>(null)
  const [activeTab, setActiveTab] = useState<"home" | "instances" | "browse" | "console" | "servers" | "skins" | "map">("home")
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
  const [navigationHistory, setNavigationHistory] = useState<Array<{tab: typeof activeTab, showDetails: boolean, instance: Instance | null}>>([])
  const [historyIndex, setHistoryIndex] = useState(-1)
  const [isNavigating, setIsNavigating] = useState(false)
  const [sidebarBackground, setSidebarBackground] = useState<string | null>(null)
  const [showSettingsModal, setShowSettingsModal] = useState(false)
  const [sidebarContextMenu, setSidebarContextMenu] = useState<{
    x: number
    y: number
    instance: Instance
  } | null>(null)
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null)
  const [showUpdateNotification, setShowUpdateNotification] = useState(false)
  const [isInstallingUpdate, setIsInstallingUpdate] = useState(false)

  const appWindow = getCurrentWindow()

  const pushToHistory = (tab: typeof activeTab, showDetails: boolean, instance: Instance | null) => {
    if (isNavigating) return
    
    setNavigationHistory(prev => {
      const newHistory = prev.slice(0, historyIndex + 1)
      newHistory.push({ tab, showDetails, instance })
      return newHistory
    })
    setHistoryIndex(prev => prev + 1)
  }

  const navigateBack = () => {
    if (historyIndex > 0) {
      setIsNavigating(true)
      const newIndex = historyIndex - 1
      const state = navigationHistory[newIndex]
      setActiveTab(state.tab)
      setShowInstanceDetails(state.showDetails)
      if (state.instance) {
        setSelectedInstance(state.instance)
      }
      setHistoryIndex(newIndex)
      setTimeout(() => setIsNavigating(false), 0)
    }
  }

  const navigateForward = () => {
    if (historyIndex < navigationHistory.length - 1) {
      setIsNavigating(true)
      const newIndex = historyIndex + 1
      const state = navigationHistory[newIndex]
      setActiveTab(state.tab)
      setShowInstanceDetails(state.showDetails)
      if (state.instance) {
        setSelectedInstance(state.instance)
      }
      setHistoryIndex(newIndex)
      setTimeout(() => setIsNavigating(false), 0)
    }
  }

  const checkForUpdates = async () => {
    try {
      const update = await invoke<UpdateInfo | null>("check_for_updates")
      if (update) {
        console.log("Update available:", update)
        
        // Check if user has dismissed this version before
        const dismissedVersion = localStorage.getItem("dismissed_update_version")
        if (dismissedVersion !== update.new_version) {
          setUpdateInfo(update)
          setShowUpdateNotification(true)
        }
      }
    } catch (error) {
      console.error("Failed to check for updates:", error)
    }
  }

  const handleInstallUpdate = async () => {
    if (!updateInfo) return
    
    setIsInstallingUpdate(true)
    try {
      await invoke("install_update")
      // The app will restart automatically after successful installation
    } catch (error) {
      console.error("Failed to install update:", error)
      setAlertModal({
        isOpen: true,
        title: "Update Failed",
        message: `Failed to install update: ${error}`,
        type: "danger"
      })
      setIsInstallingUpdate(false)
      setShowUpdateNotification(false)
    }
  }

  const handleUpdateLater = () => {
    if (updateInfo) {
      sessionStorage.setItem("dismissed_update_version", updateInfo.new_version)
    }
    setShowUpdateNotification(false)
  }

  const tabs = [
    { id: "home" as const, icon: Home, label: "Home" },
    { id: "instances" as const, icon: Package, label: "Instances" },
    { id: "browse" as const, icon: Puzzle, label: "Mods" },
    { id: "servers" as const, icon: Server, label: "Servers" },
    { id: "skins" as const, icon: HatGlasses, label: "Skins" },
    { id: "map" as const, icon: Telescope, label: "Server Maps" },
    { id: "console" as const, icon: Terminal, label: "Console" },
  ]

  useEffect(() => {
    const initializeApp = async () => {
      await new Promise(resolve => setTimeout(resolve, 100))
      setIsReady(true)

      const splash = document.getElementById('splash-screen')
      const root = document.getElementById('root')
      
      if (splash && root) {
        splash.classList.add('hidden')
        root.classList.add('visible')
        setTimeout(() => {
          splash.remove()
        }, 500)
      }

      // Check for updates after app is ready
      checkForUpdates()
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
    loadSidebarBackground()
    
    const unlistenConsole = listen<ConsoleLog>("console-log", (event) => {
      setConsoleLogs((prev) => [...prev, event.payload])
    })
    
    const unlistenExit = listen<{ instance: string }>("instance-exited", (event) => {
      setRunningInstances((prev) => {
        const newSet = new Set(prev)
        newSet.delete(event.payload.instance)
        return newSet
      })
      setLaunchingInstanceName(null)
    })
    
    const unlistenServerLaunch = listen<{ instance: string, server: string }>("server-instance-launching", (event) => {
      console.log(`Server launch detected: ${event.payload.instance} connecting to ${event.payload.server}`)
      setLaunchingInstanceName(event.payload.instance)
      setConsoleLogs([])
      setActiveTab("console")
      setRunningInstances((prev) => new Set(prev).add(event.payload.instance))
    })
    
    return () => {
      unlistenConsole.then((fn) => fn())
      unlistenExit.then((fn) => fn())
      unlistenServerLaunch.then((fn) => fn())
    }
  }, [isReady])

  useEffect(() => {
    if (!isNavigating && isReady) {
      pushToHistory(activeTab, showInstanceDetails, selectedInstance)
    }
  }, [activeTab, showInstanceDetails, selectedInstance?.name])

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

  const loadSidebarBackground = async () => {
    try {
      const bg = await invoke<string | null>("get_sidebar_background")
      setSidebarBackground(bg)
    } catch (error) {
      console.error("Failed to load sidebar background:", error)
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

  const handleLaunch = async (instance: Instance) => {
    if (!activeAccount) return
    setLaunchingInstanceName(instance.name)
    setConsoleLogs([])
    setActiveTab("console")
    setShowInstanceDetails(false)
    try {
      await invoke<string>("launch_instance_with_active_account", {
        instanceName: instance.name,
        appHandle: appWindow,
      })
      await loadInstances()
      setRunningInstances((prev) => new Set(prev).add(instance.name))
      setLaunchingInstanceName(null)
    } catch (error) {
      console.error("Launch error:", error)
      setLaunchingInstanceName(null)
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
    
    setLaunchingInstanceName(instance.name)
    setConsoleLogs([])
    setActiveTab("console")
    
    try {
      await invoke<string>("launch_instance_with_active_account", {
        instanceName: instance.name,
        appHandle: appWindow,
      })
      await loadInstances()
      setRunningInstances((prev) => new Set(prev).add(instance.name))
      setLaunchingInstanceName(null)
    } catch (error) {
      console.error("Launch error:", error)
      setLaunchingInstanceName(null)
    }
  }

  const handleKillInstance = async (instance: Instance) => {
    try {
      await invoke("kill_instance", { instanceName: instance.name })
      setRunningInstances((prev) => {
        const newSet = new Set(prev)
        newSet.delete(instance.name)
        return newSet
      })
    } catch (error) {
      console.error("Failed to kill instance:", error)
      setAlertModal({
        isOpen: true,
        title: "Error",
        message: `Failed to stop instance: ${error}`,
        type: "danger"
      })
    }
  }

  return (
    <div className="flex flex-col h-screen bg-[#101010] text-[#e6edf3] overflow-hidden font-sans">
      <div 
        data-tauri-drag-region
        style={{ userSelect: 'none', WebkitAppRegion: 'drag' } as any}
        className="h-10 bg-[#1a1a1a] flex-shrink-0 fixed top-0 left-0 right-0 z-50 flex items-center px-4 border-b border-[#2a2a2a]"
      >
        <div className="flex items-center gap-2 mr-4">
          <img src="/logo.png" alt="Atomic Launcher" className="h-5 w-5" />
          <span className="text-sm font-semibold text-[#e6edf3]">Atomic Launcher</span>
        </div>
        
        <div className="flex items-center gap-1 mr-4" style={{ WebkitAppRegion: 'no-drag' } as any}>
          <button
            onClick={navigateBack}
            disabled={historyIndex <= 0}
            className={`h-7 w-7 flex items-center justify-center rounded transition-colors ${
              historyIndex > 0
                ? "text-[#e6edf3] hover:bg-[#2a2a2a] cursor-pointer" 
                : "text-[#3a3a3a] cursor-not-allowed"
            }`}
          >
            <ChevronLeft size={16} strokeWidth={4} />
          </button>
          <button
            onClick={navigateForward}
            disabled={historyIndex >= navigationHistory.length - 1}
            className={`h-7 w-7 flex items-center justify-center rounded transition-colors ${
              historyIndex < navigationHistory.length - 1
                ? "text-[#e6edf3] hover:bg-[#2a2a2a] cursor-pointer" 
                : "text-[#3a3a3a] cursor-not-allowed"
            }`}
          >
            <ChevronRight size={16} strokeWidth={4} />
          </button>
        </div>

        <div className="flex-1 flex items-center justify-center" style={{ WebkitAppRegion: 'drag' } as any}>
          <nav className="flex items-center gap-1" style={{ WebkitAppRegion: 'no-drag' } as any}>
            {tabs.map((tab) => {
              const Icon = tab.icon
              return (
                <button
                  key={tab.id}
                  onClick={() => {
                    setActiveTab(tab.id)
                    setShowInstanceDetails(false)
                  }}
                  className={`flex items-center gap-2 px-3 py-1.5 rounded text-sm font-medium transition-all cursor-pointer ${
                    activeTab === tab.id
                      ? "bg-[#101010] text-[#e6edf3]"
                      : "text-[#7d8590] hover:text-[#e6edf3] hover:bg-[#212121]"
                  }`}
                >
                  <Icon size={16} strokeWidth={2} />
                  <span>{tab.label}</span>
                </button>
              )
            })}
          </nav>
        </div>

        <div className="flex items-center" style={{ WebkitAppRegion: 'no-drag' } as any}>
          <button
            onClick={() => setShowSettingsModal(true)}
            className="flex items-center justify-center px-3 py-1.5 rounded text-[#7d8590] hover:text-[#e6edf3] hover:bg-[#212121] transition-all cursor-pointer"
            title="Settings"
          >
            <Settings size={16} strokeWidth={2} />
          </button>
          <div className="w-px h-6 bg-[#2a2a2a] mx-2" style={{ pointerEvents: 'none' } as any} />
          <button
            onClick={() => appWindow.minimize()}
            className="h-10 w-12 flex items-center justify-center text-[#7d8590] hover:text-[#e6edf3] hover:bg-[#2a2a2a] transition-colors"
          >
            <Minus size={16} />
          </button>
          <button
            onClick={() => appWindow.toggleMaximize()}
            className="h-10 w-12 flex items-center justify-center text-[#7d8590] hover:text-[#e6edf3] hover:bg-[#2a2a2a] transition-colors"
          >
            <Square size={14} />
          </button>
          <button
            onClick={() => appWindow.close()}
            className="h-10 w-12 flex items-center justify-center text-[#7d8590] hover:text-[#e6edf3] hover:bg-red-500 transition-colors"
          >
            <X size={16} />
          </button>
        </div>
      </div>

      {creatingInstanceName && (
        <CreationProgressToast
          instanceName={creatingInstanceName}
          onComplete={handleCreationComplete}
          onError={handleCreationError}
          onDismiss={() => setCreatingInstanceName(null)}
        />
      )}

      {showUpdateNotification && updateInfo && (
        <UpdateNotificationToast
          currentVersion={updateInfo.current_version}
          newVersion={updateInfo.new_version}
          isInstalling={isInstallingUpdate}
          onInstall={handleInstallUpdate}
          onLater={handleUpdateLater}
        />
      )}

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

      <div className="flex flex-1 overflow-hidden mt-10">
        <Sidebar
          activeTab={activeTab}
          setActiveTab={setActiveTab}
          showInstanceDetails={showInstanceDetails}
          setShowInstanceDetails={setShowInstanceDetails}
          instances={instances}
          instanceIcons={instanceIcons}
          runningInstances={runningInstances}
          launchingInstanceName={launchingInstanceName}
          isAuthenticated={isAuthenticated}
          activeAccount={activeAccount}
          accounts={accounts}
          sidebarBackground={sidebarBackground}
          sidebarContextMenu={sidebarContextMenu}
          setSidebarContextMenu={setSidebarContextMenu}
          setSelectedInstance={setSelectedInstance}
          setShowSettingsModal={setShowSettingsModal}
          onQuickLaunch={handleQuickLaunch}
          onKillInstance={handleKillInstance}
          onOpenInstanceFolder={handleOpenInstanceFolderByInstance}
          onDuplicateInstance={handleDuplicateInstance}
          onDeleteInstance={handleDeleteInstance}
          loadAccounts={loadAccounts}
        />

        <main className="flex-1 overflow-y-auto bg-[#101010]">
          {showInstanceDetails && selectedInstance ? (
            <InstanceDetailsTab
              instance={selectedInstance}
              isAuthenticated={isAuthenticated}
              isLaunching={launchingInstanceName === selectedInstance.name}
              isRunning={runningInstances.has(selectedInstance.name)}
              onLaunch={() => handleLaunch(selectedInstance)}
              onBack={handleCloseDetails}
              onInstanceUpdated={loadInstances}
            />
          ) : (
            <>
              {activeTab === "home" && (
                <HomeTab
                  instances={instances}
                  isAuthenticated={isAuthenticated}
                  launchingInstanceName={launchingInstanceName}
                  runningInstances={runningInstances}
                  onLaunch={handleLaunch}
                  onOpenFolder={handleOpenInstanceFolder}
                  onDeleteInstance={handleDeleteInstance}
                  onCreateNew={() => setShowCreateModal(true)}
                  onShowDetails={handleShowDetails}
                  onOpenFolderByInstance={handleOpenInstanceFolderByInstance}
                  onDuplicateInstance={handleDuplicateInstance}
                  onRefreshInstances={loadInstances}
                  onKillInstance={handleKillInstance}
                />
              )}

              {activeTab === "instances" && (
                <InstancesTab
                  instances={instances}
                  selectedInstance={selectedInstance}
                  isAuthenticated={isAuthenticated}
                  launchingInstanceName={launchingInstanceName}
                  runningInstances={runningInstances}
                  onSetSelectedInstance={setSelectedInstance}
                  onLaunch={() => selectedInstance && handleLaunch(selectedInstance)}
                  onCreateNew={() => setShowCreateModal(true)}
                  onShowDetails={handleShowDetails}
                  onOpenFolder={handleOpenInstanceFolderByInstance}
                  onDuplicateInstance={handleDuplicateInstance}
                  onDeleteInstance={handleDeleteInstance}
                  onKillInstance={handleKillInstance}
                />
              )}

              {activeTab === "browse" && (
                <BrowseTab
                  selectedInstance={selectedInstance}
                  instances={instances}
                  onSetSelectedInstance={setSelectedInstance}
                  onRefreshInstances={loadInstances}
                  onShowCreationToast={handleStartCreating}
                />
              )}

              {activeTab === "servers" && (
                <ServersTab 
                  launchingInstanceName={launchingInstanceName}
                  runningInstances={runningInstances}
                />
              )}

              {activeTab === "skins" && (
                <SkinsTab
                  activeAccount={activeAccount}
                  isAuthenticated={isAuthenticated}
                  invoke={invoke}
                />
              )}

              {activeTab === "map" && (
                <MapTab />
              )}

              {activeTab === "console" && (
                <ConsoleTab
                  consoleLogs={consoleLogs}
                  onClearConsole={(instanceName: string) => {
                    setConsoleLogs(prev => prev.filter(log => log.instance !== instanceName))
                  }}
                />
              )}
            </>
          )}
        </main>
      </div>

      <SettingsModal
        isOpen={showSettingsModal}
        settings={settings}
        launcherDirectory={launcherDirectory}
        onClose={() => setShowSettingsModal(false)}
        onSettingsChange={setSettings}
        onBackgroundChanged={loadSidebarBackground}
      />

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