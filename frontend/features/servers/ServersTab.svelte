<script lang="ts">
  import { Server, Plus, Search, Trash2, Play, GripVertical } from "lucide-svelte"
  import { invoke } from "@tauri-apps/api/core"
  import CreateServerModal from "./CreateServerModal.svelte"
  import ConfirmModal from "../../components/ui/ConfirmModal.svelte"
  import type { ServerInfo, McSrvStatResponse } from "../../types"
  import { store } from "../../lib/launcherStore.svelte"
  import { sanitizeHtml } from "../../lib/sanitize"

  let servers: ServerInfo[] = $state([])
  let searchQuery = $state("")
  let showAddModal = $state(false)
  let serverToDelete = $state<string | null>(null)
  let launchingServer = $state<string | null>(null)
  let isAnyInstanceRunning = $derived(store.runningInstances.size > 0)

  $effect(() => { loadServers() })

  $effect(() => {
    const hasServers = servers.length > 0
    if (!hasServers) return
    const interval = setInterval(() => {
      servers.forEach(server => refreshServerStatus(server))
    }, 5 * 60 * 1000)
    return () => clearInterval(interval)
  })

  async function loadServers() {
    try {
      const serverList = await invoke<ServerInfo[]>("get_servers")
      servers = serverList
      serverList.forEach(server => refreshServerStatus(server))
    } catch (error) {
      console.error("Failed to load servers:", error)
      servers = []
    }
  }

  async function fetchServerStatus(address: string, port: number): Promise<ServerInfo | null> {
    try {
      const fullAddress = port === 25565 ? address : `${address}:${port}`
      const response = await fetch(`https://api.mcsrvstat.us/3/${fullAddress}`, {
        headers: { "User-Agent": "OctaneLauncher/1.0" },
      })

      if (!response.ok) throw new Error(`API returned ${response.status}`)

      const data: McSrvStatResponse = await response.json()

      let ping: number | undefined = undefined
      if (data.online) {
        try {
          ping = await invoke<number>("ping_server", { address, port })
        } catch (e) {
          console.error("Failed to ping server:", e)
        }
      }

      return {
        name: address,
        address,
        port,
        status: data.online ? "online" : "offline",
        players_online: data.players?.online,
        players_max: data.players?.max,
        players_list: data.players?.list,
        version: data.version || data.protocol?.name,
        motd: data.motd?.clean?.join("\n"),
        motd_html: data.motd?.html?.map(sanitizeHtml),
        favicon: data.icon ? `https://api.mcsrvstat.us/icon/${fullAddress}` : undefined,
        last_checked: Date.now(),
        ping,
      }
    } catch (error) {
      console.error("Failed to fetch server status:", error)
      return null
    }
  }

  async function refreshServerStatus(server: ServerInfo) {
    try {
      const statusData = await fetchServerStatus(server.address, server.port)
      if (statusData) {
        const updated = { ...statusData, name: server.name }
        servers = servers.map(s => s.name === server.name ? updated : s)
        try {
          await invoke("update_server_status", { serverName: server.name, status: statusData })
        } catch (e) { console.error("Failed to save server status:", e) }
      }
    } catch (error) {
      console.error("Failed to refresh server:", error)
    }
  }

  async function handleDeleteServer() {
    if (!serverToDelete) return
    try {
      await invoke("delete_server", { serverName: serverToDelete })
      await loadServers()
    } catch (error) {
      console.error("Delete error:", error)
    } finally {
      serverToDelete = null
    }
  }

  async function handleServerAdded(newServer: ServerInfo) {
    try {
      await invoke("add_server", { name: newServer.name, address: newServer.address, port: newServer.port })
    } catch (error) {
      console.error("Failed to save server:", error)
      alert(`Failed to add server: ${error}`)
      return
    }

    const statusData = await fetchServerStatus(newServer.address, newServer.port)

    if (statusData) {
      const serverWithStatus = { ...statusData, name: newServer.name }
      servers = [...servers, serverWithStatus]
      try {
        await invoke("update_server_status", { serverName: newServer.name, status: statusData })
      } catch (e) { console.error("Failed to save server status:", e) }
    } else {
      servers = [...servers, { ...newServer, status: "unknown" }]
    }
  }

  let filteredServers = $derived(
    servers.filter(server =>
      server.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      server.address.toLowerCase().includes(searchQuery.toLowerCase())
    )
  )

  let dragState: {
    fromIndex: number
    draggedOverIndex: number | null
    ghostEl: HTMLElement | null
    origCardEl: HTMLElement | null
    offsetY: number
    initialLeft: number
    minY: number
    maxY: number
  } | null = null

  let draggedOverIndex: number | null = $state(null)

  let pointerMoveImpl = (_e: PointerEvent) => {}
  let pointerUpImpl = (_e: PointerEvent) => {}
  let pointerCancelImpl = (_e: PointerEvent) => {}

  function stablePointerMove(e: PointerEvent) { pointerMoveImpl(e) }
  function stablePointerUp(e: PointerEvent) { pointerUpImpl(e) }
  function stablePointerCancel(e: PointerEvent) { pointerCancelImpl(e) }

  function cleanupDrag() {
    document.removeEventListener("pointermove", stablePointerMove)
    document.removeEventListener("pointerup", stablePointerUp)
    document.removeEventListener("pointercancel", stablePointerCancel)
    if (dragState) {
      if (dragState.ghostEl && dragState.ghostEl.parentNode) {
        dragState.ghostEl.parentNode.removeChild(dragState.ghostEl)
      }
      if (dragState.origCardEl) {
        dragState.origCardEl.style.opacity = ""
      }
    }
    dragState = null
    draggedOverIndex = null
  }

  pointerMoveImpl = (e: PointerEvent) => {
    const state = dragState
    if (!state || !state.ghostEl) return

    state.ghostEl.style.left = `${state.initialLeft}px`
    const clampedY = Math.max(state.minY, Math.min(e.clientY - state.offsetY, state.maxY))
    state.ghostEl.style.top = `${clampedY}px`

    const els = document.elementsFromPoint(e.clientX, e.clientY)
    for (const el of els) {
      if (el === state.ghostEl) continue
      const card = (el as HTMLElement).closest("[data-server-card]") as HTMLElement | null
      if (card && card !== state.ghostEl) {
        const idx = Number(card.dataset.serverIndex)
        if (!isNaN(idx) && idx !== state.draggedOverIndex) {
          state.draggedOverIndex = idx
          draggedOverIndex = idx
        }
        return
      }
    }
    if (state.draggedOverIndex !== null) {
      state.draggedOverIndex = null
      draggedOverIndex = null
    }
  }

  pointerUpImpl = (_e: PointerEvent) => {
    const state = dragState
    if (!state) return
    const fromIdx = state.fromIndex
    const toIdx = state.draggedOverIndex
    cleanupDrag()
    if (toIdx === null || fromIdx === toIdx) return

    const list = filteredServers
    const reordered = list.slice()
    const [moved] = reordered.splice(fromIdx, 1)
    reordered.splice(toIdx, 0, moved)

    if (searchQuery) {
      const movedNames = new Set(reordered.map(s => s.name))
      const remaining = servers.filter(s => !movedNames.has(s.name))
      persistReorder([...reordered, ...remaining])
    } else {
      persistReorder(reordered)
    }
  }

  pointerCancelImpl = (_e: PointerEvent) => {
    cleanupDrag()
  }

  async function persistReorder(ordered: ServerInfo[]) {
    servers = ordered
    try {
      await invoke("reorder_servers", { serverNames: ordered.map(s => s.name) })
    } catch (e) {
      console.error("Failed to reorder servers:", e)
    }
  }

  function handleGripPointerDown(e: PointerEvent, index: number) {
    if (searchQuery.trim()) return
    e.preventDefault()
    e.stopPropagation()

    const gripEl = e.currentTarget as HTMLElement
    const cardEl = gripEl.closest("[data-server-card]") as HTMLElement | null
    if (!cardEl) return

    const rect = cardEl.getBoundingClientRect()
    const offsetY = e.clientY - rect.top
    const initialLeft = rect.left

    const listEl = cardEl.parentElement
    let minY = -Infinity
    let maxY = Infinity
    if (listEl) {
      const listRect = listEl.getBoundingClientRect()
      minY = listRect.top
      maxY = listRect.bottom - rect.height
    }

    const ghost = cardEl.cloneNode(true) as HTMLElement
    ghost.removeAttribute("data-server-card")
    ghost.dataset.serverIndex = "-1"
    ghost.style.position = "fixed"
    ghost.style.left = `${initialLeft}px`
    ghost.style.top = `${e.clientY - offsetY}px`
    ghost.style.width = `${rect.width}px`
    ghost.style.pointerEvents = "none"
    ghost.style.zIndex = "9999"
    ghost.style.transform = "scale(1.03)"
    ghost.style.opacity = "0.95"
    ghost.style.transition = "none"
    ghost.style.outline = "none"
    ghost.style.willChange = "transform, left, top"
    document.body.appendChild(ghost)

    cardEl.style.opacity = "0.3"

    dragState = {
      fromIndex: index,
      draggedOverIndex: index,
      ghostEl: ghost,
      origCardEl: cardEl,
      offsetY,
      initialLeft,
      minY,
      maxY,
    }
    draggedOverIndex = index
    document.addEventListener("pointermove", stablePointerMove)
    document.addEventListener("pointerup", stablePointerUp)
    document.addEventListener("pointercancel", stablePointerCancel)
  }

  async function handleLaunchServer(server: ServerInfo, e: MouseEvent) {
    e.stopPropagation()
    if (server.status !== "online") {
      alert("Server must be online to connect")
      return
    }
    launchingServer = server.name
    try {
      await invoke("launch_server", {
        serverAddress: server.address,
        serverPort: server.port,
        serverName: server.name,
      })
      setTimeout(() => { launchingServer = null }, 1000)
    } catch (error) {
      console.error("Failed to launch server:", error)
      alert(`Failed to launch server: ${error}`)
      launchingServer = null
    }
  }
</script>

<div class="p-8 space-y-4">
  <div class="max-w-7xl mx-auto">
    <div class="flex items-center justify-between mb-4">
      <div class="flex items-center gap-2">
        {#if servers.length > 0}
          <div class="relative rounded-md bg-[var(--bg-tertiary)]">
            <Search size={16} class="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--text-muted)] z-20 pointer-events-none" strokeWidth={2} />
            <input
              type="text"
              placeholder="Search servers..."
              bind:value={searchQuery}
              class="w-72 bg-transparent rounded-md pl-9 pr-3 py-1.5 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none transition-all relative z-10"
            />
          </div>
        {/if}
      </div>
      <div class="flex items-center gap-2">
        <button
          onclick={() => showAddModal = true}
          class="px-4 h-8 bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white rounded-md text-sm font-medium flex items-center gap-2 transition-colors cursor-pointer"
        >
          <Plus size={16} />
          Add Server
        </button>
      </div>
    </div>

    {#if servers.length === 0}
      <div class="flex flex-col items-center justify-center py-14">
        <Server size={48} class="text-[var(--text-muted)] mb-3" strokeWidth={1.5} />
        <h3 class="text-base font-semibold text-[var(--text-primary)] mb-1">No servers added</h3>
        <p class="text-sm text-[var(--text-muted)] mb-4">Add your first server to get started</p>
        <button
          onclick={() => showAddModal = true}
          class="px-4 py-2 bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white rounded font-medium text-sm flex items-center gap-2 transition-all cursor-pointer"
        >
          <Plus size={16} strokeWidth={2} />
          <span>Add Server</span>
        </button>
      </div>
    {:else if filteredServers.length === 0}
      <div class="rounded-md p-8 flex flex-col items-center justify-center">
        <Search size={48} class="text-[var(--text-primary)] mb-3" strokeWidth={1.5} />
        <h3 class="text-base font-semibold text-[var(--text-primary)] mb-1">No servers found</h3>
        <p class="text-sm text-[var(--text-muted)]">Try adjusting your search query</p>
      </div>
    {:else}
      <div class="space-y-4">
        {#each filteredServers as server, index (server.name)}
          {@const displayAddress = server.port === 25565 ? server.address : `${server.address}:${server.port}`}
          {@const isLaunching = launchingServer === server.name}
          {@const isOnline = server.status === "online"}
          {@const isOver = draggedOverIndex === index}
          <div
            data-server-card
            data-server-index={index}
            class="group flex items-center gap-3 px-4 rounded-md bg-[var(--bg-tertiary)] hover:bg-[var(--bg-hover)] {isOver ? 'outline outline-2 outline-[var(--accent-primary)]' : 'transition-colors'}"
            style="height: 100px"
          >
            <div
              role="presentation"
              onpointerdown={e => handleGripPointerDown(e, index)}
              class="flex-shrink-0 cursor-grab active:cursor-grabbing touch-none text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors"
            >
              <GripVertical size={18} strokeWidth={2} />
            </div>
            <div class="flex-shrink-0">
              {#if server.favicon}
                <img
                  src={server.favicon}
                  alt={server.name}
                  class="w-20 h-20 rounded object-cover"
                  style="image-rendering: pixelated"
                />
              {:else}
                <div class="w-20 h-20 bg-[var(--bg-secondary)] rounded flex items-center justify-center">
                  <Server size={40} class="text-[var(--text-muted)]" strokeWidth={2} />
                </div>
              {/if}
            </div>

            <div class="flex-1 min-w-0">
              <p class="font-semibold text-xl text-[var(--text-primary)] truncate leading-none mb-2">{server.name}</p>
              {#if server.motd_html}
                <p class="text-base truncate leading-snug">{@html server.motd_html[0]}</p>
                {#if server.motd_html[1]}
                  <p class="text-base truncate leading-snug">{@html server.motd_html[1]}</p>
                {/if}
              {:else if server.motd}
                <p class="text-base text-[var(--text-muted)] truncate leading-snug">{server.motd.split("\n")[0]}</p>
                {#if server.motd.includes("\n")}
                  <p class="text-base text-[var(--text-muted)] truncate leading-snug">{server.motd.split("\n")[1]}</p>
                {/if}
              {:else}
                <p class="text-base text-[var(--text-muted)] truncate">{displayAddress}</p>
              {/if}
            </div>

            <div class="flex-shrink-0 flex flex-col items-end gap-2 group-hover:hidden">
              {#if isOnline}
                {#if server.ping === undefined}
                  <span class="text-base font-medium text-[var(--text-muted)]">N/A</span>
                {:else}
                  <span class="text-base font-medium" style="color: {server.ping < 80 ? '#16a34a' : server.ping < 200 ? '#eab308' : '#e93636'}">{server.ping}ms</span>
                {/if}
              {/if}
              {#if isOnline && server.players_online !== undefined}
                <span class="text-base text-[var(--text-muted)]">
                  {server.players_online}/{server.players_max}
                </span>
              {:else}
                <span class="text-base font-medium {server.status === 'offline' ? 'text-[#ef4444]' : 'text-[var(--text-muted)]'}">
                  {server.status === "offline" ? "Offline" : "-"}
                </span>
              {/if}
            </div>

            <div class="flex-shrink-0 items-center gap-2 hidden group-hover:flex">
              <button
                onclick={e => { e.stopPropagation(); serverToDelete = server.name }}
                class="w-10 h-10 flex items-center justify-center rounded text-[var(--text-muted)] hover:text-[#ef4444] hover:bg-[#ef4444]/10 transition-colors cursor-pointer"
              >
                <Trash2 size={18} strokeWidth={2} />
              </button>

              <button
                onclick={e => handleLaunchServer(server, e)}
                disabled={!isOnline || isLaunching || isAnyInstanceRunning}
                title={!isOnline ? "Server offline" : isAnyInstanceRunning ? "Instance already running" : undefined}
                class="flex-shrink-0 h-10 px-8 flex items-center justify-center gap-3 rounded transition-all active:scale-95 cursor-pointer {isLaunching || isAnyInstanceRunning ? 'bg-red-500/10 text-red-400 cursor-not-allowed' : isOnline ? 'bg-[#16a34a] hover:bg-[#15803d] text-[#181a1f]' : 'bg-[var(--bg-hover-strong)] text-[var(--text-muted)] cursor-not-allowed'} disabled:opacity-50"
              >
                {#if isLaunching || isAnyInstanceRunning}
                  <div class="w-5 h-5 border-2 border-red-400/30 border-t-red-400 rounded-full animate-spin"></div>
                {:else}
                  <Play size={18} fill="currentColor" strokeWidth={0} />
                  <span class="text-base font-semibold">Join</span>
                {/if}
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

{#if showAddModal}
  <CreateServerModal
    {servers}
    onClose={() => showAddModal = false}
    onSuccess={handleServerAdded}
  />
{/if}

<ConfirmModal
  isOpen={serverToDelete !== null}
  title="Remove Server"
  message={`Are you sure you want to remove "${serverToDelete}"? This action cannot be undone.`}
  confirmText="Remove"
  cancelText="Cancel"
  type="danger"
  onConfirm={handleDeleteServer}
  onCancel={() => serverToDelete = null}
/>
