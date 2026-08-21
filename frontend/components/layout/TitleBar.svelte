<script lang="ts">
  import { invoke } from "@tauri-apps/api/core"
  import { Minus, Square, X, ChevronLeft, ChevronRight, ChevronDown, LogIn, LogOut, Check } from "lucide-svelte"
  import {
    store, navigateBack, navigateForward,
    setShowAccountDropdown, loadAccounts, appWindow,
    dragRegion, noDragRegion,
  } from "../../lib/launcherStore.svelte"

  const tabLabels: Record<string, string> = {
    home: "Home", instances: "Instances", browse: "Addons",
    servers: "Servers", skins: "Skins", screenshots: "Screenshots", console: "Console",
  }

  const browseSubTabLabels: Record<string, string> = {
    mods: "Mods", modpacks: "Modpacks", resourcepacks: "Resource Packs", shaderpacks: "Shader Packs",
  }

  $effect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as HTMLElement
      if (!target.closest("[data-account-dropdown]") && !target.closest("[data-account-button]")) {
        setShowAccountDropdown(false)
      }
    }
    if (store.showAccountDropdown) {
      document.addEventListener("mousedown", handleClickOutside)
    }
    return () => document.removeEventListener("mousedown", handleClickOutside)
  })

  let historyLen = $derived(store.navigationHistory.length)
</script>

<div data-tauri-drag-region class="h-9 flex-shrink-0 flex items-center pl-5 pr-3 gap-2 select-none" style={dragRegion}>
  <div class="flex items-center gap-2 flex-shrink-0">
    <img src="/logo.png" alt="Octane" class="h-4 w-4" />
    <span class="text-sm font-semibold text-[var(--text-secondary)]">Octane Launcher</span>
  </div>

  <div class="flex items-center gap-0.5 ml-1" style={noDragRegion}>
    <button
      onclick={navigateBack}
      disabled={store.historyIndex <= 0}
      class="h-6 w-6 flex items-center justify-center rounded transition-colors {store.historyIndex > 0 ? 'text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)] hover:text-[var(--text-primary)] cursor-pointer' : 'text-[var(--text-disabled)] cursor-not-allowed'}"
    >
      <ChevronLeft size={18} strokeWidth={3} />
    </button>
    <button
      onclick={navigateForward}
      disabled={store.historyIndex >= historyLen - 1}
      class="h-6 w-6 flex items-center justify-center rounded transition-colors {store.historyIndex < historyLen - 1 ? 'text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)] hover:text-[var(--text-primary)] cursor-pointer' : 'text-[var(--text-disabled)] cursor-not-allowed'}"
    >
      <ChevronRight size={18} strokeWidth={3} />
    </button>
  </div>

  <span class="text-sm font-medium text-[var(--text-secondary)] ml-1 select-none" style={dragRegion}>
    {store.showInstanceDetails && store.selectedInstance
      ? `Instances / ${store.selectedInstance.name}`
      : store.activeTab === "browse"
      ? `Addons / ${browseSubTabLabels[store.browseSubTab]}`
      : tabLabels[store.activeTab]}
  </span>

  <div class="flex-1" style={dragRegion}></div>

  <div class="relative flex items-center mr-1" style={noDragRegion}>
    {#if store.isAuthenticated && store.activeAccount}
      {@const activeAccount = store.activeAccount}
      <button
        data-account-button
        onclick={() => setShowAccountDropdown(!store.showAccountDropdown)}
        class="flex items-center gap-1.5 px-2 h-7 rounded text-sm font-medium text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] transition-all cursor-pointer"
      >
        <img
          src="https://avatar.mcindex.net/avatar/{activeAccount.username}/16"
          alt={activeAccount.username}
          class="w-4 h-4 rounded object-cover flex-shrink-0"
          style="image-rendering: pixelated"
        />
        <span>{activeAccount.username}</span>
        <ChevronDown size={14} strokeWidth={3} class="transition-transform {store.showAccountDropdown ? 'rotate-180' : ''}" />
      </button>
      {#if store.showAccountDropdown}
        <div
          data-account-dropdown
          class="absolute top-full mt-1 w-48 bg-[var(--bg-tertiary)] rounded shadow-lg overflow-hidden z-50 left-1/2 -translate-x-1/2"
        >
          <div>
            {#each store.accounts as acc}
              <button
                onclick={async () => {
                  if (!acc.is_active) {
                    try { await invoke("switch_account", { uuid: acc.uuid }); await loadAccounts() } catch {}
                  }
                  setShowAccountDropdown(false)
                }}
                class="w-full flex items-center gap-2.5 px-3 py-2 text-sm text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-hover)] transition-colors cursor-pointer"
              >
                <img
                  src="https://avatar.mcindex.net/avatar/{acc.username}/24"
                  alt={acc.username}
                  class="w-6 h-6 rounded object-cover flex-shrink-0"
                  style="image-rendering: pixelated"
                />
                <span class="flex-1 text-left">{acc.username}</span>
                {#if acc.is_active}
                  <Check size={14} strokeWidth={3} class="text-[#16a34a]" />
                {/if}
              </button>
            {/each}
          </div>
          <div class="border-t border-[var(--border-default)]"></div>
          <div>
            <button
              onclick={async () => {
                try { await invoke("microsoft_login_and_store"); await loadAccounts(); setShowAccountDropdown(false) } catch {}
              }}
              class="w-full flex items-center gap-2.5 px-3 py-2 text-sm text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-hover)] transition-colors cursor-pointer"
            >
              <LogIn size={16} strokeWidth={3} class="text-[#16a34a]" />
              Add Account
            </button>
            <button
              onclick={async () => {
                try { await invoke("remove_account", { uuid: activeAccount.uuid }); await loadAccounts(); setShowAccountDropdown(false) } catch {}
              }}
              class="w-full flex items-center gap-2.5 px-3 py-2 text-sm text-red-400 hover:bg-[var(--bg-hover)] transition-colors cursor-pointer"
            >
              <LogOut size={16} strokeWidth={3} />
              Sign Out
            </button>
          </div>
        </div>
      {/if}
    {:else}
      <button
        onclick={async () => {
          try { await invoke("microsoft_login_and_store"); await loadAccounts() } catch {}
        }}
        class="flex items-center gap-1.5 px-2 h-6 rounded text-xs font-medium text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] transition-all cursor-pointer"
      >
        <LogIn size={14} strokeWidth={2} class="text-[#16a34a]" />
        Sign in
      </button>
    {/if}
  </div>

  <div class="flex items-center" style={noDragRegion}>
    <button onclick={() => appWindow.minimize()} class="h-9 w-9 flex items-center justify-center text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors cursor-pointer">
      <Minus size={18} strokeWidth={3} />
    </button>
    <button onclick={() => appWindow.toggleMaximize()} class="h-9 w-9 flex items-center justify-center text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors cursor-pointer">
      <Square size={14} strokeWidth={3} />
    </button>
    <button onclick={() => appWindow.close()} class="h-9 w-9 flex items-center justify-center text-[var(--text-secondary)] hover:text-red-500 transition-colors cursor-pointer">
      <X size={18} strokeWidth={3} />
    </button>
  </div>
</div>
