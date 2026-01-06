import { useEffect, useRef, useState } from "react"
import * as skinview3d from "skinview3d"
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
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const viewerRef = useRef<any>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)
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
    if (!canvasRef.current) return

    const viewer = new skinview3d.SkinViewer({
      canvas: canvasRef.current,
      width: 300,
      height: 500,
    })

    viewer.background = 0x0d0d0d
    viewer.zoom = 0.8
    viewer.autoRotate = true
    viewer.autoRotateSpeed = 0.2
    viewer.globalLight.intensity = 2.5
    viewer.cameraLight.intensity = 3.0
    
    viewerRef.current = viewer
    loadUserSkin(viewer)

    return () => {
      if (viewerRef.current) {
        viewerRef.current.dispose()
      }
    }
  }, [activeAccount])
  
  useEffect(() => {
    if (!viewerRef.current) return
    
    let animationId: number
    let lastTime = performance.now()
    
    function animate(currentTime: number) {
      if (viewerRef.current) {
        const deltaTime = currentTime - lastTime
        lastTime = currentTime
        
        if (viewerRef.current.autoRotate) {
          viewerRef.current.playerObject.rotation.y += (viewerRef.current.autoRotateSpeed * deltaTime) / 1000
        }
        
        viewerRef.current.render()
        animationId = requestAnimationFrame(animate)
      }
    }
    
    animationId = requestAnimationFrame(animate)
    
    return () => {
      if (animationId) {
        cancelAnimationFrame(animationId)
      }
    }
  }, [viewerRef.current])

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

  const loadUserSkin = async (viewer: any) => {
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
        await viewer.loadSkin(cached.url, {
          model: cached.variant === "slim" ? "slim" : "default"
        })
        setSkinVariant(cached.variant)
        viewer.render()
        setLoading(false)
        loadCapes()
        return
      }
      
      const skinData = await invoke("get_current_skin")
      
      if (skinData && skinData.url) {
        const variant = skinData.variant === "slim" ? "slim" : "classic"
        await viewer.loadSkin(skinData.url, {
          model: variant === "slim" ? "slim" : "default"
        })
        
        setSkinVariant(variant)
        
        skinCacheRef.current.set(cacheKey, {
          url: skinData.url,
          variant: variant,
          timestamp: now
        })
        
        viewer.render()
        setLoading(false)
        loadCapes()
      } else {
        const defaultSkinUrl = `https://cravatar.eu/avatar/${activeAccount.username}/128.png`
        await viewer.loadSkin(defaultSkinUrl)
        
        skinCacheRef.current.set(cacheKey, {
          url: defaultSkinUrl,
          variant: "classic",
          timestamp: now
        })
        
        viewer.render()
        setLoading(false)
        loadCapes()
      }
    } catch (err) {
      console.error("Failed to load skin:", err)
      setError(`Failed to load skin: ${err}`)
      setLoading(false)
      
      try {
        const steveSkinUrl = "https://textures.minecraft.net/texture/31f477eb1a7beee631c2ca64d06f8f68fa93a3386d04452ab27f43acdf1b60cb"
        await viewer.loadSkin(steveSkinUrl)
        viewer.render()
      } catch (fallbackErr) {
        console.error("Fallback skin also failed:", fallbackErr)
      }
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

        if (active && viewerRef.current) {
          try {
            await viewerRef.current.loadCape(active.url)
            viewerRef.current.render()
          } catch (err) {
            console.error("Failed to load active cape in viewer:", err)
          }
        }
      }
    } catch (err) {
      console.error("Failed to load capes:", err)
    } finally {
      setLoadingCapes(false)
    }
  }

  const handleCapeSelect = async (capeUrl: string, capeId: string) => {
    if (!viewerRef.current || !invoke) return

    try {
      await viewerRef.current.loadCape(capeUrl)
      viewerRef.current.render()
      
      await invoke("equip_cape", { capeId })
      
      setActiveCape(capeId)
    } catch (err) {
      console.error("Failed to equip cape:", err)
      setError(`Failed to equip cape: ${err}`)
    }
  }

  const handleCapeRemove = async () => {
    if (!viewerRef.current || !invoke) return
    
    try {
      viewerRef.current.loadCape(null)
      viewerRef.current.render()
      
      await invoke("remove_cape")
      
      setActiveCape(null)
    } catch (err) {
      console.error("Failed to remove cape:", err)
      setError(`Failed to remove cape: ${err}`)
    }
  }

  const handleFileSelect = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    if (!file || !invoke || !viewerRef.current) return

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

        await loadUserSkin(viewerRef.current)
        
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
    if (!invoke || !viewerRef.current) return

    setResetting(true)
    setError(null)

    try {
      await invoke("reset_skin")
      
      if (activeAccount) {
        skinCacheRef.current.delete(activeAccount.uuid)
      }
      
      await loadUserSkin(viewerRef.current)
    } catch (err) {
      setError(`Reset failed: ${err}`)
    } finally {
      setResetting(false)
    }
  }

  const handleRecentSkinSelect = async (skin: RecentSkin) => {
    if (!viewerRef.current || !invoke) return

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

      await loadUserSkin(viewerRef.current)
      
      // Move to top of recent skins
      await addToRecentSkins(skin.url, skin.variant)
      
      setError(null)
    } catch (err) {
      setError(`Failed to apply skin: ${err}`)
    } finally {
      setUploading(false)
    }
  }

  if (!isAuthenticated) {
    return (
      <div className="p-6 space-y-4">
        <div className="max-w-7xl mx-auto">
          <div className="mb-6">
            <h1 className="text-2xl font-semibold text-[#e8e8e8] tracking-tight">Skins</h1>
            <p className="text-sm text-[#808080] mt-0.5">Manage your Minecraft skin</p>
          </div>
          
          <div className="flex flex-col items-center justify-center min-h-[calc(100vh-300px)]">
            <HatGlasses size={64} className="text-[#16a34a] mb-4" strokeWidth={1.5} />
            <h3 className="text-lg font-semibold text-[#e8e8e8] mb-1">Sign In Required</h3>
            <p className="text-sm text-[#808080]">Please sign in with your Microsoft account to manage your skin</p>
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
            <h1 className="text-2xl font-semibold text-[#e8e8e8] tracking-tight">Skins</h1>
            <p className="text-sm text-[#808080] mt-0.5">
              {activeAccount ? `Viewing skin for ${activeAccount.username}` : "Manage your Minecraft skin"}
            </p>
          </div>
        </div>

        <div className="flex gap-24 items-start justify-center">
          {/* 3D Skin Viewer */}
          <div className="flex-shrink-0">
            <div className="rounded-md overflow-hidden relative">
              {loading && (
                <div className="absolute inset-0 flex items-center justify-center bg-[#0d0d0d] bg-opacity-90 z-10 rounded-md">
                  <div className="text-center">
                    <Loader2 size={32} className="animate-spin text-[#16a34a] mx-auto mb-3" />
                    <p className="text-sm text-[#808080]">Loading skin...</p>
                  </div>
                </div>
              )}
              
              <div className="flex items-center justify-center p-4">
                <canvas 
                  ref={canvasRef}
                  className="rounded"
                />
              </div>
            </div>
          </div>

          {/* Controls Panel */}
          <div className="flex-1 max-w-sm space-y-4">
            <div className="bg-[#1a1a1a] rounded-md p-5">
              <h3 className="text-base font-semibold text-[#e8e8e8] mb-4">Skin Model</h3>
              
              <div className="flex gap-2 mb-6">
                <button
                  onClick={() => {
                    setSkinVariant("classic")
                    if (viewerRef.current) {
                      const currentSkinUrl = viewerRef.current.skinCanvas?.source?.img?.src
                      if (currentSkinUrl) {
                        viewerRef.current.loadSkin(currentSkinUrl, { model: "default" })
                      }
                      viewerRef.current.render()
                    }
                  }}
                  className={`flex-1 px-4 py-2.5 rounded text-sm font-medium transition-all cursor-pointer ${
                    skinVariant === "classic"
                      ? "bg-[#4572e3] text-white"
                      : "bg-[#0d0d0d] text-[#808080] hover:bg-[#1f1f1f] hover:text-[#e8e8e8]"
                  }`}
                >
                  Classic
                </button>
                <button
                  onClick={() => {
                    setSkinVariant("slim")
                    if (viewerRef.current) {
                      const currentSkinUrl = viewerRef.current.skinCanvas?.source?.img?.src
                      if (currentSkinUrl) {
                        viewerRef.current.loadSkin(currentSkinUrl, { model: "slim" })
                      }
                      viewerRef.current.render()
                    }
                  }}
                  className={`flex-1 px-4 py-2.5 rounded text-sm font-medium transition-all cursor-pointer ${
                    skinVariant === "slim"
                      ? "bg-[#4572e3] text-white"
                      : "bg-[#0d0d0d] text-[#808080] hover:bg-[#1f1f1f] hover:text-[#e8e8e8]"
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
                  className="w-full flex items-center justify-center gap-2 px-4 py-3 bg-[#16a34a] hover:bg-[#15803d] disabled:opacity-50 disabled:cursor-not-allowed text-white rounded font-medium text-sm transition-all cursor-pointer"
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
                  className="w-full flex items-center justify-center gap-2 px-4 py-3 bg-[#0d0d0d] hover:bg-[#1f1f1f] disabled:opacity-50 disabled:cursor-not-allowed text-[#e8e8e8] rounded font-medium text-sm transition-all cursor-pointer"
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
              <div className="bg-[#1a1a1a] rounded-md p-5">
                <div className="flex items-center gap-2 mb-4">
                  <h3 className="text-base font-semibold text-[#e8e8e8]">Recent Skins</h3>
                </div>
                
                <div className="flex gap-3">
                  {recentSkins.map((skin, index) => {
                    // Extract texture hash from URL
                    const match = skin.url.match(/texture\/([a-f0-9]+)/)
                    const hash = match ? match[1] : null
                    // Use head avatar
                    const renderUrl = hash ? `https://mc-heads.net/avatar/${hash}/128` : skin.url
                    
                    return (
                      <button
                        key={`${skin.url}-${index}`}
                        onClick={() => handleRecentSkinSelect(skin)}
                        disabled={uploading}
                        className="bg-[#0d0d0d] rounded hover:ring-2 hover:ring-[#4572e3] transition-all disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer overflow-hidden"
                      >
                        <img
                          src={renderUrl}
                          alt="Recent skin"
                          className="w-16 h-16"
                          style={{ imageRendering: 'pixelated' }}
                        />
                      </button>
                    )
                  })}
                </div>
              </div>
            )}

            {/* Capes Section */}
            <div className="bg-[#1a1a1a] rounded-md p-5">
              <button
                onClick={() => setCapesExpanded(!capesExpanded)}
                className="w-full flex items-center justify-between cursor-pointer group"
              >
                <h3 className="text-base font-semibold text-[#e8e8e8]">Capes</h3>
                <ChevronDown 
                  size={20}
                  strokeWidth={3}
                  className={`text-[#808080] group-hover:text-[#e8e8e8] transition-all ${
                    capesExpanded ? 'rotate-180' : ''
                  }`}
                />
              </button>
              
              {capesExpanded && (
                <div className="mt-4">
                  {loadingCapes ? (
                    <div className="flex items-center justify-center py-8">
                      <Loader2 size={24} className="animate-spin text-[#16a34a]" />
                    </div>
                  ) : capes.length > 0 ? (
                    <div className="space-y-3">
                      <div className="grid grid-cols-4 gap-2">
                        {capes.map((cape) => (
                          <button
                            key={cape.id}
                            onClick={() => handleCapeSelect(cape.url, cape.id)}
                            className={`w-20 h-32 bg-[#0d0d0d] rounded overflow-hidden flex items-center justify-center transition-all cursor-pointer hover:ring-2 hover:ring-[#4572e3] ${
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
                          className="w-full px-3 py-2.5 bg-[#0d0d0d] hover:bg-[#1f1f1f] text-[#808080] hover:text-[#e8e8e8] rounded text-sm font-medium transition-all cursor-pointer"
                        >
                          Remove Cape
                        </button>
                      )}
                    </div>
                  ) : (
                    <p className="text-sm text-[#808080] text-center py-4">
                      No capes available
                    </p>
                  )}
                </div>
              )}
            </div>
            
            {error && (
              <div className="bg-[#1a1a1a] rounded p-4">
                <p className="text-xs text-red-400 leading-relaxed">{error}</p>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}