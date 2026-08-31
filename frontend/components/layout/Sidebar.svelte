<script lang="ts">
  import { House, LibraryBig, Package, Blocks, Shirt, Images, SquareTerminal, HardDrive, Settings, Download, Plus, FolderOpen, Copy, Trash2 } from "lucide-svelte"
  import ContextMenu from "../ui/ContextMenu.svelte"
  import Tooltip from "../ui/Tooltip.svelte"
  import {
    store, setActiveTab, setShowInstanceDetails,
    setSidebarContextMenu, setSelectedInstance,
    handleInstallUpdate, handleOpenSettings, handleCreateNew,
    handleOpenInstanceFolderByInstance, handleDuplicateInstance, handleDeleteInstance,
  } from "../../lib/launcherStore.svelte"

  const tabs = [
    { id: "home" as const, icon: House, label: "Home" },
    { id: "instances" as const, icon: LibraryBig, label: "Instances" },
    { id: "addons" as const, icon: Blocks, label: "Addons" },
    { id: "skins" as const, icon: Shirt, label: "Skins" },
    { id: "screenshots" as const, icon: Images, label: "Screenshots" },
    { id: "servers" as const, icon: HardDrive, label: "Servers" },
    { id: "console" as const, icon: SquareTerminal, label: "Console" },
  ]
</script>

<div class="w-10 flex-shrink-0 flex flex-col items-center gap-1 relative z-10">
  <div class="flex flex-col items-center gap-2.5 flex-1">
    {#each tabs as tab}
      {@const Icon = tab.icon}
      {@const isActive = store.activeTab === tab.id}
      <Tooltip text={tab.label}>
        <button
          onclick={() => { setActiveTab(tab.id); setShowInstanceDetails(false) }}
          class="w-11 h-11 flex items-center justify-center rounded-lg transition-all cursor-pointer {isActive ? 'bg-[var(--bg-active)] text-[var(--accent-primary)]' : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-active)]'}"
        >
          <Icon size={28} strokeWidth={2} />
        </button>
      </Tooltip>
    {/each}
  </div>

  <div class="flex flex-col items-center gap-2">
    {#if store.updateInfo}
      <Tooltip text={store.isInstallingUpdate ? "Installing update..." : `Update available: ${store.updateInfo.new_version}`}>
        <button
          onclick={handleInstallUpdate}
          disabled={store.isInstallingUpdate}
          class="w-11 h-11 flex items-center justify-center rounded-lg transition-all cursor-pointer {store.isInstallingUpdate ? 'text-[#16a34a] animate-pulse' : 'text-[#16a34a] hover:bg-[var(--bg-active)]'}"
        >
          <Download size={28} strokeWidth={2} />
        </button>
      </Tooltip>
    {/if}

    <Tooltip text="New instance">
      <button
        onclick={handleCreateNew}
        class="w-11 h-11 flex items-center justify-center rounded-lg text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-active)] transition-all cursor-pointer"
      >
        <Plus size={28} strokeWidth={2} />
      </button>
    </Tooltip>

    <Tooltip text="Settings">
      <button
        onclick={handleOpenSettings}
        class="w-11 h-11 flex items-center justify-center rounded-lg text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-active)] transition-all cursor-pointer"
      >
        <Settings size={28} strokeWidth={2} />
      </button>
    </Tooltip>
  </div>
</div>

{#if store.sidebarContextMenu}
  {@const scm = store.sidebarContextMenu}
  <ContextMenu
    x={scm.x}
    y={scm.y}
    onClose={() => setSidebarContextMenu(null)}
    items={[
      {
        label: "Open",
        icon: Package,
        onClick: () => {
          setSelectedInstance(scm.instance)
          setActiveTab("instances")
          setShowInstanceDetails(true)
        },
      },
      {
        label: "Open Folder",
        icon: FolderOpen,
        onClick: () => handleOpenInstanceFolderByInstance(scm.instance),
      },
      {
        label: "Duplicate",
        icon: Copy,
        onClick: () => handleDuplicateInstance(scm.instance),
      },
      { separator: true },
      {
        label: "Delete",
        icon: Trash2,
        onClick: () => handleDeleteInstance(scm.instance.name),
        danger: true,
      },
    ]}
  />
{/if}
