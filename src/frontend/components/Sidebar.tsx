import { useState, useRef, useEffect } from "react"
import { invoke } from "@tauri-apps/api/core"
import { LogIn, LogOut, ChevronUp, ChevronDown, Play, Package, FolderOpen, Copy, Trash2, UserPlus, Gamepad2, Bell, Check, X } from "lucide-react"
import { ContextMenu } from "../modals/ContextMenu"
import { ConfirmModal } from "../modals/ConfirmModal"
import type { Instance } from "../../types"

interface AccountInfo {
  uuid: string
  username: string
  is_active: boolean
  added_at: string
  last_used: string | null
}

interface Friend {
  uuid: string
  username: string
  status: "online" | "offline" | "ingame"
  last_seen: string
  current_instance?: string
}

interface FriendRequest {
  id: string
  from_uuid: string
  from_username: string
  to_uuid: string
  status: "pending"
  created_at: string
}

interface SidebarProps {
  activeTab: "home" | "instances" | "browse" | "console" | "servers" | "skins" | "map"
  setActiveTab: (tab: "home" | "instances" | "browse" | "console" | "servers" | "skins" | "map") => void
  showInstanceDetails: boolean
  setShowInstanceDetails: (show: boolean) => void
  instances: Instance[]
  instanceIcons: Record<string, string | null>
  runningInstances: Set<string>
  launchingInstanceName: string | null
  isAuthenticated: boolean
  activeAccount: AccountInfo | null
  accounts: AccountInfo[]
  sidebarBackground: string | null
  sidebarContextMenu: { x: number; y: number; instance: Instance } | null
  setSidebarContextMenu: (menu: { x: number; y: number; instance: Instance } | null) => void
  setSelectedInstance: (instance: Instance) => void
  setShowSettingsModal: (show: boolean) => void
  onQuickLaunch: (instance: Instance) => void
  onKillInstance: (instance: Instance) => void
  onOpenInstanceFolder: (instance: Instance) => void
  onDuplicateInstance: (instance: Instance) => void
  onDeleteInstance: (instanceName: string) => void
  loadAccounts: () => Promise<void>
}

export function Sidebar(props: SidebarProps) {
  const {
    instances,
    instanceIcons,
    runningInstances,
    launchingInstanceName,
    isAuthenticated,
    activeAccount,
    accounts,
    sidebarBackground,
    sidebarContextMenu,
    setSidebarContextMenu,
    setSelectedInstance,
    onQuickLaunch,
    onKillInstance,
    onOpenInstanceFolder,
    onDuplicateInstance,
    onDeleteInstance,
    loadAccounts,
    setActiveTab,
    setShowInstanceDetails,
  } = props

  const [isLoggingIn, setIsLoggingIn] = useState(false)
  const [showAccountDropdown, setShowAccountDropdown] = useState(false)
  const [showAddFriend, setShowAddFriend] = useState(false)
  const [friendUsername, setFriendUsername] = useState("")
  const [dropdownPosition, setDropdownPosition] = useState({ top: 0, left: 0, width: 0 })
  const [friends, setFriends] = useState<Friend[]>([])
  const [friendRequests, setFriendRequests] = useState<FriendRequest[]>([])
  const [showRequests, setShowRequests] = useState(false)
  const [sendingRequest, setSendingRequest] = useState(false)
  const [requestStatus, setRequestStatus] = useState<{ type: 'success' | 'error', message: string } | null>(null)
  const [confirmRemoveFriend, setConfirmRemoveFriend] = useState<{ uuid: string, username: string } | null>(null)
  const [processingRequestId, setProcessingRequestId] = useState<string | null>(null)
  const [friendsUpdateKey, setFriendsUpdateKey] = useState(0)
  const accountButtonRef = useRef<HTMLButtonElement>(null)

  useEffect(() => {
    if (showAccountDropdown && accountButtonRef.current) {
      const rect = accountButtonRef.current.getBoundingClientRect()
      setDropdownPosition({
        top: rect.top,
        left: rect.left,
        width: rect.width
      })
    }
  }, [showAccountDropdown])

  // Load friends and requests when authenticated
  useEffect(() => {
    if (isAuthenticated && activeAccount) {
      const registerAndLoad = async () => {
        try {
          console.log("Registering user in friends system:", activeAccount.username)
          await invoke("register_user_in_friends_system")
          console.log("User registered successfully")
          
          // Load friends and requests
          await loadFriends()
          await loadFriendRequests()
        } catch (err) {
          console.error("Failed to initialize friends system:", err)
        }
      }
      
      registerAndLoad()

      // Poll for updates every 5 seconds
      const interval = setInterval(() => {
        loadFriends()
        loadFriendRequests()
      }, 5000)

      return () => {
        clearInterval(interval)
      }
    }
  }, [isAuthenticated, activeAccount])

  const loadFriends = async () => {
    try {
      const friendsList = await invoke<Friend[]>("get_friends")
      console.log("Loaded friends with status:", friendsList.map(f => ({
        username: f.username,
        status: f.status,
        server: f.current_instance,
        statusText: getStatusText(f)
      })))
      setFriends([...friendsList])
      setFriendsUpdateKey(prev => prev + 1)
    } catch (error) {
      console.error("Failed to load friends:", error)
    }
  }

  const loadFriendRequests = async () => {
    try {
      const requests = await invoke<FriendRequest[]>("get_friend_requests")
      setFriendRequests(requests)
    } catch (error) {
      console.error("Failed to load friend requests:", error)
    }
  }

  const handleSendFriendRequest = async () => {
    if (!friendUsername.trim()) return
    
    // Check if user is trying to add themselves
    if (activeAccount && friendUsername.trim().toLowerCase() === activeAccount.username.toLowerCase()) {
      setRequestStatus({ type: 'error', message: "Can't add yourself" })
      setTimeout(() => setRequestStatus(null), 3000)
      return
    }
    
    setSendingRequest(true)
    setRequestStatus(null)
    try {
      await invoke("send_friend_request", { username: friendUsername.trim() })
      setRequestStatus({ type: 'success', message: 'Request sent!' })
      setFriendUsername("")
      setTimeout(() => {
        setShowAddFriend(false)
        setRequestStatus(null)
      }, 2000)
    } catch (error) {
      const errorMsg = String(error)
      // Clean up the error message for better display
      let displayMsg = errorMsg
      if (errorMsg.includes("Friend request already sent")) {
        displayMsg = "Already sent friend request"
      } else if (errorMsg.includes("already sent you a friend request")) {
        displayMsg = "Check your friend requests!"
      } else if (errorMsg.includes("User") && errorMsg.includes("not found")) {
        displayMsg = "User not found"
      } else if (errorMsg.includes("Already friends")) {
        displayMsg = "Already friends"
      }
      
      setRequestStatus({ type: 'error', message: displayMsg })
      setTimeout(() => setRequestStatus(null), 3000)
    } finally {
      setSendingRequest(false)
    }
  }

  const handleAcceptRequest = async (requestId: string) => {
    setProcessingRequestId(requestId)
    try {
      await invoke("accept_friend_request", { requestId })
      await loadFriends()
      await loadFriendRequests()
    } catch (error) {
      console.error("Failed to accept request:", error)
    } finally {
      setProcessingRequestId(null)
    }
  }

  const handleRejectRequest = async (requestId: string) => {
    setProcessingRequestId(requestId)
    setConfirmRemoveFriend(null) // Clear any existing confirm modal state
    try {
      await invoke("reject_friend_request", { requestId })
      await loadFriendRequests()
    } catch (error) {
      console.error("Failed to reject request:", error)
    } finally {
      setProcessingRequestId(null)
    }
  }

  const handleRemoveFriend = async (friendUuid: string, friendUsername: string) => {
    setConfirmRemoveFriend({ uuid: friendUuid, username: friendUsername })
  }

  const confirmRemove = async () => {
    if (!confirmRemoveFriend) return
    
    const friendToRemove = confirmRemoveFriend
    setConfirmRemoveFriend(null) // Close modal immediately
    
    try {
      await invoke("remove_friend", { friendUuid: friendToRemove.uuid })
      await loadFriends()
    } catch (error) {
      console.error("Failed to remove friend:", error)
    }
  }

  const recentInstances = [...instances]
    .filter(inst => inst.last_played)
    .sort((a, b) => {
      const timeA = a.last_played ? new Date(a.last_played).getTime() : 0
      const timeB = b.last_played ? new Date(b.last_played).getTime() : 0
      return timeB - timeA
    })
    .slice(0, 5)

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

  const handleSidebarContextMenu = (e: React.MouseEvent, instance: Instance) => {
    e.preventDefault()
    e.stopPropagation()
    setSidebarContextMenu({
      x: e.clientX,
      y: e.clientY,
      instance,
    })
  }

  const handleAddAccount = async () => {
    setIsLoggingIn(true)
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
      await invoke("update_specific_user_status", {
        userUuid: uuid,
        status: "offline", 
        currentInstance: null 
      }).catch(err => console.error("Failed to set offline:", err))
      
      await invoke("remove_account", { uuid })
      await loadAccounts()
      setShowAccountDropdown(false)
    } catch (error) {
      console.error("Failed to remove account:", error)
    }
  }

  const getStatusColor = (status: Friend["status"]) => {
    switch (status) {
      case "online":
        return "bg-green-500"
      case "ingame":
        return "bg-green-500"
      case "offline":
        return "bg-gray-500"
    }
  }

  const getStatusText = (friend: Friend) => {
    switch (friend.status) {
      case "online":
        return "Online"
      case "ingame":
        return friend.current_instance ? `Playing ${friend.current_instance}` : "In Game"
      case "offline":
        return "Offline"
    }
  }

  return (
    <>
      <aside 
        className="sidebar-bg w-58 bg-[#1a1a1a] flex flex-col relative"
        data-custom-bg={sidebarBackground ? "true" : undefined}
        style={{
          backgroundImage: sidebarBackground 
            ? `linear-gradient(to top, rgba(0, 0, 0, 0.9), rgba(0, 0, 0, 0.9)), url(${sidebarBackground})` 
            : undefined,
          backgroundSize: 'cover',
          backgroundPosition: 'center',
          backgroundRepeat: sidebarBackground ? 'repeat' : 'no-repeat',
          backgroundColor: '#1a1a1a'
        }}
      >
        <div className="flex-shrink-0 p-2 pt-4 space-y-1">
          {isAuthenticated && activeAccount ? (
            <>
              <div className="py-1">
                <button
                  ref={accountButtonRef}
                  onClick={() => setShowAccountDropdown(!showAccountDropdown)}
                  className="w-full flex items-center gap-2.5 p-2 cursor-pointer hover:bg-[#171717] rounded transition-colors"
                >
                  <div className="relative">
                    <img
                      src={`https://cravatar.eu/avatar/${activeAccount.username}/32`}
                      alt={activeAccount.username}
                      className="w-8 h-8 rounded"
                    />
                  </div>
                  <div className="flex-1 min-w-0 text-left">
                    <div className="text-xs text-[#7d8590]">Welcome back,</div>
                    <div className="text-sm font-medium text-[#e6edf3] truncate">{activeAccount.username}</div>
                  </div>
                  <div className="flex flex-col text-[#7d8590]">
                    <ChevronUp size={14} strokeWidth={2.5} />
                    <ChevronDown size={14} strokeWidth={2.5} />
                  </div>
                </button>
              </div>
            </>
          ) : (
            <button
              onClick={handleAddAccount}
              disabled={isLoggingIn}
              className="w-full flex items-center gap-3 px-4 py-2.5 rounded text-base font-medium transition-all cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed text-[#7d8590] hover:text-[#e6edf3] hover:bg-[#171717]"
            >
              <LogIn size={20} className="text-[#16a34a]" strokeWidth={2} />
              <span>{isLoggingIn ? 'Authenticating...' : 'Sign In'}</span>
            </button>
          )}
        </div>

        <div className="flex-1 flex flex-col overflow-hidden">
          {recentInstances.length > 0 && (
            <div className="flex-shrink-0 overflow-y-auto px-2 pb-3">
              <div className="py-2">
                <h3 className="text-xs font-semibold text-[#7d8590] uppercase tracking-wider mb-2 px-2">
                  Recently Played
                </h3>
                <div className="space-y-1">
                  {recentInstances.map((instance) => {
                    const icon = instanceIcons[instance.name]
                    const isRunning = runningInstances.has(instance.name)
                    const isLaunching = launchingInstanceName === instance.name
                    return (
                      <button
                        key={instance.name}
                        onClick={() => {
                          setSelectedInstance(instance)
                          setActiveTab("instances")
                          setShowInstanceDetails(true)
                        }}
                        onContextMenu={(e) => handleSidebarContextMenu(e, instance)}
                        className="group w-full flex items-center gap-2 rounded cursor-pointer transition-all text-[#7d8590] hover:text-[#e6edf3] hover:bg-[#171717] px-1.5 py-1.5 relative"
                      >
                        {icon ? (
                          <img
                            src={icon}
                            alt={instance.name}
                            className="w-9 h-9 rounded object-cover flex-shrink-0"
                          />
                        ) : (
                          <div className="w-9 h-9 flex items-center justify-center flex-shrink-0 bg-[#101010] rounded">
                            <Package size={24} className="text-[#3a3a3a]" strokeWidth={1.5} />
                          </div>
                        )}
                        <div className="flex-1 min-w-0 text-left group-hover:pr-10 transition-all">
                          <div className="text-sm font-medium text-[#e6edf3] truncate leading-tight">
                            {instance.name}
                          </div>
                          <div className="text-xs text-[#7d8590] leading-tight mt-0.5 truncate">
                            {getMinecraftVersion(instance)} • {formatLastPlayed(instance.last_played!)}
                          </div>
                        </div>
                        {isAuthenticated && (
                          <button
                            onClick={(e) => {
                              e.stopPropagation()
                              if (isRunning) {
                                onKillInstance(instance)
                              } else {
                                onQuickLaunch(instance)
                              }
                            }}
                            disabled={launchingInstanceName !== null}
                            className={`absolute right-1.5 opacity-0 group-hover:opacity-100 flex-shrink-0 w-8 h-8 flex items-center justify-center rounded transition-all cursor-pointer ${
                              isRunning || isLaunching
                                ? "bg-red-500/10 text-red-400 opacity-100 hover:bg-red-500/20"
                                : "bg-[#16a34a]/10 hover:bg-[#16a34a]/20 text-[#16a34a]"
                            } disabled:opacity-50`}
                            title={isRunning ? "Stop instance" : "Launch instance"}
                          >
                            {isLaunching ? (
                              <div className="w-3.5 h-3.5 border-2 border-red-400/30 border-t-red-400 rounded-full animate-spin" />
                            ) : isRunning ? (
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

          {isAuthenticated && (
            <div className="flex-1 overflow-y-auto px-2 pb-3">
              <div className="py-2">
                <div className="flex items-center justify-between mb-2">
                  <h3 className="text-xs font-semibold text-[#7d8590] uppercase tracking-wider px-2">
                    Friends
                  </h3>
                  <div className="flex items-center gap-1 px-2">
                    {friendRequests.length > 0 && (
                      <button
                        onClick={() => setShowRequests(!showRequests)}
                        className="relative text-[#7d8590] hover:text-[#e6edf3] transition-colors cursor-pointer"
                        title="Friend Requests"
                      >
                        <Bell size={14} strokeWidth={2} />
                        <div className="absolute -top-1 -right-1 w-3 h-3 bg-red-500 rounded-full flex items-center justify-center text-[8px] text-white font-bold">
                          {friendRequests.length}
                        </div>
                      </button>
                    )}
                    <button
                      onClick={() => setShowAddFriend(!showAddFriend)}
                      className="text-[#7d8590] hover:text-[#e6edf3] transition-colors cursor-pointer"
                      title="Add Friend"
                    >
                      <UserPlus size={14} strokeWidth={2} />
                    </button>
                  </div>
                </div>

                {showRequests && friendRequests.length > 0 && (
                  <div className="mb-2 space-y-1">
                    {friendRequests.map((request) => (
                      <div key={request.id} className="rounded px-2 py-2 hover:bg-[#171717] transition-colors">
                        <div className="flex items-center gap-2 mb-2">
                          <img
                            src={`https://cravatar.eu/avatar/${request.from_username}/24`}
                            alt={request.from_username}
                            className="w-5 h-5 rounded"
                          />
                          <span className="text-xs text-[#e6edf3] flex-1 truncate">
                            {request.from_username}
                          </span>
                        </div>
                        <div className="flex gap-1.5">
                          <button
                            onClick={() => handleAcceptRequest(request.id)}
                            disabled={processingRequestId !== null}
                            className="flex-1 bg-[#16a34a]/10 hover:bg-[#16a34a]/20 text-[#16a34a] px-3 py-1.5 rounded text-xs cursor-pointer flex items-center justify-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
                          >
                            {processingRequestId === request.id ? (
                              <div className="w-3 h-3 border-2 border-[#16a34a]/30 border-t-[#16a34a] rounded-full animate-spin" />
                            ) : (
                              <>
                                <Check size={12} />
                                Accept
                              </>
                            )}
                          </button>
                          <button
                            onClick={() => handleRejectRequest(request.id)}
                            disabled={processingRequestId !== null}
                            className="flex-1 bg-red-500/10 hover:bg-red-500/20 text-red-400 px-3 py-1.5 rounded text-xs cursor-pointer flex items-center justify-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
                          >
                            <X size={12} />
                            Reject
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                )}

                {showAddFriend && (
                  <div className="mb-2 px-1">
                    <div className="flex gap-1">
                      <input
                        type="text"
                        value={friendUsername}
                        onChange={(e) => setFriendUsername(e.target.value)}
                        placeholder="Username..."
                        className="flex-1 min-w-0 bg-[#101010] border border-[#2a2a2a] rounded px-2 py-1 text-xs text-[#e6edf3] placeholder-[#7d8590] focus:outline-none focus:border-[#3a3a3a]"
                        onKeyDown={(e) => {
                          if (e.key === 'Enter' && friendUsername.trim()) {
                            handleSendFriendRequest()
                          }
                        }}
                      />
                      <button
                        onClick={handleSendFriendRequest}
                        disabled={!friendUsername.trim() || sendingRequest}
                        className="bg-[#16a34a]/10 hover:bg-[#16a34a]/20 text-[#16a34a] px-2.5 py-1 rounded text-xs cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap flex items-center justify-center"
                      >
                        {sendingRequest ? (
                          <div className="w-3 h-3 border-2 border-[#16a34a]/30 border-t-[#16a34a] rounded-full animate-spin" />
                        ) : (
                          "Add"
                        )}
                      </button>
                    </div>
                    {requestStatus && (
                      <div className={`mt-1 px-2 py-1 rounded text-xs ${
                        requestStatus.type === 'success' 
                          ? 'bg-[#16a34a]/10 text-[#16a34a]' 
                          : 'bg-red-500/10 text-red-400'
                      }`}>
                        {requestStatus.message}
                      </div>
                    )}
                  </div>
                )}

                {friends.length === 0 ? (
                  <div className="flex flex-col items-center justify-center py-8 px-4 text-center">
                    <UserPlus size={32} className="text-[#3a3a3a] mb-3" strokeWidth={1.5} />
                    <p className="text-sm text-[#7d8590]">No friends yet</p>
                  </div>
                ) : (
                  <div className="space-y-1" key={friendsUpdateKey}>
                    {friends.map((friend) => {
                      const statusKey = `${friend.uuid}-${friend.status}-${friend.current_instance || 'none'}`
                      return (
                        <div
                          key={statusKey}
                          className="group relative flex items-center gap-2 px-1.5 py-1.5 rounded cursor-pointer hover:bg-[#171717] transition-all"
                        >
                          <div className="relative flex-shrink-0">
                            <img
                              src={`https://cravatar.eu/avatar/${friend.username}/32`}
                              alt={friend.username}
                              className="w-8 h-8 rounded"
                            />
                            <div className={`absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full border-2 border-[#1a1a1a] ${getStatusColor(friend.status)}`} />
                          </div>
                          <div className="flex-1 min-w-0 group-hover:pr-6 transition-all">
                            <div className="text-sm font-medium text-[#e6edf3] truncate leading-tight">
                              {friend.username}
                            </div>
                            <div className="flex items-center gap-1 text-xs text-[#7d8590] truncate leading-tight">
                              {friend.status === "ingame" && (
                                <Gamepad2 size={14} className="flex-shrink-0 text-green-400" />
                              )}
                              <span className="truncate">
                                {friend.status === "online" ? "Online" : 
                                 friend.status === "ingame" ? (friend.current_instance || "In Game") :
                                 "Offline"}
                              </span>
                            </div>
                          </div>
                          <button
                            onClick={() => handleRemoveFriend(friend.uuid, friend.username)}
                            className="absolute right-1.5 opacity-0 group-hover:opacity-100 p-1 hover:bg-red-500/10 rounded transition-all cursor-pointer bg-[#171717]"
                            title="Remove Friend"
                          >
                            <X size={14} className="text-red-400" />
                          </button>
                        </div>
                      )
                    })}
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      </aside>

      {showAccountDropdown && (
        <>
          <div 
            className="fixed inset-0 z-[60]"
            onClick={() => setShowAccountDropdown(false)}
          />
          
          <div 
            className="fixed bg-[#1a1a1a] rounded shadow-xl z-[70] overflow-hidden border border-[#2a2a2a]"
            style={{
              top: `${dropdownPosition.top}px`,
              left: `${dropdownPosition.left}px`,
              width: `${dropdownPosition.width}px`
            }}
          >
            {/* Active Account */}
            {activeAccount && (
              <div className="bg-[#0d0d0d] group">
                <div className="flex items-center gap-2 p-2.5">
                  <button
                    onClick={() => setShowAccountDropdown(false)}
                    className="flex-1 flex items-center gap-2.5 cursor-pointer"
                  >
                    <img
                      src={`https://cravatar.eu/avatar/${activeAccount.username}/32`}
                      alt={activeAccount.username}
                      className="w-8 h-8 rounded"
                    />
                    <div className="flex-1 min-w-0 text-left">
                      <div className="text-sm font-medium text-[#e6edf3] truncate">
                        {activeAccount.username}
                      </div>
                      <div className="text-xs text-[#7d8590]">
                        Active Account
                      </div>
                    </div>
                  </button>
                  <button
                    onClick={() => handleRemoveAccount(activeAccount.uuid)}
                    className="opacity-0 group-hover:opacity-100 p-1.5 hover:bg-red-500/10 rounded transition-all cursor-pointer"
                  >
                    <LogOut size={16} className="text-red-400" />
                  </button>
                </div>
              </div>
            )}

            {/* Other Accounts */}
            {accounts.filter(acc => !acc.is_active).length > 0 && (
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
                          className="w-8 h-8 rounded"
                        />
                        <div className="flex-1 min-w-0 text-left">
                          <div className="text-sm font-medium text-[#e6edf3] truncate">
                            {account.username}
                          </div>
                          <div className="text-xs text-[#7d8590]">
                            {formatLastUsed(account.last_used)}
                          </div>
                        </div>
                      </button>
                      
                      <button
                        onClick={() => handleRemoveAccount(account.uuid)}
                        className="opacity-0 group-hover:opacity-100 p-1.5 hover:bg-red-500/10 rounded transition-all cursor-pointer"
                      >
                        <LogOut size={16} className="text-red-400" />
                      </button>
                    </div>
                  ))}
              </div>
            )}

            {/* Separator before Add Account */}
            <div className="h-px bg-[#2a2a2a]" />
            
            {/* Add Another Account */}
            <button
              onClick={(e) => {
                e.stopPropagation()
                handleAddAccount()
              }}
              disabled={isLoggingIn}
              className="w-full flex items-center gap-2 px-2.5 py-2.5 text-sm text-[#e6edf3] hover:bg-[#1f1f1f] transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <LogIn size={18} className="text-[#16a34a]" />
              <span>{isLoggingIn ? 'Authenticating...' : 'Add another account'}</span>
            </button>
          </div>
        </>
      )}

      {sidebarContextMenu && (
        <ContextMenu
          x={sidebarContextMenu.x}
          y={sidebarContextMenu.y}
          onClose={() => setSidebarContextMenu(null)}
          items={[
            {
              label: "Open",
              icon: <Package size={16} />,
              onClick: () => {
                setSelectedInstance(sidebarContextMenu.instance)
                setActiveTab("instances")
                setShowInstanceDetails(true)
              },
            },
            {
              label: "Open Folder",
              icon: <FolderOpen size={16} />,
              onClick: () => {
                onOpenInstanceFolder(sidebarContextMenu.instance)
              },
            },
            {
              label: "Duplicate",
              icon: <Copy size={16} />,
              onClick: () => {
                onDuplicateInstance(sidebarContextMenu.instance)
              },
            },
            { separator: true },
            {
              label: "Delete",
              icon: <Trash2 size={16} />,
              onClick: () => {
                onDeleteInstance(sidebarContextMenu.instance.name)
              },
              danger: true,
            },
          ]}
        />
      )}

      <ConfirmModal
        isOpen={confirmRemoveFriend !== null}
        title="Remove Friend"
        message={`Are you sure you want to remove ${confirmRemoveFriend?.username || 'this friend'}? You can send them a friend request again later.`}
        confirmText="Remove"
        cancelText="Cancel"
        type="danger"
        onConfirm={confirmRemove}
        onCancel={() => setConfirmRemoveFriend(null)}
      />
    </>
  )
}