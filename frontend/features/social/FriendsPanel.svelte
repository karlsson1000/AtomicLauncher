<script lang="ts">
  import { invoke } from "@tauri-apps/api/core"
  import { Users, UserPlus, UserCheck, UserX, Search, Loader2, LogIn, ChevronDown, ChevronRight, Inbox } from "lucide-svelte"
  import type { Friend, FriendRequest } from "../../types"

  let { isOpen, isAuthenticated, activeAccountUuid }: { isOpen: boolean, isAuthenticated: boolean, activeAccountUuid?: string } = $props()

  let friends = $state<Friend[]>([])
  let requests = $state<FriendRequest[]>([])
  let isLoading = $state(false)
  let sending = $state(false)
  let sendError = $state<string | null>(null)
  let searchQuery = $state("")

  $effect(() => {
    if (isOpen && isAuthenticated) {
      loadFriends()
      loadRequests()
      const interval = setInterval(pollFriends, 30000)
      return () => clearInterval(interval)
    }
    void activeAccountUuid
  })

  const loadFriends = async () => {
    isLoading = true
    try {
      const result = await invoke<Friend[]>("get_friends")
      friends = result
    } catch (error) {
      console.error("Failed to load friends:", error)
    } finally {
      isLoading = false
    }
  }

  const pollFriends = async () => {
    try {
      const result = await invoke<Friend[]>("get_friends")
      friends = result
    } catch (error) {
      console.error("Failed to poll friends:", error)
    }
  }

  const loadRequests = async () => {
    try {
      const result = await invoke<FriendRequest[]>("get_friend_requests")
      requests = result
    } catch (error) {
      console.error("Failed to load requests:", error)
    }
  }

  const handleSendRequest = async (username: string) => {
    if (!username.trim()) return
    sending = true
    sendError = null
    try {
      await invoke("send_friend_request", { username: username.trim() })
      searchQuery = ""
      loadRequests()
    } catch (error) {
      sendError = String(error)
    } finally {
      sending = false
    }
  }

  const handleAcceptRequest = async (requestId: string) => {
    try {
      await invoke("accept_friend_request", { requestId })
      requests = requests.filter(r => r.id !== requestId)
      loadFriends()
    } catch (error) {
      console.error("Failed to accept request:", error)
    }
  }

  const handleRejectRequest = async (requestId: string) => {
    try {
      await invoke("reject_friend_request", { requestId })
      requests = requests.filter(r => r.id !== requestId)
    } catch (error) {
      console.error("Failed to reject request:", error)
    }
  }

  const handleRemoveFriend = async (friendUuid: string) => {
    try {
      await invoke("remove_friend", { friendUuid })
      friends = friends.filter(f => f.uuid !== friendUuid)
    } catch (error) {
      console.error("Failed to remove friend:", error)
    }
  }

  let filteredFriends = $derived(
    friends.filter(f => f.username.toLowerCase().includes(searchQuery.toLowerCase()))
  )

  let sortedFriends = $derived(
    [...filteredFriends].sort((a, b) => {
      const order: Record<string, number> = { ingame: 0, online: 1, offline: 2 }
      return (order[a.status] ?? 2) - (order[b.status] ?? 2)
    })
  )
  let onlineFriends = $derived(
    sortedFriends.filter(f => f.status === "online" || f.status === "ingame")
  )

  let offlineFriends = $derived(
    sortedFriends.filter(f => f.status === "offline")
  )

  let onlineCollapsed = $state(false)
  let offlineCollapsed = $state(false)
  let showSearch = $state(false)
  let showRequests = $state(false)

</script>

{#snippet friendRow(friend: Friend)}
  <div class="group flex items-center gap-3 px-1 py-1 relative">
    <div class="relative flex-shrink-0">
      <img
        src="https://avatar.mcindex.net/avatar/{friend.username}/32"
        alt={friend.username}
        class="w-8 h-8 rounded object-cover"
        loading="lazy"
        decoding="async"
      />
      <div class="absolute -bottom-0.5 -right-0.5">
        {#if friend.status === "online"}
          <span title="Online"><div class="w-2 h-2 rounded-full bg-[#16a34a] ring-2 ring-[var(--bg-primary)]"></div></span>
        {:else if friend.status === "ingame"}
          <span title="In Game"><div class="w-2 h-2 rounded-full bg-[#3b82f6] ring-2 ring-[var(--bg-primary)]"></div></span>
        {:else}
          <span title="Offline"><div class="w-2 h-2 rounded-full bg-[var(--bg-hover-strong)] ring-2 ring-[var(--bg-primary)]"></div></span>
        {/if}
      </div>
    </div>
    <div class="flex-1 min-w-0">
      <div class="text-base text-[var(--text-primary)] truncate font-medium">{friend.username}</div>
      <div class="text-[13px] text-[var(--text-muted)] truncate -mt-0.75">
        {#if friend.status === "ingame" && friend.current_instance}
          Playing <span class="text-[#3b82f6] font-semibold">{friend.current_instance}</span>
        {:else if friend.status === "online"}
          In Launcher
        {:else}
          Offline
        {/if}
      </div>
    </div>
    <button
      onclick={() => handleRemoveFriend(friend.uuid)}
      class="opacity-0 group-hover:opacity-100 p-1 hover:bg-red-500/20 rounded text-[var(--text-muted)] hover:text-red-400 transition-all cursor-pointer absolute right-1"
      title="Remove friend"
    >
      <UserX size={16} strokeWidth={3} />
    </button>
  </div>
{/snippet}

<div
  class="flex-shrink-0 bg-[var(--bg-primary)] flex flex-col h-full overflow-hidden transition-all duration-200 ease-in-out {isOpen ? 'w-60' : 'w-0 -mr-4'}"
>
  <div class="flex items-center gap-2 px-1 pt-2 pb-1">
    <span class="text-xl font-semibold text-[var(--text-primary)]">Friends</span>
    <div class="flex-1"></div>
      <button
        onclick={() => { showSearch = !showSearch; if (showSearch) showRequests = false }}
        class="h-7 w-7 flex items-center justify-center rounded transition-colors cursor-pointer {showSearch ? 'bg-[var(--bg-tertiary)] text-[var(--text-primary)]' : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)]'}"
        title="Search or add friends"
      >
        <Search size={18} strokeWidth={2.5} />
      </button>
      <div class="relative">
        <button
          onclick={() => { showRequests = !showRequests; if (showRequests) showSearch = false }}
          class="h-7 w-7 flex items-center justify-center rounded transition-colors cursor-pointer {showRequests ? 'bg-[var(--bg-tertiary)] text-[var(--text-primary)]' : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)]'}"
          title="Pending requests"
        >
          <Inbox size={18} strokeWidth={2.5} />
        </button>
        {#if requests.length > 0}
          <span class="absolute -top-1 -right-1 min-w-4 h-4 px-1 flex items-center justify-center rounded-full bg-red-500 text-white text-[10px] font-semibold leading-none">{requests.length}</span>
        {/if}
      </div>
  </div>

  {#if !isAuthenticated}
    <div class="flex-1 flex flex-col items-center justify-center px-1 py-6 text-center">
      <LogIn size={32} class="text-[var(--text-muted)] mb-3" />
      <p class="text-sm text-[var(--text-muted)]">Sign in to see your friends</p>
    </div>
  {:else}
    <div class="flex-1 flex flex-col min-h-0">
      {#if showSearch}
        <div class="px-1 pt-1 pb-1 space-y-1">
          <div class="relative">
            <Search size={14} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--text-muted)]" />
            <input
              type="text"
              placeholder="Search or add friends..."
              bind:value={searchQuery}
              oninput={() => sendError = null}
              onkeydown={(e) => {
                if (e.key === "Enter" && searchQuery.trim() && !friends.some(f => f.username.toLowerCase() === searchQuery.trim().toLowerCase())) {
                  handleSendRequest(searchQuery.trim())
                }
              }}
              class="w-full bg-[var(--bg-secondary)] rounded pl-8 pr-2 py-1.5 text-xs text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none"
            />
          </div>

          {#if searchQuery.trim() && !friends.some(f => f.username.toLowerCase() === searchQuery.trim().toLowerCase())}
            <button
              onclick={() => handleSendRequest(searchQuery.trim())}
              disabled={sending}
              class="w-full flex items-center gap-2 px-2 py-1.5 rounded text-xs text-[var(--text-muted)] hover:bg-[var(--bg-tertiary)] hover:text-[var(--text-primary)] transition-colors cursor-pointer"
            >
              {#if sending}
                <Loader2 size={14} class="animate-spin" />
              {:else}
                <UserPlus size={14} strokeWidth={3} />
              {/if}
              Send friend request to "{searchQuery.trim()}"
            </button>
          {/if}
          {#if sendError}
            <p class="text-xs text-red-400 px-1">{sendError}</p>
          {/if}
        </div>
      {/if}

      {#if showRequests}
        <div>
          {#if requests.length > 0}
            <div class="max-h-40 overflow-y-auto">
              {#each requests as req (req.id)}
                <div class="flex items-center gap-2 px-1 py-2 transition-colors">
                  <img
                    src="https://avatar.mcindex.net/avatar/{req.from_username}/24"
                    alt={req.from_username}
                    class="w-6 h-6 rounded object-cover flex-shrink-0"
                  />
                  <div class="flex-1 min-w-0">
                    <div class="text-sm text-[var(--text-primary)] truncate">{req.from_username}</div>
                  </div>
                  <button
                    onclick={() => handleAcceptRequest(req.id)}
                    class="p-1 hover:bg-[#16a34a]/20 rounded text-[var(--text-muted)] hover:text-[#16a34a] transition-colors cursor-pointer"
                    title="Accept"
                  >
                    <UserCheck size={16} strokeWidth={3} />
                  </button>
                  <button
                    onclick={() => handleRejectRequest(req.id)}
                    class="p-1 hover:bg-red-500/20 rounded text-[var(--text-muted)] hover:text-red-400 transition-colors cursor-pointer"
                    title="Reject"
                  >
                    <UserX size={16} strokeWidth={3} />
                  </button>
                </div>
              {/each}
            </div>
          {:else}
            <p class="px-1 pt-2 pb-2 text-xs text-[var(--text-muted)]">No pending requests</p>
          {/if}
        </div>
      {/if}

      <div class="flex-1 overflow-y-auto">
        {#if isLoading}
          <div class="flex items-center justify-center py-8">
            <Loader2 size={20} class="animate-spin text-[#3b82f6]" />
          </div>
        {:else if sortedFriends.length === 0}
          <div class="flex flex-col items-center justify-center py-8 text-center px-1">
            <Users size={32} class="text-[var(--text-muted)] mb-3" />
            <p class="text-sm text-[var(--text-muted)]">No friends yet</p>
            <p class="text-xs text-[var(--text-muted)] mt-1">Send a friend request to get started</p>
          </div>
        {:else}
          {#if onlineFriends.length > 0}
            <button
              onclick={() => onlineCollapsed = !onlineCollapsed}
              class="flex items-center gap-1 px-1 pt-2 pb-1 text-[13px] font-medium text-[var(--text-secondary)] tracking-wider hover:text-[var(--text-primary)] transition-colors cursor-pointer w-full text-left"
            >
              {#if onlineCollapsed}
                <ChevronRight size={14} strokeWidth={3} />
              {:else}
                <ChevronDown size={14} strokeWidth={3} />
              {/if}
              Online ({onlineFriends.length})
            </button>
            {#if !onlineCollapsed}
              <div class="border-b border-[var(--bg-tertiary)]"></div>
              <div class="pt-1 pb-1">
                {#each onlineFriends as friend (friend.uuid)}
                  {@render friendRow(friend)}
                {/each}
              </div>
            {/if}
          {/if}

          {#if offlineFriends.length > 0}
            <button
              onclick={() => offlineCollapsed = !offlineCollapsed}
              class="flex items-center gap-1 px-1 pt-2 pb-1 text-[13px] font-medium text-[var(--text-secondary)] tracking-wider hover:text-[var(--text-primary)] transition-colors cursor-pointer w-full text-left"
            >
              {#if offlineCollapsed}
                <ChevronRight size={14} strokeWidth={3} />
              {:else}
                <ChevronDown size={14} strokeWidth={3} />
              {/if}
              Offline ({offlineFriends.length})
            </button>
            {#if !offlineCollapsed}
              <div class="border-b border-[var(--bg-tertiary)]"></div>
              <div class="pt-1 pb-1">
                {#each offlineFriends as friend (friend.uuid)}
                  {@render friendRow(friend)}
                {/each}
              </div>
            {/if}
          {/if}
        {/if}
      </div>
    </div>
  {/if}
</div>
