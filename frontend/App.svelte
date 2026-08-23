<script lang="ts">
  import TitleBar from "./components/layout/TitleBar.svelte"
  import Sidebar from "./components/layout/Sidebar.svelte"
  import FriendsPanel from "./features/social/FriendsPanel.svelte"
  import SettingsModal from "./features/settings/SettingsModal.svelte"
  import CreateInstanceModal from "./features/instances/CreateInstanceModal.svelte"
  import CreationProgressToast from "./features/instances/CreationProgressToast.svelte"
  import InstanceDetailsTab from "./features/instances/InstanceDetailsTab.svelte"
  import ConfirmModal from "./components/ui/ConfirmModal.svelte"
  import AlertModal from "./components/ui/AlertModal.svelte"
  import ExportModal from "./features/instances/ExportModal.svelte"
  import OnboardingModal from "./features/onboarding/OnboardingModal.svelte"
  import GlobalSearch from "./components/ui/GlobalSearch.svelte"
  import Toaster from "./components/ui/Toaster.svelte"
  import HomeTab from "./features/home/HomeTab.svelte"
  import InstancesTab from "./features/instances/InstancesTab.svelte"
  import AddonsTab from "./features/addons/AddonsTab.svelte"
  import ConsoleTab from "./features/console/ConsoleTab.svelte"
  import ServersTab from "./features/servers/ServersTab.svelte"
  import SkinsTab from "./features/skins/SkinsTab.svelte"
  import ScreenshotsTab from "./features/screenshots/ScreenshotsTab.svelte"
  import {
    store,
    setShowSettingsModal, setShowCreateModal, setConfirmModal,
    setAlertModal, loadAllInitialData, setupEventListeners, pushToHistory,
    handleStartCreating, handleCreationComplete, handleCreationError,
    setExportModal, setShowSearchPalette,
  } from "./lib/launcherStore.svelte"
  import { onMount, untrack } from "svelte"

  function handleGlobalKeydown(e: KeyboardEvent) {
    const key = e.key.toLowerCase()
    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && key === "f") {
      e.preventDefault()
      setShowSearchPalette(!store.showSearchPalette)
    }
  }

  $effect(() => {
    window.addEventListener("keydown", handleGlobalKeydown)
    return () => window.removeEventListener("keydown", handleGlobalKeydown)
  })

  $effect(() => {
    const theme = store.settings?.theme || "octane"
    const root = document.documentElement
    for (const cls of Array.from(root.classList)) {
      if (cls.startsWith("theme-")) root.classList.remove(cls)
    }
    root.classList.add(`theme-${theme}`)
  })

  onMount(() => {
    setTimeout(async () => {
      store.isReady = true
      const splash = document.getElementById("splash-screen")
      const root = document.getElementById("root")
      if (splash && root) {
        splash.classList.add("hidden")
        root.classList.add("visible")
        setTimeout(() => splash.remove(), 500)
      }
      loadAllInitialData()
    }, 100)
  })

  $effect(() => {
    if (!store.isReady) return
    return setupEventListeners()
  })

  $effect(() => {
    if (!store.isReady) return
    store.activeTab
    store.showInstanceDetails
    store.selectedInstance?.name
    untrack(pushToHistory)
  })
</script>

<!-- preload avatar keeps the browser decode cache warm across tab switches -->
{#if store.activeAccount}
  <img src="https://renders.stellarmc.gg/bust/{store.activeAccount.username}" alt="" aria-hidden="true" class="fixed opacity-0 pointer-events-none" />
{/if}

<div class="flex flex-col h-screen bg-[var(--bg-primary)] text-[var(--text-primary)] overflow-hidden font-sans {store.settings?.theme ? `theme-${store.settings.theme}` : 'theme-octane'}">
  <TitleBar />

  <div class="flex flex-1 overflow-hidden px-4 gap-4">
    <Sidebar />

    <div
      class="flex-1 rounded-xl overflow-hidden flex flex-col relative"
      style={store.background
        ? `background-image: url(${store.background}); background-size: cover; background-position: center`
        : 'background-color: var(--content-bg)'}
    >
      {#if store.background}
        <div class="absolute inset-0 bg-black/80"></div>
      {/if}

      <main class="flex-1 min-h-0 overflow-y-auto relative z-10">
        {#if store.showInstanceDetails && store.selectedInstance}
          <InstanceDetailsTab
            instance={store.selectedInstance}
          />
        {:else if store.activeTab === "home"}
          <HomeTab />
        {:else if store.activeTab === "instances"}
          <InstancesTab />
        {:else if store.activeTab === "addons"}
          <AddonsTab />
        {:else if store.activeTab === "servers"}
          <ServersTab />
        {:else if store.activeTab === "skins"}
          <SkinsTab />
        {:else if store.activeTab === "screenshots"}
          <ScreenshotsTab />
        {:else if store.activeTab === "console"}
          <ConsoleTab />
        {/if}
      </main>

      {#if store.creatingInstanceName}
        <div class="absolute bottom-0 left-0 right-0 z-20">
          <CreationProgressToast instanceName={store.creatingInstanceName} onDismiss={() => store.creatingInstanceName = null} onError={handleCreationError} />
        </div>
      {/if}
    </div>

    <FriendsPanel isOpen={store.showFriendsPanel} isAuthenticated={store.isAuthenticated} activeAccountUuid={store.activeAccount?.uuid} />
  </div>

  <div class="flex flex-shrink-0 px-4 pb-4">
    <div class="w-14 flex-shrink-0"></div>
    <div class="flex-1 h-0"></div>
  </div>

  {#if store.confirmModal}
    <ConfirmModal
      isOpen={store.confirmModal.isOpen}
      title={store.confirmModal.title}
      message={store.confirmModal.message}
      type={store.confirmModal.type}
      confirmText={store.confirmModal.type === "danger" ? "Delete" : "Confirm"}
      onConfirm={store.confirmModal.onConfirm}
      onCancel={() => setConfirmModal(null)}
      checkboxLabel={store.confirmModal.checkboxLabel}
      checkboxChecked={store.confirmModal.checkboxChecked}
      onCheckboxChange={store.confirmModal.onCheckboxChange}
    />
  {/if}

  {#if store.alertModal}
    <AlertModal
      isOpen={store.alertModal.isOpen}
      title={store.alertModal.title}
      message={store.alertModal.message}
      type={store.alertModal.type}
      onClose={() => setAlertModal(null)}
    />
  {/if}

  {#if store.exportModalInstance}
    <ExportModal
      instanceName={store.exportModalInstance.name}
      onClose={() => setExportModal(null)}
    />
  {/if}

  <SettingsModal
    isOpen={store.showSettingsModal}
    onClose={() => setShowSettingsModal(false)}
  />

  {#if store.showCreateModal}
    <CreateInstanceModal
      instances={store.instances}
      onClose={() => setShowCreateModal(false)}
      onSuccess={() => { handleCreationComplete() }}
      onStartCreating={(name) => { handleStartCreating(name) }}
    />
  {/if}

  {#if store.showOnboarding}
    <OnboardingModal />
  {/if}

  <GlobalSearch />

  <Toaster />
</div>
