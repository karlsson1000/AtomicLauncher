import { useEffect, useRef, useState } from "react"
import { Upload, RotateCcw, Loader2, HatGlasses, ChevronDown } from "lucide-react"

interface SkinsTabProps {
  activeAccount?: { uuid: string; username: string } | null
  isAuthenticated?: boolean
  invoke?: (command: string, args?: any) => Promise<any>
}

interface CachedSkin {
  url: string
  variant: "classic" | "slim"
  timestamp: number
}

interface RecentSkin {
  url: string
  variant: "classic" | "slim"
  timestamp: number
}

interface Cape {
  id: string
  state: string
  url: string
  alias: string
  icon?: string
}

export function SkinsTab(props: SkinsTabProps) {
  const { activeAccount, isAuthenticated, invoke } = props
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [uploading, setUploading] = useState(false)
  const [resetting, setResetting] = useState(false)
  const [skinVariant, setSkinVariant] = useState<"classic" | "slim">("classic")
  const [capes, setCapes] = useState<Cape[]>([])
  const [activeCape, setActiveCape] = useState<string | null>(null)
  const [loadingCapes, setLoadingCapes] = useState(false)
  const [recentSkins, setRecentSkins] = useState<RecentSkin[]>([])
  const [capesExpanded, setCapesExpanded] = useState(false)
  const [currentSkinHash, setCurrentSkinHash] = useState<string | null>(null)
  const [showCape, setShowCape] = useState(true)
  const fileInputRef = useRef<HTMLInputElement>(null)
  const skinCacheRef = useRef<Map<string, CachedSkin>>(new Map())
  const CACHE_DURATION = 5 * 60 * 1000 // 5 minutes

  // Load recent skins from persistent storage on mount
  useEffect(() => {
    if (!activeAccount || !invoke) {
      setRecentSkins([])
      return
    }
    
    loadRecentSkins()
  }, [activeAccount, invoke])

  // Load recent skins from Tauri storage
  const loadRecentSkins = async () => {
    if (!activeAccount || !invoke) return
    
    try {
      const result = await invoke("load_recent_skins", { 
        accountUuid: activeAccount.uuid 
      })
      
      if (result && Array.isArray(result)) {
        setRecentSkins(result)
      } else {
        setRecentSkins([])
      }
    } catch (err) {
      console.error("Failed to load recent skins:", err)
      setRecentSkins([])
    }
  }

  // Save a skin to recent skins
  const addToRecentSkins = async (url: string, variant: "classic" | "slim") => {
    if (!activeAccount || !invoke) return
    
    try {
      // Create the new recent skin entry
      const newSkin: RecentSkin = { url, variant, timestamp: Date.now() }
      
      // Update local state optimistically
      setRecentSkins(prev => {
        const filtered = prev.filter(s => s.url !== url)
        return [newSkin, ...filtered].slice(0, 3)
      })
      
      // Save to persistent storage
      await invoke("save_recent_skin", {
        accountUuid: activeAccount.uuid,
        skinUrl: url,
        variant: variant
      })
    } catch (err) {
      console.error("Failed to save recent skin:", err)
    }
  }

  useEffect(() => {
    loadUserSkin()
  }, [activeAccount])

  const getCapeImageName = (alias: string) => {
    const specialCases: Record<string, string> = {
      "follower's": "followers",
      "purple heart": "purple",
      "15th anniversary": "15th"
    }
    
    const lowerAlias = alias.toLowerCase()
    if (specialCases[lowerAlias]) {
      return specialCases[lowerAlias]
    }
    
    return lowerAlias.replace(/\s+/g, '_').replace(/['']/g, '')
  }

  const loadUserSkin = async () => {
    if (!isAuthenticated || !activeAccount || !invoke) {
      setLoading(false)
      setError("Please sign in to view your skin")
      return
    }

    try {
      setLoading(true)
      setError(null)
      
      const cacheKey = activeAccount.uuid
      const cached = skinCacheRef.current.get(cacheKey)
      const now = Date.now()
      
      if (cached && (now - cached.timestamp) < CACHE_DURATION) {
        const match = cached.url.match(/texture\/([a-f0-9]+)/)
        if (match) {
          setCurrentSkinHash(match[1])
        }
        setSkinVariant(cached.variant)
        setLoading(false)
        loadCapes()
        return
      }
      
      const skinData = await invoke("get_current_skin")
      
      if (skinData && skinData.url) {
        const variant = skinData.variant === "slim" ? "slim" : "classic"
        
        const match = skinData.url.match(/texture\/([a-f0-9]+)/)
        if (match) {
          setCurrentSkinHash(match[1])
        }
        
        setSkinVariant(variant)
        
        skinCacheRef.current.set(cacheKey, {
          url: skinData.url,
          variant: variant,
          timestamp: now
        })
        
        setLoading(false)
        loadCapes()
      } else {
        setCurrentSkinHash(null)
        setLoading(false)
        loadCapes()
      }
    } catch (err) {
      console.error("Failed to load skin:", err)
      setError(`Failed to load skin: ${err}`)
      setLoading(false)
      setCurrentSkinHash(null)
    }
  }

  const loadCapes = async () => {
    if (!invoke || !isAuthenticated) return

    try {
      setLoadingCapes(true)
      const capeData = await invoke("get_user_capes")
      
      if (capeData && capeData.capes) {
        setCapes(capeData.capes)
        const active = capeData.capes.find((cape: Cape) => cape.state === "ACTIVE")
        const activeCapeId = active?.id || null
        setActiveCape(activeCapeId)
        setShowCape(!!activeCapeId)
      }
    } catch (err) {
      console.error("Failed to load capes:", err)
    } finally {
      setLoadingCapes(false)
    }
  }

  const handleCapeSelect = async (capeId: string) => {
    if (!invoke) return

    try {
      await invoke("equip_cape", { capeId })
      setActiveCape(capeId)
      setShowCape(true)
    } catch (err) {
      console.error("Failed to equip cape:", err)
      setError(`Failed to equip cape: ${err}`)
    }
  }

  const handleCapeRemove = async () => {
    if (!invoke) return
    
    try {
      await invoke("remove_cape")
      setActiveCape(null)
      setShowCape(false)
    } catch (err) {
      console.error("Failed to remove cape:", err)
      setError(`Failed to remove cape: ${err}`)
    }
  }

  const handleFileSelect = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    if (!file || !invoke) return

    const reader = new FileReader()
    reader.onload = async (e) => {
      const base64Data = e.target?.result as string
      const base64 = base64Data.split(',')[1]

      setUploading(true)
      setError(null)

      try {
        await invoke("upload_skin", {
          skinData: base64,
          variant: skinVariant
        })

        if (activeAccount) {
          skinCacheRef.current.delete(activeAccount.uuid)
        }

        await loadUserSkin()
        
        // Add to recent skins after successful upload
        const skinData = await invoke("get_current_skin")
        if (skinData && skinData.url) {
          await addToRecentSkins(skinData.url, skinData.variant === "slim" ? "slim" : "classic")
        }
        
        setError(null)
      } catch (err) {
        setError(`Upload failed: ${err}`)
      } finally {
        setUploading(false)
      }
    }

    reader.readAsDataURL(file)
  }

  const handleReset = async () => {
    if (!invoke) return

    setResetting(true)
    setError(null)

    try {
      await invoke("reset_skin")
      
      if (activeAccount) {
        skinCacheRef.current.delete(activeAccount.uuid)
      }
      
      await loadUserSkin()
    } catch (err) {
      setError(`Reset failed: ${err}`)
    } finally {
      setResetting(false)
    }
  }

  const handleRecentSkinSelect = async (skin: RecentSkin) => {
    if (!invoke) return

    setUploading(true)
    setError(null)

    try {
      // Download the skin image
      const response = await fetch(skin.url)
      const blob = await response.blob()
      
      // Convert to base64
      const base64 = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader()
        reader.onload = () => {
          const result = reader.result as string
          resolve(result.split(',')[1])
        }
        reader.onerror = reject
        reader.readAsDataURL(blob)
      })

      // Upload the skin
      await invoke("upload_skin", {
        skinData: base64,
        variant: skin.variant
      })

      if (activeAccount) {
        skinCacheRef.current.delete(activeAccount.uuid)
      }

      await loadUserSkin()
      
      // Move to top of recent skins
      await addToRecentSkins(skin.url, skin.variant)
      
      setError(null)
    } catch (err) {
      setError(`Failed to apply skin: ${err}`)
    } finally {
      setUploading(false)
    }
  }

  const getSkinRenderUrl = () => {
    if (!currentSkinHash) return null
    
    const variant = skinVariant === "slim" ? "slim" : "wide"
    const capeParam = showCape && activeCape ? "" : "&no=cape"
    return `https://vzge.me/full/512/${currentSkinHash}?${variant}${capeParam}`
  }

  if (!isAuthenticated) {
    return (
      <div className="p-6 space-y-4">
        <div className="max-w-7xl mx-auto">
          <div className="mb-6">
            <h1 className="text-2xl font-semibold text-[#e6edf3] tracking-tight">Skins</h1>
            <p className="text-sm text-[#7d8590] mt-0.5">Manage your Minecraft skin</p>
          </div>
          
          <div className="flex flex-col items-center justify-center min-h-[calc(100vh-300px)]">
            <HatGlasses size={64} className="text-[#238636] mb-4" strokeWidth={1.5} />
            <h3 className="text-lg font-semibold text-[#e6edf3] mb-1">Sign In Required</h3>
            <p className="text-sm text-[#7d8590]">Please sign in with your Microsoft account to manage your skin</p>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="p-6 space-y-4">
      <div className="max-w-7xl mx-auto">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h1 className="text-2xl font-semibold text-[#e6edf3] tracking-tight">Skins</h1>
            <p className="text-sm text-[#7d8590] mt-0.5">
              {activeAccount ? `Viewing skin for ${activeAccount.username}` : "Manage your Minecraft skin"}
            </p>
          </div>
        </div>

        <div className="flex gap-24 items-center justify-center">
          {/* Skin Viewer */}
          <div className="flex-shrink-0 self-start mt-8">
            <div className="rounded-md overflow-hidden relative bg-[#101010] p-4">
              {loading && (
                <div className="w-[250px] h-[406px] flex items-center justify-center bg-[#101010] rounded-md">
                  <div className="text-center">
                    <Loader2 size={32} className="animate-spin text-[#238636] mx-auto mb-3" />
                    <p className="text-sm text-[#7d8590]">Loading skin...</p>
                  </div>
                </div>
              )}
              
              {!loading && currentSkinHash && (
                <img
                  src={getSkinRenderUrl() || ''}
                  alt="Minecraft skin render"
                  className="w-[250px] h-[406px]"
                  style={{ imageRendering: 'pixelated' }}
                />
              )}
              
              {!loading && !currentSkinHash && (
                <div className="w-[300px] h-[487px] flex items-center justify-center">
                  <p className="text-sm text-[#7d8590]">No skin loaded</p>
                </div>
              )}
            </div>
          </div>

          {/* Controls Panel */}
          <div className="flex-1 max-w-sm space-y-4">
            <div className="bg-[#141414] border border-[#2a2a2a] rounded-md p-5">
              <h3 className="text-base font-semibold text-[#e6edf3] mb-4">Skin Model</h3>
              
              <div className="flex gap-2 mb-6">
                <button
                  onClick={() => setSkinVariant("classic")}
                  className={`flex-1 px-4 py-2.5 rounded-md text-sm font-medium transition-all cursor-pointer border ${
                    skinVariant === "classic"
                      ? "bg-[#4572e3] text-white border-[#4572e3]"
                      : "bg-[#0f0f0f] text-[#7d8590] hover:bg-[#1a1a1a] hover:text-[#e6edf3] border-[#2a2a2a]"
                  }`}
                >
                  Classic
                </button>
                <button
                  onClick={() => setSkinVariant("slim")}
                  className={`flex-1 px-4 py-2.5 rounded-md text-sm font-medium transition-all cursor-pointer border ${
                    skinVariant === "slim"
                      ? "bg-[#4572e3] text-white border-[#4572e3]"
                      : "bg-[#0f0f0f] text-[#7d8590] hover:bg-[#1a1a1a] hover:text-[#e6edf3] border-[#2a2a2a]"
                  }`}
                >
                  Slim
                </button>
              </div>

              <div className="space-y-3">
                <input
                  ref={fileInputRef}
                  type="file"
                  accept="image/png"
                  onChange={handleFileSelect}
                  className="hidden"
                />
                
                <button
                  onClick={() => fileInputRef.current?.click()}
                  disabled={uploading || loading}
                  className="w-full flex items-center justify-center gap-2 px-4 py-3 bg-[#238636] hover:bg-[#2ea043] disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-md font-medium text-sm transition-all cursor-pointer shadow-sm"
                >
                  {uploading ? (
                    <>
                      <Loader2 size={16} className="animate-spin" />
                      <span>Uploading...</span>
                    </>
                  ) : (
                    <>
                      <Upload size={16} />
                      <span>Upload New Skin</span>
                    </>
                  )}
                </button>
                
                <button
                  onClick={handleReset}
                  disabled={resetting || loading}
                  className="w-full flex items-center justify-center gap-2 px-4 py-3 bg-[#0f0f0f] hover:bg-[#1a1a1a] border border-[#2a2a2a] disabled:opacity-50 disabled:cursor-not-allowed text-[#e6edf3] rounded-md font-medium text-sm transition-all cursor-pointer"
                >
                  {resetting ? (
                    <>
                      <Loader2 size={16} className="animate-spin" />
                      <span>Resetting...</span>
                    </>
                  ) : (
                    <>
                      <RotateCcw size={16} />
                      <span>Reset to Default</span>
                    </>
                  )}
                </button>
              </div>
            </div>

            {/* Recent Skins */}
            {recentSkins.length > 0 && (
              <div className="bg-[#141414] border border-[#2a2a2a] rounded-md p-5">
                <div className="flex items-center gap-2 mb-4">
                  <h3 className="text-base font-semibold text-[#e6edf3]">Recent Skins</h3>
                </div>
                
                <div className="flex gap-3">
                  {recentSkins.map((skin, index) => {
                    // Extract texture hash from URL
                    const match = skin.url.match(/texture\/([a-f0-9]+)/)
                    const hash = match ? match[1] : null
                    // Use bust render from VZGE
                    const renderUrl = hash 
                      ? `https://vzge.me/bust/128/${hash}${skin.variant === 'slim' ? '?slim' : '?wide'}`
                      : skin.url
                    
                    return (
                      <button
                        key={`${skin.url}-${index}`}
                        onClick={() => handleRecentSkinSelect(skin)}
                        disabled={uploading}
                        className="bg-[#0f0f0f] border border-[#2a2a2a] rounded-md hover:ring-2 hover:ring-[#4572e3] transition-all disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer overflow-hidden"
                      >
                        <img
                          src={renderUrl}
                          alt="Recent skin"
                          className="w-20 h-20"
                          style={{ imageRendering: 'pixelated' }}
                        />
                      </button>
                    )
                  })}
                </div>
              </div>
            )}

            {/* Capes Section */}
            <div className="bg-[#141414] border border-[#2a2a2a] rounded-md p-5">
              <button
                onClick={() => setCapesExpanded(!capesExpanded)}
                className="w-full flex items-center justify-between cursor-pointer group"
              >
                <h3 className="text-base font-semibold text-[#e6edf3]">Capes</h3>
                <ChevronDown 
                  size={20}
                  strokeWidth={3}
                  className={`text-[#7d8590] group-hover:text-[#e6edf3] transition-all ${
                    capesExpanded ? 'rotate-180' : ''
                  }`}
                />
              </button>
              
              {capesExpanded && (
                <div className="mt-4">
                  {loadingCapes ? (
                    <div className="flex items-center justify-center py-8">
                      <Loader2 size={24} className="animate-spin text-[#238636]" />
                    </div>
                  ) : capes.length > 0 ? (
                    <div className="space-y-3">
                      <div className="grid grid-cols-4 gap-2">
                        {capes.map((cape) => (
                          <button
                            key={cape.id}
                            onClick={() => handleCapeSelect(cape.id)}
                            className={`w-20 h-32 bg-[#0f0f0f] border border-[#2a2a2a] rounded-md overflow-hidden flex items-center justify-center transition-all cursor-pointer hover:ring-2 hover:ring-[#4572e3] ${
                              activeCape === cape.id
                                ? "ring-2 ring-[#4572e3]"
                                : ""
                            }`}
                            title={cape.alias}
                          >
                            <img 
                              src={`/capes/${getCapeImageName(cape.alias)}.png`}
                              alt={cape.alias}
                              className="w-full h-full object-contain"
                              style={{ imageRendering: 'pixelated' }}
                              onError={(e) => {
                                e.currentTarget.src = '/logo.png'
                              }}
                            />
                          </button>
                        ))}
                      </div>
                      
                      {activeCape && (
                        <button
                          onClick={handleCapeRemove}
                          className="w-full px-3 py-2.5 bg-[#0f0f0f] hover:bg-[#1a1a1a] border border-[#2a2a2a] text-[#7d8590] hover:text-[#e6edf3] rounded-md text-sm font-medium transition-all cursor-pointer"
                        >
                          Remove Cape
                        </button>
                      )}
                    </div>
                  ) : (
                    <p className="text-sm text-[#7d8590] text-center py-4">
                      No capes available
                    </p>
                  )}
                </div>
              )}
            </div>
            
            {error && (
              <div className="bg-[#141414] border border-[#2a2a2a] rounded-md p-4">
                <p className="text-xs text-red-400 leading-relaxed">{error}</p>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}