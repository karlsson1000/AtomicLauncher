import { Server, Plus, Search, Trash2 } from "lucide-react"
import { useState, useEffect } from "react"
import { invoke } from "@tauri-apps/api/core"
import { CreateServerModal } from "../modals/CreateServerModal"
import { ConfirmModal } from "../modals/ConfirmModal"

interface ServerInfo {
  name: string
  address: string
  port: number
  status: "online" | "offline" | "unknown"
  players_online?: number
  players_max?: number
  version?: string
  motd?: string
  favicon?: string
  last_checked?: number
}

interface McSrvStatResponse {
  online: boolean
  ip?: string
  port?: number
  hostname?: string
  version?: string
  protocol?: {
    version: number
    name?: string
  }
  icon?: string
  motd?: {
    clean?: string[]
  }
  players?: {
    online: number
    max: number
  }
}

interface ServersTabProps {
}

export function ServersTab({}: ServersTabProps) {
  const [servers, setServers] = useState<ServerInfo[]>([])
  const [searchQuery, setSearchQuery] = useState("")
  const [selectedServer, setSelectedServer] = useState<ServerInfo | null>(null)
  const [showAddModal, setShowAddModal] = useState(false)
  const [serverToDelete, setServerToDelete] = useState<string | null>(null)

  useEffect(() => {
    loadServers()
  }, [])

  // Auto-refresh all servers every 5 minutes
  useEffect(() => {
    const interval = setInterval(() => {
      servers.forEach(server => {
        refreshServerStatus(server)
      })
    }, 5 * 60 * 1000) // 5 minutes

    return () => clearInterval(interval)
  }, [servers])

  const loadServers = async () => {
    try {
      const serverList = await invoke<ServerInfo[]>("get_servers")
      setServers(serverList)
      // Refresh status for all servers on load
      serverList.forEach(server => {
        refreshServerStatus(server)
      })
    } catch (error) {
      console.error("Failed to load servers:", error)
      setServers([])
    }
  }

  const fetchServerStatus = async (address: string, port: number): Promise<ServerInfo | null> => {
    try {
      const fullAddress = port === 25565 ? address : `${address}:${port}`
      const response = await fetch(`https://api.mcsrvstat.us/3/${fullAddress}`, {
        headers: {
          'User-Agent': 'AtomicLauncher/1.0'
        }
      })
      
      if (!response.ok) {
        throw new Error(`API returned ${response.status}`)
      }

      const data: McSrvStatResponse = await response.json()
      
      return {
        name: address,
        address: address,
        port: port,
        status: data.online ? "online" : "offline",
        players_online: data.players?.online,
        players_max: data.players?.max,
        version: data.protocol?.name || data.version,
        motd: data.motd?.clean?.join('\n'),
        favicon: data.icon ? `https://api.mcsrvstat.us/icon/${fullAddress}` : undefined,
        last_checked: Date.now()
      }
    } catch (error) {
      console.error("Failed to fetch server status:", error)
      return null
    }
  }

  const refreshServerStatus = async (server: ServerInfo) => {
    try {
      const statusData = await fetchServerStatus(server.address, server.port)
      
      if (statusData) {
        // Update server in list
        setServers(prev => prev.map(s => 
          s.name === server.name 
            ? { ...statusData, name: server.name } 
            : s
        ))
        
        // Update selected server if it's the one being refreshed
        if (selectedServer?.name === server.name) {
          setSelectedServer({ ...statusData, name: server.name })
        }

        // Save updated status to backend
        try {
          await invoke("update_server_status", {
            serverName: server.name,
            status: statusData
          })
        } catch (error) {
          console.error("Failed to save server status:", error)
        }
      }
    } catch (error) {
      console.error("Failed to refresh server:", error)
    }
  }

  const handleDeleteServer = async () => {
    if (!serverToDelete) return

    try {
      await invoke("delete_server", { serverName: serverToDelete })
      await loadServers()
      if (selectedServer?.name === serverToDelete) {
        setSelectedServer(null)
      }
    } catch (error) {
      console.error("Delete error:", error)
    } finally {
      setServerToDelete(null)
    }
  }

  const handleServerAdded = async (newServer: ServerInfo) => {
    // First, save the server to backend
    try {
      await invoke("add_server", {
        name: newServer.name,
        address: newServer.address,
        port: newServer.port,
      })
    } catch (error) {
      console.error("Failed to save server:", error)
      alert(`Failed to add server: ${error}`)
      return
    }

    // Then fetch initial status for the new server
    const statusData = await fetchServerStatus(newServer.address, newServer.port)
    
    if (statusData) {
      const serverWithStatus = { ...statusData, name: newServer.name }
      setServers(prev => [...prev, serverWithStatus])
      
      // Update backend with status
      try {
        await invoke("update_server_status", {
          serverName: newServer.name,
          status: statusData
        })
      } catch (error) {
        console.error("Failed to save server status:", error)
      }
    } else {
      // Add server even if status fetch failed
      setServers(prev => [...prev, { ...newServer, status: "unknown" }])
    }
  }

  const filteredServers = servers.filter(server =>
    server.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    server.address.toLowerCase().includes(searchQuery.toLowerCase())
  )

  const getStatusColor = (status: string) => {
    switch (status) {
      case "online": return "text-[#16a34a]"
      case "offline": return "text-[#dc2626]"
      default: return "text-[#808080]"
    }
  }

  const getStatusBgColor = (status: string) => {
    switch (status) {
      case "online": return "bg-[#16a34a]/10"
      case "offline": return "bg-[#dc2626]/10"
      default: return "bg-[#808080]/10"
    }
  }

  return (
    <div className="p-6 space-y-4">
      <div className="max-w-7xl mx-auto">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h1 className="text-2xl font-semibold text-[#e8e8e8] tracking-tight">Servers</h1>
            <p className="text-sm text-[#808080] mt-0.5">Manage your favorite Minecraft servers</p>
          </div>
          <button
            onClick={() => setShowAddModal(true)}
            className="px-4 py-2 bg-[#16a34a] hover:bg-[#15803d] text-white rounded-lg font-medium text-sm flex items-center gap-2 transition-all shadow-lg cursor-pointer"
          >
            <Plus size={16} strokeWidth={2} />
            <span>Add Server</span>
          </button>
        </div>

        {servers.length > 0 && (
          <div className="relative mb-4">
            <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-[#4a4a4a]" strokeWidth={2} />
            <input
              type="text"
              placeholder="Search servers..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full bg-[#1a1a1a] rounded-lg pl-10 pr-4 py-2.5 text-sm text-[#e8e8e8] placeholder-[#4a4a4a] focus:outline-none focus:ring-2 focus:ring-[#16a34a] transition-all"
            />
          </div>
        )}

        {servers.length === 0 ? (
          <div className="flex flex-col items-center justify-center min-h-[calc(100vh-300px)]">
            <Server size={64} className="text-[#16a34a] mb-4" strokeWidth={1.5} />
            <h3 className="text-lg font-semibold text-[#e8e8e8] mb-1">No servers added</h3>
            <p className="text-sm text-[#808080] mb-4">Add your first server to get started</p>
            <button
              onClick={() => setShowAddModal(true)}
              className="px-4 py-2 bg-[#16a34a] hover:bg-[#15803d] text-white rounded-lg font-medium text-sm flex items-center gap-2 transition-all shadow-lg cursor-pointer"
            >
              <Plus size={16} strokeWidth={2} />
              <span>Add Server</span>
            </button>
          </div>
        ) : filteredServers.length === 0 ? (
          <div className="bg-[#1a1a1a] rounded-xl p-12 flex flex-col items-center justify-center">
            <Search size={64} className="text-[#16a34a] mb-4" strokeWidth={1.5} />
            <h3 className="text-lg font-semibold text-[#e8e8e8] mb-1">No servers found</h3>
            <p className="text-sm text-[#808080]">Try adjusting your search query</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
            {filteredServers.map((server) => {
              const displayAddress = server.port === 25565 
                ? server.address 
                : `${server.address}:${server.port}`
              
              return (
                <div
                  key={server.name}
                  onClick={() => setSelectedServer(server)}
                  className="bg-[#1a1a1a] rounded-xl p-4 cursor-pointer transition-all hover:ring-2 hover:ring-[#2a2a2a] relative"
                >
                  <button
                    onClick={(e) => {
                      e.stopPropagation()
                      setServerToDelete(server.name)
                    }}
                    className="absolute top-4 right-4 px-3 py-2 bg-[#dc2626]/10 hover:bg-[#dc2626]/20 text-[#dc2626] rounded-lg text-xs font-medium transition-all cursor-pointer"
                    title="Remove server"
                  >
                    <Trash2 size={14} />
                  </button>

                  <div className="flex gap-3 mb-2 pr-10">
                    {server.favicon ? (
                      <img 
                        src={server.favicon} 
                        alt={server.name} 
                        className="w-16 h-16 rounded-lg object-cover"
                      />
                    ) : (
                      <div className="w-16 h-16 bg-gradient-to-br from-[#16a34a]/10 to-[#15803d]/10 rounded-lg flex items-center justify-center flex-shrink-0">
                        <Server size={28} className="text-[#16a34a]/60" strokeWidth={1.5} />
                      </div>
                    )}
                    <div className="flex-1 min-w-0">
                      <h3 className="font-semibold text-base text-[#e8e8e8] truncate mb-0">
                        {server.name}
                      </h3>
                      <div className="flex items-center gap-2 mb-1">
                        <span className="text-xs text-[#808080] truncate">
                          {displayAddress}
                        </span>
                      </div>
                      <div className="flex items-center gap-2">
                        <span className={`px-2 py-0.5 rounded text-xs font-medium ${getStatusBgColor(server.status)} ${getStatusColor(server.status)}`}>
                          {server.status}
                        </span>
                        {server.version && (
                          <span className="text-xs text-[#4a4a4a] truncate">{server.version}</span>
                        )}
                      </div>
                    </div>
                  </div>

                  {server.motd && (
                    <p className="text-xs text-[#808080] mb-2 truncate leading-relaxed">
                      {server.motd}
                    </p>
                  )}

                  {server.status === "online" && server.players_online !== undefined && (
                    <div className="flex items-center justify-between">
                      <span className="text-xs text-[#808080]">Players</span>
                      <span className="text-sm font-medium text-[#e8e8e8]">
                        {server.players_online.toLocaleString()} / {server.players_max?.toLocaleString()}
                      </span>
                    </div>
                  )}
                </div>
              )
            })}
          </div>
        )}
      </div>

      {showAddModal && (
        <CreateServerModal
          servers={servers}
          onClose={() => setShowAddModal(false)}
          onSuccess={handleServerAdded}
        />
      )}

      <ConfirmModal
        isOpen={serverToDelete !== null}
        title="Remove Server"
        message={`Are you sure you want to remove "${serverToDelete}"? This action cannot be undone.`}
        confirmText="Remove"
        cancelText="Cancel"
        type="danger"
        onConfirm={handleDeleteServer}
        onCancel={() => setServerToDelete(null)}
      />
    </div>
  )
}