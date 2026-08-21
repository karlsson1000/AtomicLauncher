import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { getCurrentWindow } from "@tauri-apps/api/window"
import type { Instance, LauncherSettings, ConsoleLog, AccountInfo, UpdateInfo } from "../types"
import { storeGet, storeSet } from "./store"

type ActiveTab = "home" | "instances" | "browse" | "console" | "servers" | "skins" | "screenshots"
type BrowseSubTab = "mods" | "modpacks" | "resourcepacks" | "shaderpacks"
type ModalType = "warning" | "danger" | "success" | "info"

interface ConfirmModalState {
  isOpen: boolean
  title: string
  message: string
  type: ModalType
  onConfirm: () => void
  checkboxLabel?: string
  checkboxChecked?: boolean
  onCheckboxChange?: (checked: boolean) => void
}

interface AlertModalState {
  isOpen: boolean
  title: string
  message: string
  type: ModalType
}

interface NavEntry {
  tab: ActiveTab
  showDetails: boolean
  instance: Instance | null
}

interface SidebarMenu {
  x: number
  y: number
  instance: Instance
}

export const store = $state({
  isReady: false,
  isAuthenticated: false,
  activeAccount: null as AccountInfo | null,
  accounts: [] as AccountInfo[],
  launchingInstanceName: null as string | null,
  runningInstances: new Set<string>(),
  showCreateModal: false,
  versions: [] as string[],
  instances: [] as Instance[],
  selectedInstance: null as Instance | null,
  launcherDirectory: "",
  settings: null as LauncherSettings | null,
  activeTab: "home" as ActiveTab,
  consoleLogs: [] as ConsoleLog[],
  showInstanceDetails: false,
  creatingInstanceName: null as string | null,
  confirmModal: null as ConfirmModalState | null,
  alertModal: null as AlertModalState | null,
  navigationHistory: [] as NavEntry[],
  historyIndex: -1,
  isNavigating: false,
  background: null as string | null,
  showSettingsModal: false,
  sidebarContextMenu: null as SidebarMenu | null,
  updateInfo: null as UpdateInfo | null,
  isInstallingUpdate: false,
  showAccountDropdown: false,
  showFriendsPanel: false,
  browseSubTab: "mods" as BrowseSubTab,
  showOnboarding: false,
  exportModalInstance: null as Instance | null,
})

const appWindow = getCurrentWindow()
const dragRegion = "webkit-app-region: drag"
const noDragRegion = "webkit-app-region: no-drag"

function pushToHistory() {
  if (store.isNavigating) return
  store.navigationHistory = [...store.navigationHistory.slice(0, store.historyIndex + 1), { tab: store.activeTab, showDetails: store.showInstanceDetails, instance: store.selectedInstance }]
  store.historyIndex = store.navigationHistory.length - 1
}

function navigateBack() {
  if (store.historyIndex > 0) {
    store.isNavigating = true
    const newIndex = store.historyIndex - 1
    const state = store.navigationHistory[newIndex]
    store.activeTab = state.tab
    store.showInstanceDetails = state.showDetails
    if (state.instance) store.selectedInstance = state.instance
    store.historyIndex = newIndex
    setTimeout(() => store.isNavigating = false, 0)
  }
}

function navigateForward() {
  if (store.historyIndex < store.navigationHistory.length - 1) {
    store.isNavigating = true
    const newIndex = store.historyIndex + 1
    const state = store.navigationHistory[newIndex]
    store.activeTab = state.tab
    store.showInstanceDetails = state.showDetails
    if (state.instance) store.selectedInstance = state.instance
    store.historyIndex = newIndex
    setTimeout(() => store.isNavigating = false, 0)
  }
}

async function checkForUpdates() {
  try {
    const update = await invoke<string | null>("check_for_updates")
    if (update) {
      const parts = update.split(' -> ')
      if (parts.length === 2) {
        const info: UpdateInfo = {
          current_version: parts[0].trim(),
          new_version: parts[1].trim()
        }
        const dismissedVersion = await storeGet<string>("dismissed_update_version")
        if (dismissedVersion !== info.new_version) {
          store.updateInfo = info
        }
      }
    }
  } catch (error) {
    console.error("Failed to check for updates:", error)
  }
}

async function handleInstallUpdate() {
  if (!store.updateInfo) return
  store.isInstallingUpdate = true
  try {
    await invoke("install_update")
    store.isInstallingUpdate = false
  } catch (error) {
    console.error("Failed to install update:", error)
    store.alertModal = {
      isOpen: true,
      title: "Update Failed",
      message: `Failed to install update: ${error}`,
      type: "danger"
    }
    store.isInstallingUpdate = false
  }
}

async function loadVersions() {
  try {
    const versionList = await invoke<string[]>("get_minecraft_versions")
    store.versions = versionList
  } catch (error) {
    console.error("Failed to load versions:", error)
  }
}

async function loadInstances(renamedFrom?: string, renamedTo?: string) {
  try {
    const instanceList = await invoke<Instance[]>("get_instances")
    store.instances = instanceList
    if (store.selectedInstance) {
      let searchName = store.selectedInstance.name
      if (renamedFrom && renamedTo && store.selectedInstance.name === renamedFrom) {
        searchName = renamedTo
      }
      const updated = instanceList.find(i => i.name === searchName)
      if (updated) store.selectedInstance = updated
      else store.selectedInstance = instanceList[0] || null
    } else if (instanceList.length > 0 && !store.selectedInstance) {
      store.selectedInstance = instanceList[0]
    }
  } catch (error) {
    console.error("Failed to load instances:", error)
  }
}

async function handleInstanceRenamed(oldName: string, newName: string) {
  await loadInstances(oldName, newName)
}

async function loadLauncherDirectory() {
  try {
    const dir = await invoke<string>("get_launcher_directory")
    store.launcherDirectory = dir
  } catch (error) {
    console.error("Failed to get launcher directory:", error)
  }
}

async function loadSettings() {
  try {
    const s = await invoke<LauncherSettings>("get_settings")
    store.settings = s
  } catch (e) {
    console.error("Failed to load settings", e)
  }
}

async function loadBackground() {
  try {
    const bg = await invoke<string | null>("get_background")
    store.background = bg
  } catch (error) {
    console.error("Failed to load background:", error)
  }
}

async function loadAccounts() {
  try {
    const accountList = await invoke<AccountInfo[]>("get_accounts")
    store.accounts = accountList
    const active = accountList.find(acc => acc.is_active)
    store.activeAccount = active || null
    store.isAuthenticated = !!active
    if (active) {
      invoke("register_user_in_friends_system").catch(() => {})
      startHeartbeat()
    } else {
      stopHeartbeat()
    }
  } catch (error) {
    console.error("Failed to load accounts:", error)
  }
}

async function handleLaunch(instance: Instance) {
  if (!store.activeAccount) return
  store.launchingInstanceName = instance.name
  store.consoleLogs = []
  store.consoleLogs = [{ instance: instance.name, message: `Launching ${instance.name}...`, type: "stdout" }]
  if (store.settings?.auto_navigate_to_console !== false) {
    store.activeTab = "console"
    store.showInstanceDetails = false
  }
  try {
    await invoke<string>("launch_instance_with_active_account", {
      instanceName: instance.name,
    })
    store.consoleLogs = [...store.consoleLogs, { instance: instance.name, message: "Minecraft started", type: "stdout" }]
    await loadInstances()
    store.runningInstances = new Set(store.runningInstances).add(instance.name)
    store.launchingInstanceName = null
  } catch (error) {
    console.error("Launch error:", error)
    store.consoleLogs = [...store.consoleLogs, { instance: instance.name, message: `ERROR: ${String(error)}`, type: "stderr" }]
    store.launchingInstanceName = null
  }
}

let skipTrash = false

async function handleDeleteInstance(instanceName: string) {
  skipTrash = false
  store.confirmModal = {
    isOpen: true,
    title: "Delete Instance",
    message: `Are you sure you want to delete "${instanceName}"?\n\nThis action cannot be undone.`,
    type: "danger",
    checkboxLabel: "Delete permanently",
    onCheckboxChange: (checked: boolean) => { skipTrash = checked },
    onConfirm: async () => {
      store.confirmModal = null
      try {
        await invoke<string>("delete_instance", { instanceName, permanent: skipTrash })
        await loadInstances()
        if (store.selectedInstance?.name === instanceName) {
          store.selectedInstance = store.instances.length > 1 ? store.instances.find(i => i.name !== instanceName) || null : null
        }
      } catch (error) {
        console.error("Delete error:", error)
      }
    }
  }
}

async function handleDuplicateInstance(instance: Instance) {
  let baseName = instance.name
  let counter = 1
  let newName = `${baseName} (Copy)`
  while (store.instances.some(i => i.name === newName)) {
    counter++
    newName = `${baseName} (Copy ${counter})`
  }
  store.creatingInstanceName = newName
  try {
    await invoke("duplicate_instance", {
      instanceName: instance.name,
      newName,
    })
    await loadInstances()
  } catch (error) {
    console.error("Duplicate error:", error)
    store.creatingInstanceName = null
    store.alertModal = {
      isOpen: true,
      title: "Error",
      message: `Failed to duplicate instance: ${error}`,
      type: "danger"
    }
  }
}

async function handleOpenInstanceFolderByInstance(instance: Instance) {
  try {
    await invoke("open_instance_folder", { instanceName: instance.name })
  } catch (error) {
    console.error("Failed to open folder:", error)
  }
}

function handleShowDetails(instance: Instance) {
  store.selectedInstance = instance
  store.showInstanceDetails = true
}

function handleCloseDetails() {
  store.showInstanceDetails = false
}

function handleStartCreating(instanceName: string) {
  store.creatingInstanceName = instanceName
  store.activeTab = 'instances'
}

async function handleCreationComplete() {
  store.creatingInstanceName = null
  await loadInstances()
}

function handleCreationError() {
  store.creatingInstanceName = null
}

async function handleKillInstance(instance: Instance) {
  try {
    await invoke("kill_instance", { instanceName: instance.name })
    const newSet = new Set(store.runningInstances)
    newSet.delete(instance.name)
    store.runningInstances = newSet
  } catch (error) {
    console.error("Failed to kill instance:", error)
    store.alertModal = {
      isOpen: true,
      title: "Error",
      message: `Failed to stop instance: ${error}`,
      type: "danger"
    }
  }
}

function handleOpenSettings() { store.showSettingsModal = true }
function handleCreateNew() { store.showCreateModal = true }
function handleClearConsole(instanceName: string) {
  store.consoleLogs = store.consoleLogs.filter(log => log.instance !== instanceName)
}
function handleNavigateToInstances() { store.activeTab = "instances" }

async function handleWorldLaunch(worldName: string) {
  if (!store.selectedInstance || !store.activeAccount) return
  store.launchingInstanceName = store.selectedInstance.name
  store.consoleLogs = []
  if (store.settings?.auto_navigate_to_console !== false) {
    store.activeTab = "console"
    store.showInstanceDetails = false
  }
  try {
    await invoke("launch_world", { instanceName: store.selectedInstance.name, worldName })
    await loadInstances()
    store.runningInstances = new Set(store.runningInstances).add(store.selectedInstance.name)
    store.launchingInstanceName = null
  } catch (error) {
    console.error("World launch error:", error)
    store.launchingInstanceName = null
  }
}

// Setter functions (for use from other modules)
function setShowCreateModal(v: boolean) { store.showCreateModal = v }
function setSelectedInstance(v: Instance | null) { store.selectedInstance = v }
function setSettings(v: LauncherSettings | null) { store.settings = v }
function setActiveTab(v: ActiveTab) { store.activeTab = v }
function setShowInstanceDetails(v: boolean) { store.showInstanceDetails = v }
function setCreatingInstanceName(v: string | null) { store.creatingInstanceName = v }
function setConfirmModal(v: ConfirmModalState | null) { store.confirmModal = v }
function setAlertModal(v: AlertModalState | null) { store.alertModal = v }
function setShowSettingsModal(v: boolean) { store.showSettingsModal = v }
function setSidebarContextMenu(v: SidebarMenu | null) { store.sidebarContextMenu = v }
function setShowAccountDropdown(v: boolean) { store.showAccountDropdown = v }
function setShowFriendsPanel(v: boolean) {
  store.showFriendsPanel = v
  storeSet("friends_panel_open", v)
}
function setBrowseSubTab(v: BrowseSubTab) { store.browseSubTab = v }
function setShowOnboarding(v: boolean) {
  store.showOnboarding = v
  if (!v) storeSet("onboarding_complete", true)
}
function setExportModal(v: Instance | null) { store.exportModalInstance = v }
function generateUniqueName(base: string): string {
  let candidate = base
  let counter = 1
  while (store.instances.some(i => i.name === candidate)) {
    counter++
    candidate = `${base} (${counter})`
  }
  return candidate
}

let heartbeatTimer: ReturnType<typeof setInterval> | undefined

function startHeartbeat() {
  stopHeartbeat()
  const sendHeartbeat = async () => {
    try {
      const instances = store.runningInstances
      const firstInstance = instances.values().next().value
      if (firstInstance) {
        await invoke("update_user_status", { status: "ingame", currentInstance: firstInstance })
      } else {
        await invoke("update_user_status", { status: "online", currentInstance: null })
      }
    } catch {
    }
  }
  sendHeartbeat()
  heartbeatTimer = setInterval(sendHeartbeat, 60000)
}

function stopHeartbeat() {
  if (heartbeatTimer !== undefined) {
    clearInterval(heartbeatTimer)
    heartbeatTimer = undefined
  }
}

let appliedDefaultTab = false

async function loadAllInitialData() {
  await Promise.all([
    loadVersions(),
    loadInstances(),
    loadLauncherDirectory(),
    loadSettings(),
    loadAccounts(),
    loadBackground(),
    storeGet<boolean>("friends_panel_open").then(v => { if (v) store.showFriendsPanel = true }),
    storeGet<boolean>("onboarding_complete").then(v => { if (!v) store.showOnboarding = true }),
  ])
  if (!appliedDefaultTab && store.settings?.default_tab) {
    const validTabs = ["home", "instances", "browse", "console", "servers", "skins", "screenshots"]
    if (validTabs.includes(store.settings.default_tab)) {
      store.activeTab = store.settings.default_tab as ActiveTab
    }
    appliedDefaultTab = true
  }
  checkForUpdates()
}

function setupEventListeners() {
  const MAX_CONSOLE_LOGS = 10000
  let active = true
  const unlistenFns: (() => void)[] = []

  Promise.all([
    listen<ConsoleLog>("console-log", (event) => {
      if (!active) return
      const next = [...store.consoleLogs, event.payload]
      store.consoleLogs = next.length > MAX_CONSOLE_LOGS ? next.slice(-MAX_CONSOLE_LOGS) : next
    }),
    listen<{ instance: string }>("instance-exited", (event) => {
      if (!active) return
      const newSet = new Set(store.runningInstances)
      newSet.delete(event.payload.instance)
      store.runningInstances = newSet
      store.launchingInstanceName = null
    }),
    listen<{ instance: string, server: string }>("server-instance-launching", (event) => {
      if (!active) return
      store.launchingInstanceName = event.payload.instance
      store.consoleLogs = []
      if (store.settings?.auto_navigate_to_console !== false) store.activeTab = "console"
      store.runningInstances = new Set(store.runningInstances).add(event.payload.instance)
    }),
  ]).then(fns => {
    unlistenFns.push(...fns)
  })

  return () => {
    active = false
    unlistenFns.forEach(fn => fn())
  }
}

export {
  appWindow, dragRegion, noDragRegion,
  loadAllInitialData, setupEventListeners, startHeartbeat, stopHeartbeat,
  setShowCreateModal,
  setSelectedInstance,
  setSettings,
  setActiveTab,
  setShowInstanceDetails,
  setCreatingInstanceName,
  setConfirmModal,
  setAlertModal,
  setShowSettingsModal,
  setSidebarContextMenu,
  setShowAccountDropdown,
  setShowFriendsPanel,
  setBrowseSubTab,
  setShowOnboarding,
  setExportModal,
  generateUniqueName,
  navigateBack,
  navigateForward,
  pushToHistory,
  handleInstallUpdate,
  handleLaunch,
  handleDeleteInstance,
  handleDuplicateInstance,
  handleOpenInstanceFolderByInstance,
  handleShowDetails,
  handleCloseDetails,
  handleStartCreating,
  handleCreationComplete,
  handleCreationError,
  handleKillInstance,
  handleOpenSettings,
  handleCreateNew,
  handleClearConsole,
  handleNavigateToInstances,
  handleWorldLaunch,
  loadInstances,
  handleInstanceRenamed,
  loadAccounts,
  loadBackground,
}
