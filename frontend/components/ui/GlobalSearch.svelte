<script lang="ts">
  import { fade, fly } from "svelte/transition"
  import {
    House, LibraryBig, Package, Blocks, Shirt, Image, Images, SquareTerminal, Terminal,
    Settings, Plus, Cpu, Coffee, Palette, HardDrive, FolderOpen, User,
    Search,
  } from "lucide-svelte"
  import { invoke } from "@tauri-apps/api/core"
  import {
    store, setActiveTab, setShowInstanceDetails, setSelectedInstance,
    setShowSettingsModal, setShowCreateModal, loadAccounts,
    setAddonsSubTab, setShowSearchPalette,
  } from "../../lib/launcherStore.svelte"
  import { getMinecraftVersion } from "../../lib/version"

  interface Entry {
    id: string
    label: string
    hint?: string
    icon: typeof House
    keywords?: string
    disabled?: boolean
    action: () => void
  }

  let query = $state("")
  let selected = $state(0)
  let inputEl = $state<HTMLInputElement>()

  const SETTINGS_SECTIONS = [
    { id: "memory", label: "Memory Allocation", icon: Cpu },
    { id: "java", label: "Java", icon: Coffee },
    { id: "appearance", label: "Appearance", icon: Palette },
    { id: "background", label: "Background", icon: Image },
    { id: "console", label: "Console Settings", icon: Terminal },
    { id: "directory", label: "Game Directory", icon: FolderOpen },
    { id: "storage", label: "Storage", icon: HardDrive },
  ]

  function close() {
    setShowSearchPalette(false)
  }

  function goTab(tab: Parameters<typeof setActiveTab>[0], label: string, icon: typeof House, extraKeywords?: string) {
    return {
      id: `tab-${tab}`,
      label,
      icon,
      keywords: `open go navigate${extraKeywords ? ` ${extraKeywords}` : ""}`,
      action: () => {
        setActiveTab(tab)
        setShowInstanceDetails(false)
      },
    }
  }

  const entries = $derived.by<Entry[]>(() => {
    const list: Entry[] = [
      goTab("home", "Home", House),
      goTab("instances", "Instances", LibraryBig),
      goTab("addons", "Addons", Blocks, "mods modpacks browse modrinth curseforge search"),
      goTab("skins", "Skins", Shirt),
      goTab("screenshots", "Screenshots", Images),
      goTab("servers", "Servers", HardDrive),
      goTab("console", "Console", SquareTerminal),
      {
        id: "action-new-instance",
        label: "Create New Instance",
        icon: Plus,
        keywords: "new add instance create",
        action: () => setShowCreateModal(true),
      },
      {
        id: "settings-root",
        label: "Settings",
        hint: "Open settings",
        icon: Settings,
        keywords: "options preferences",
        action: () => setShowSettingsModal(true),
      },
    ]

    for (const section of SETTINGS_SECTIONS) {
      list.push({
        id: `settings-${section.id}`,
        label: section.label,
        hint: "Settings",
        icon: section.icon,
        keywords: "settings option preference",
        action: () => {
          store.settingsScrollTarget = section.id
          setShowSettingsModal(true)
        },
      })
    }

    for (const tab of ["mods", "modpacks", "resourcepacks", "shaderpacks"] as const) {
      if (!query.trim()) continue
      list.push({
        id: `addons-${tab}`,
        label: `Search ${tab} for "${query.trim()}"`,
        hint: "Addons",
        icon: Blocks,
        action: () => {
          store.pendingAddonsSearch = query.trim()
          setAddonsSubTab(tab)
          setActiveTab("addons")
          setShowInstanceDetails(false)
        },
      })
    }

    for (const instance of store.instances) {
      list.push({
        id: `instance-${instance.name}`,
        label: instance.name,
        hint: `${getMinecraftVersion(instance)} · ${instance.loader || "vanilla"}`,
        icon: Package,
        keywords: "instance open play launch",
        action: () => {
          setSelectedInstance(instance)
          setActiveTab("instances")
          setShowInstanceDetails(true)
        },
      })
    }

    if (store.isAuthenticated && store.accounts.length > 0) {
      list.push({
        id: "accounts-header",
        label: "Switch account",
        icon: User,
        keywords: "account user switch sign in",
        disabled: true,
        action: () => {},
      })

      for (const account of store.accounts) {
        list.push({
          id: `account-${account.uuid}`,
          label: account.username,
          hint: account.is_active ? "Active" : "Switch to this account",
          icon: User,
          keywords: "account user switch",
          action: () => {
            invoke("switch_account", { uuid: account.uuid })
              .then(() => loadAccounts())
              .catch(() => {})
          },
        })
      }
    }

    const q = query.trim().toLowerCase()
    if (!q) return list

    return list
      .filter(e =>
        e.label.toLowerCase().includes(q) ||
        (e.keywords ?? "").toLowerCase().includes(q) ||
        (e.hint ?? "").toLowerCase().includes(q)
      )
      .sort((a, b) => {
        const aStarts = a.label.toLowerCase().startsWith(q) ? 0 : 1
        const bStarts = b.label.toLowerCase().startsWith(q) ? 0 : 1
        return aStarts - bStarts
      })
  })

  $effect(() => {
    query;
    store.showSearchPalette;
    selected = 0
  })

  $effect(() => {
    inputEl?.focus()
  })

  function run(entry: Entry) {
    entry.action()
    query = ""
    close()
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault()
      close()
    } else if (e.key === "ArrowDown") {
      e.preventDefault()
      selected = Math.min(selected + 1, entries.length - 1)
    } else if (e.key === "ArrowUp") {
      e.preventDefault()
      selected = Math.max(selected - 1, 0)
    } else if (e.key === "Enter") {
      e.preventDefault()
      const entry = entries[selected]
      if (entry && !entry.disabled) run(entry)
    }
  }
</script>

{#if store.showSearchPalette}
  <div
    class="fixed inset-0 z-[100] bg-black/70 flex items-start justify-center pt-[14vh]"
    transition:fade={{ duration: 120 }}
    onclick={close}
    onkeydown={() => {}}
    role="presentation"
  >
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div
      class="w-full max-w-lg bg-[var(--bg-primary)] border border-[var(--border-subtle)] rounded-xl overflow-hidden shadow-2xl"
      transition:fly={{ y: -8, duration: 150 }}
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      aria-label="Global search"
    >
      <div class="flex items-center gap-3 px-4 py-3.5">
        <Search size={15} class="text-[var(--text-muted)] flex-shrink-0" />
        <input
          bind:this={inputEl}
          bind:value={query}
          onkeydown={onKeydown}
          placeholder="Search Octane..."
          class="flex-1 bg-transparent text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none"
        />
        <kbd class="text-[10px] text-[var(--text-muted)] bg-[var(--bg-tertiary)] px-1.5 py-0.5 rounded">ESC</kbd>
      </div>

      <div class="border-t border-[var(--border-subtle)] max-h-[50vh] overflow-y-auto p-1.5">
        {#if entries.length === 0}
          <p class="py-8 text-center text-sm text-[var(--text-muted)]">No results</p>
        {:else}
          {#each entries as entry, i (entry.id)}
            {@const Icon = entry.icon}
            <button
              onclick={() => run(entry)}
              onmousemove={() => (selected = i)}
              class="w-full flex items-center gap-3 px-3 py-2 rounded-md text-left transition-colors {selected === i ? 'bg-[var(--bg-secondary)]' : ''} {entry.disabled ? 'opacity-40 pointer-events-none' : 'cursor-pointer'}"
            >
              <Icon size={15} class="text-[var(--text-muted)] flex-shrink-0" />
              <span class="text-sm text-[var(--text-primary)] truncate flex-1">{entry.label}</span>
              {#if entry.hint}
                <span class="text-xs text-[var(--text-muted)]">{entry.hint}</span>
              {/if}
            </button>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}