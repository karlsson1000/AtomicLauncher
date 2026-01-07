import { useState, useEffect, useRef } from "react"
import { invoke } from "@tauri-apps/api/core"
import { Search, Download, Loader2, Package, ChevronDown, ChevronLeft, ChevronRight, Heart } from "lucide-react"
import type { Instance, ModrinthSearchResult, ModrinthProject, ModrinthVersion } from "../../types"

interface ModFile {
  filename: string
  size: number
}

// Export the ModsTab selector component separately
interface ModsSelectorProps {
  instances: Instance[]
  selectedInstance: Instance | null
  onSetSelectedInstance: (instance: Instance) => void
  scrollContainerRef?: React.RefObject<HTMLDivElement>
}

export function ModsSelector({ instances, selectedInstance, onSetSelectedInstance }: ModsSelectorProps) {
  const [showInstanceSelector, setShowInstanceSelector] = useState(false)
  const instanceSelectorRef = useRef<HTMLDivElement>(null)
  const [instanceIcons, setInstanceIcons] = useState<Record<string, string | null>>({})

  useEffect(() => {
    const loadIcons = async () => {
      const icons: Record<string, string | null> = {}
      for (const instance of instances) {
        try {
          const icon = await invoke<string | null>("get_instance_icon", {
            instanceName: instance.name
          })
          icons[instance.name] = icon
        } catch (error) {
          console.error(`Failed to load icon for ${instance.name}:`, error)
          icons[instance.name] = null
        }
      }
      setInstanceIcons(icons)
    }

    if (instances.length > 0) {
      loadIcons()
    }
  }, [instances])

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (instanceSelectorRef.current && !instanceSelectorRef.current.contains(event.target as Node)) {
        setShowInstanceSelector(false)
      }
    }

    if (showInstanceSelector) {
      document.addEventListener('mousedown', handleClickOutside)
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [showInstanceSelector])

  const getMinecraftVersion = (instance: Instance): string => {
    if (instance.loader === "fabric") {
      const parts = instance.version.split('-')
      return parts[parts.length - 1]
    }
    return instance.version
  }

  if (!selectedInstance || selectedInstance.loader !== "fabric") {
    return null
  }

  return (
    <div className="relative self-center" ref={instanceSelectorRef}>
      <button
        onClick={() => setShowInstanceSelector(!showInstanceSelector)}
        className="flex items-center gap-2 px-3 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] rounded text-sm transition-colors cursor-pointer"
      >
        {instanceIcons[selectedInstance.name] ? (
          <img
            src={instanceIcons[selectedInstance.name]!}
            alt={selectedInstance.name}
            className="w-7 h-7 rounded object-cover flex-shrink-0"
          />
        ) : (
          <div className="w-7 h-7 flex items-center justify-center flex-shrink-0">
            <Package size={24} className="text-[#4a4a4a]" strokeWidth={1.5} />
          </div>
        )}
        <div className="text-left min-w-0">
          <div className="font-semibold text-[#e8e8e8] whitespace-nowrap leading-tight">{selectedInstance.name}</div>
          <div className="flex items-center gap-1 text-xs leading-tight mt-0.5">
            <span className="text-[#808080]">{getMinecraftVersion(selectedInstance)}</span>
            <span className="text-[#4a4a4a]">•</span>
            <span className="text-[#3b82f6]">Fabric</span>
          </div>
        </div>
        <ChevronDown size={16} className={`text-[#808080] ml-auto transition-transform ${showInstanceSelector ? 'rotate-180' : ''}`} strokeWidth={2} />
      </button>
      {showInstanceSelector && (
        <div className="absolute top-full mt-1 right-0 bg-[#1a1a1a] rounded overflow-hidden z-10 min-w-[240px] max-h-[400px] overflow-y-auto">
          {instances.filter(instance => instance.loader === "fabric").length === 0 ? (
            <div className="px-3 py-4 text-center">
              <p className="text-sm text-[#808080] mb-1">No Fabric instances</p>
              <p className="text-xs text-[#4a4a4a]">Create a Fabric instance to install mods</p>
            </div>
          ) : (
            instances
              .filter(instance => instance.loader === "fabric")
              .map((instance) => {
                const icon = instanceIcons[instance.name]
                return (
                  <button
                    key={instance.name}
                    onClick={() => {
                      onSetSelectedInstance(instance)
                      setShowInstanceSelector(false)
                    }}
                    className={`w-full flex items-center gap-3 px-3 py-2.5 text-left text-sm cursor-pointer transition-colors ${
                      selectedInstance.name === instance.name
                        ? "bg-[#3b82f6]/10 text-[#e8e8e8]"
                        : "text-[#808080] hover:bg-[#0d0d0d]"
                    }`}
                  >
                    {icon ? (
                      <img
                        src={icon}
                        alt={instance.name}
                        className="w-8 h-8 rounded object-cover flex-shrink-0"
                      />
                    ) : (
                      <div className="w-8 h-8 flex items-center justify-center flex-shrink-0">
                        <Package size={24} className="text-[#4a4a4a]" strokeWidth={1.5} />
                      </div>
                    )}
                    <div className="flex-1 min-w-0">
                      <div className="font-semibold text-[#e8e8e8] truncate">{instance.name}</div>
                      <div className="flex items-center gap-1 text-xs">
                        <span>{getMinecraftVersion(instance)}</span>
                        <span>•</span>
                        <span className="text-[#3b82f6]">Fabric</span>
                      </div>
                    </div>
                  </button>
                )
              })
          )}
        </div>
      )}
    </div>
  )
}

interface ModsTabProps {
  selectedInstance: Instance | null
  instances: Instance[]
  onSetSelectedInstance: (instance: Instance) => void
  scrollContainerRef?: React.RefObject<HTMLDivElement | null>
}

export function ModsTab({ selectedInstance, instances, onSetSelectedInstance, scrollContainerRef }: ModsTabProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [searchResults, setSearchResults] = useState<ModrinthSearchResult | null>(null)
  const [isSearching, setIsSearching] = useState(false)
  const [selectedMod, setSelectedMod] = useState<ModrinthProject | null>(null)
  const [modVersions, setModVersions] = useState<ModrinthVersion[]>([])
  const [isLoadingVersions, setIsLoadingVersions] = useState(false)
  const [downloadingMods, setDownloadingMods] = useState<Set<string>>(new Set())
  const [installedModFiles, setInstalledModFiles] = useState<Set<string>>(new Set())
  const [currentPage, setCurrentPage] = useState(1)
  const [itemsPerPage] = useState(20)
  const searchTimeoutRef = useRef<NodeJS.Timeout | null>(null)
  
  // Favorites state
  const [favoriteMods, setFavoriteMods] = useState<Array<{
    projectId: string
    title: string
    iconUrl?: string | null
  }>>([])
  const [showFavorites, setShowFavorites] = useState(false)
  const [installingFavorites, setInstallingFavorites] = useState(false)

  useEffect(() => {
    loadPopularMods()
    loadFavoriteMods()
  }, [])

  useEffect(() => {
    if (!selectedInstance || selectedInstance.loader !== "fabric") {
      const fabricInstances = instances.filter(instance => instance.loader === "fabric")
      if (fabricInstances.length > 0) {
        onSetSelectedInstance(fabricInstances[0])
      }
    }
  }, [instances, selectedInstance])

  useEffect(() => {
    if (selectedInstance && selectedInstance.loader === "fabric") {
      loadInstalledMods()
    }
  }, [selectedInstance])

  useEffect(() => {
    if (searchTimeoutRef.current) {
      clearTimeout(searchTimeoutRef.current)
    }
    searchTimeoutRef.current = setTimeout(() => {
      setCurrentPage(1)
      handleSearch(1)
    }, 500)
    return () => {
      if (searchTimeoutRef.current) {
        clearTimeout(searchTimeoutRef.current)
      }
    }
  }, [searchQuery])

  // Load favorite mods
  const loadFavoriteMods = async () => {
    try {
      const stored = await invoke<string>("get_favorite_mods")
      if (stored) {
        setFavoriteMods(JSON.parse(stored))
      }
    } catch (error) {
      console.error("Failed to load favorite mods:", error)
      setFavoriteMods([])
    }
  }

  // Save favorite mods
  const saveFavoriteMods = async (mods: typeof favoriteMods) => {
    try {
      await invoke("save_favorite_mods", {
        data: JSON.stringify(mods)
      })
      setFavoriteMods(mods)
    } catch (error) {
      console.error("Failed to save favorite mods:", error)
    }
  }

  const toggleFavorite = (mod: ModrinthProject) => {
    const exists = favoriteMods.some(m => m.projectId === mod.project_id)
    
    if (exists) {
      const updated = favoriteMods.filter(m => m.projectId !== mod.project_id)
      saveFavoriteMods(updated)
    } else {
      const updated = [...favoriteMods, {
        projectId: mod.project_id,
        title: mod.title,
        iconUrl: mod.icon_url
      }]
      saveFavoriteMods(updated)
    }
  }

  const isInFavorites = (projectId: string): boolean => {
    return favoriteMods.some(m => m.projectId === projectId)
  }

  const installAllFavorites = async () => {
    if (!selectedInstance || selectedInstance.loader !== "fabric") return
    
    setInstallingFavorites(true)
    
    for (const mod of favoriteMods) {
      try {
        const mcVersion = getMinecraftVersion(selectedInstance)
        const versions = await invoke<ModrinthVersion[]>("get_mod_versions", {
          idOrSlug: mod.projectId,
          loaders: [selectedInstance.loader],
          gameVersions: [mcVersion],
        })
        
        const version = versions[0]
        if (!version) continue
        
        const primaryFile = version.files.find(f => f.primary) || version.files[0]
        if (!primaryFile) continue
        
        if (installedModFiles.has(primaryFile.filename)) continue
        
        await invoke<string>("download_mod", {
          instanceName: selectedInstance.name,
          downloadUrl: primaryFile.url,
          filename: primaryFile.filename,
        })
        
        setInstalledModFiles(prev => new Set(prev).add(primaryFile.filename))
      } catch (error) {
        console.error(`Failed to install ${mod.title}:`, error)
      }
    }
    
    setInstallingFavorites(false)
  }

  const loadInstalledMods = async () => {
    if (!selectedInstance) return
    
    try {
      const mods = await invoke<ModFile[]>("get_installed_mods", {
        instanceName: selectedInstance.name,
      })
      
      const filenames = new Set(mods.map(mod => mod.filename))
      setInstalledModFiles(filenames)
    } catch (error) {
      console.error("Failed to load installed mods:", error)
    }
  }

  const loadPopularMods = async () => {
    setIsSearching(true)
    try {
      const facets = JSON.stringify([["project_type:mod"]])
      const result = await invoke<ModrinthSearchResult>("search_mods", {
        query: "",
        facets,
        index: "downloads",
        offset: 0,
        limit: itemsPerPage,
      })
      setSearchResults(result)
    } catch (error) {
      console.error("Failed to load popular mods:", error)
    } finally {
      setIsSearching(false)
    }
  }

  const handleSearch = async (page: number = currentPage) => {
    const query = searchQuery.trim()
    setIsSearching(true)
    try {
      const facets = JSON.stringify([["project_type:mod"]])
      const offset = (page - 1) * itemsPerPage
      const result = await invoke<ModrinthSearchResult>("search_mods", {
        query: query || "",
        facets,
        index: query ? "relevance" : "downloads",
        offset,
        limit: itemsPerPage,
      })
      setSearchResults(result)
      setSelectedMod(null)
    } catch (error) {
      console.error("Search error:", error)
    } finally {
      setIsSearching(false)
    }
  }

  const handlePageChange = (newPage: number) => {
    if (scrollContainerRef?.current) {
      scrollContainerRef.current.scrollIntoView({ behavior: 'smooth', block: 'start' })
    }
    window.scrollTo({ top: 0, behavior: 'smooth' })
    
    setCurrentPage(newPage)
    setTimeout(() => handleSearch(newPage), 100)
  }

  const getMinecraftVersion = (instance: Instance): string => {
    if (instance.loader === "fabric") {
      const parts = instance.version.split('-')
      return parts[parts.length - 1]
    }
    return instance.version
  }

  const handleModSelect = async (mod: ModrinthProject) => {
    if (!selectedInstance || selectedInstance.loader !== "fabric") return
    
    setSelectedMod(mod)
    setIsLoadingVersions(true)
    try {
      const mcVersion = getMinecraftVersion(selectedInstance)
      const loaders = [selectedInstance.loader]
      
      const versions = await invoke<ModrinthVersion[]>("get_mod_versions", {
        idOrSlug: mod.project_id,
        loaders: loaders,
        gameVersions: [mcVersion],
      })
      setModVersions(versions)
    } catch (error) {
      console.error("Failed to load versions:", error)
    } finally {
      setIsLoadingVersions(false)
    }
  }

  const isModInstalled = (version: ModrinthVersion): boolean => {
    return version.files.some(file => installedModFiles.has(file.filename))
  }

  const handleDownloadMod = async (version: ModrinthVersion) => {
    if (!selectedInstance || selectedInstance.loader !== "fabric") return

    const primaryFile = version.files.find(f => f.primary) || version.files[0]
    if (!primaryFile) return

    setDownloadingMods(prev => new Set(prev).add(version.id))
    
    try {
      await invoke<string>("download_mod", {
        instanceName: selectedInstance.name,
        downloadUrl: primaryFile.url,
        filename: primaryFile.filename,
      })
      
      setInstalledModFiles(prev => new Set(prev).add(primaryFile.filename))
    } catch (error) {
      console.error("Download error:", error)
    } finally {
      setDownloadingMods(prev => {
        const newSet = new Set(prev)
        newSet.delete(version.id)
        return newSet
      })
    }
  }

  const formatDownloads = (downloads: number): string => {
    if (downloads >= 1000000) return `${(downloads / 1000000).toFixed(1)}M`
    if (downloads >= 1000) return `${(downloads / 1000).toFixed(1)}K`
    return downloads.toString()
  }

  const totalPages = searchResults ? Math.ceil(searchResults.total_hits / itemsPerPage) : 1
  const showPagination = searchResults && searchResults.total_hits > itemsPerPage

  return (
     <div className="max-w-7xl mx-auto">
      <div className="flex gap-2 mb-4">
        <div className="relative flex-1">
          <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-[#4a4a4a]" strokeWidth={2} />
          <input
            type="text"
            placeholder="Search mods..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-[#1a1a1a] rounded pl-10 pr-4 py-2.5 text-sm text-[#e8e8e8] placeholder-[#4a4a4a] focus:outline-none focus:ring-2 focus:ring-[#2a2a2a] transition-all"
          />
          {isSearching && (
            <div className="absolute right-3 top-1/2 -translate-y-1/2">
              <Loader2 size={16} className="animate-spin text-[#16a34a]" />
            </div>
          )}
        </div>
        <button
          onClick={() => setShowFavorites(!showFavorites)}
          className={`p-2.5 rounded transition-colors relative cursor-pointer ${
            showFavorites 
              ? "bg-[#16a34a] text-white" 
              : "bg-[#1a1a1a] hover:bg-[#1f1f1f] text-[#808080]"
          }`}
        >
          <Heart size={18} className={showFavorites ? "fill-white" : ""} />
          {favoriteMods.length > 0 && (
            <span className="absolute -top-1 -right-1 bg-[#16a34a] text-white text-xs rounded-full w-5 h-5 flex items-center justify-center font-semibold">
              {favoriteMods.length}
            </span>
          )}
        </button>
      </div>

      {showFavorites && favoriteMods.length > 0 && (
        <div className="bg-[#1a1a1a] rounded-md p-4 mb-4">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-[#e8e8e8]">Favorite Mods ({favoriteMods.length})</h3>
            <button
              onClick={installAllFavorites}
              disabled={!selectedInstance || installingFavorites}
              className="flex items-center gap-2 px-4 py-2 bg-[#16a34a] hover:bg-[#15803d] disabled:opacity-50 disabled:cursor-not-allowed text-white rounded text-sm font-medium transition-colors cursor-pointer"
            >
              {installingFavorites ? (
                <>
                  <Loader2 size={16} className="animate-spin" />
                  Installing...
                </>
              ) : (
                <>
                  <Download size={16} />
                  Install All
                </>
              )}
            </button>
          </div>
          
          <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-8 gap-3">
            {favoriteMods.map((mod) => (
              <div
                key={mod.projectId}
                className="bg-[#0d0d0d] rounded p-3"
              >
                {mod.iconUrl ? (
                  <img src={mod.iconUrl} alt={mod.title} className="w-full aspect-square rounded-md mb-2" />
                ) : (
                  <div className="w-full aspect-square bg-gradient-to-br from-[#16a34a]/10 to-[#15803d]/10 rounded-md flex items-center justify-center mb-2">
                    <Package size={20} className="text-[#16a34a]/60" />
                  </div>
                )}
                <p className="text-xs text-[#e8e8e8] truncate font-medium">{mod.title}</p>
              </div>
            ))}
          </div>
        </div>
      )}

      {searchResults ? (
        <>
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
            <div className="lg:col-span-2 space-y-3">
              {searchResults.hits.map((mod) => (
                <div
                  key={mod.project_id}
                  className={`bg-[#1a1a1a] hover:bg-[#1f1f1f] rounded-md overflow-hidden cursor-pointer transition-all relative ${
                    selectedMod?.project_id === mod.project_id ? "ring-2 ring-[#2a2a2a]" : ""
                  }`}
                >
                  {showFavorites && (
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                        toggleFavorite(mod)
                      }}
                      className={`absolute bottom-3 right-3 p-2 rounded transition-colors z-10 cursor-pointer ${
                        isInFavorites(mod.project_id)
                          ? "bg-[#16a34a] text-white"
                          : "bg-[#0d0d0d] hover:bg-[#1a1a1a] text-[#808080] hover:text-[#16a34a]"
                      }`}
                    >
                      <Heart size={16} className={isInFavorites(mod.project_id) ? "fill-white" : ""} />
                    </button>
                  )}
                  <div 
                    className="flex min-h-0"
                    onClick={() => handleModSelect(mod)}
                  >
                    {mod.icon_url ? (
                      <div className="w-24 h-24 flex items-center justify-center flex-shrink-0 rounded m-2">
                        <img
                          src={mod.icon_url}
                          alt={mod.title}
                          className="w-full h-full object-contain rounded"
                        />
                      </div>
                    ) : (
                      <div className="w-24 h-24 bg-gradient-to-br from-[#16a34a]/10 to-[#15803d]/10 flex items-center justify-center flex-shrink-0 rounded m-2">
                        <Package size={48} className="text-[#16a34a]" />
                      </div>
                    )}
                    <div className="flex-1 min-w-0 py-2 px-3 flex items-center gap-3">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-start justify-between gap-2 mb-0">
                          <h3 className="font-semibold text-base text-[#e8e8e8] truncate">{mod.title}</h3>
                          <span className="text-xs text-[#808080] whitespace-nowrap">by {mod.author}</span>
                        </div>
                        <p className="text-sm text-[#808080] line-clamp-2 mb-2">{mod.description}</p>
                        <div className="flex items-center gap-2 text-xs flex-wrap">
                          <span className="flex items-center gap-1 bg-[#0d0d0d] px-2 py-1 rounded text-[#808080]">
                            <Download size={12} />
                            {formatDownloads(mod.downloads)}
                          </span>
                          {mod.categories.slice(0, 2).map((category) => (
                            <span key={category} className="bg-[#0d0d0d] px-2 py-1 rounded text-[#808080]">
                              {category}
                            </span>
                          ))}
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>

            {selectedMod && (
              <div className="bg-[#1a1a1a] rounded-md p-5 sticky top-4 self-start">
                <div className="flex gap-3 mb-4">
                  {selectedMod.icon_url && (
                    <img src={selectedMod.icon_url} alt={selectedMod.title} className="w-16 h-16 rounded" />
                  )}
                  <div className="flex-1 min-w-0">
                    <h2 className="text-xl font-semibold text-[#e8e8e8] truncate">{selectedMod.title}</h2>
                    <p className="text-sm text-[#808080]">by {selectedMod.author}</p>
                  </div>
                </div>
                
                <p className="text-sm text-[#808080] mb-4 leading-relaxed">{selectedMod.description}</p>
                
                <div className="flex gap-2 mb-5 text-xs flex-wrap">
                  <span className="flex items-center gap-1 bg-[#0d0d0d] px-2 py-1 rounded text-[#808080]">
                    <Download size={12} />
                    {formatDownloads(selectedMod.downloads)}
                  </span>
                  <span className="bg-[#0d0d0d] px-2 py-1 rounded text-[#808080]">{selectedMod.follows.toLocaleString()} followers</span>
                </div>

                <div className="border-t border-[#2a2a2a] pt-4">
                  <h3 className="font-semibold text-sm text-[#e8e8e8] mb-3">Versions</h3>
                  {isLoadingVersions ? (
                    <div className="text-center py-6">
                      <Loader2 size={20} className="animate-spin text-[#16a34a] mx-auto" />
                    </div>
                  ) : modVersions.length === 0 ? (
                    <p className="text-sm text-[#4a4a4a] text-center py-3">No compatible versions</p>
                  ) : (
                    <div className="space-y-2 max-h-96 overflow-y-auto">
                      {modVersions.map((version) => {
                        const installed = isModInstalled(version)
                        const downloading = downloadingMods.has(version.id)
                        
                        return (
                          <div
                            key={version.id}
                            className="bg-[#0d0d0d] rounded p-3 flex items-center justify-between gap-2"
                          >
                            <div className="flex-1 min-w-0">
                              <div className="text-sm font-medium text-[#e8e8e8] truncate">{version.name}</div>
                              <div className="text-xs text-[#4a4a4a] truncate mt-0.5">
                                {version.loaders.join(', ')} • {version.game_versions[0]}
                              </div>
                            </div>
                            <div className="flex gap-1">
                              <button
                                onClick={() => handleDownloadMod(version)}
                                disabled={!selectedInstance || downloading || installed}
                                className="px-3 py-2 bg-[#16a34a] hover:bg-[#15803d] disabled:opacity-50 disabled:cursor-not-allowed text-white rounded text-xs font-medium whitespace-nowrap transition-all shadow-sm cursor-pointer flex items-center gap-1"
                              >
                                {downloading ? (
                                  <Loader2 size={14} className="animate-spin" />
                                ) : installed ? (
                                  "Installed"
                                ) : (
                                  <>
                                    <Download size={14} />
                                    Install
                                  </>
                                )}
                              </button>
                            </div>
                          </div>
                        )
                      })}
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>

          {showPagination && (
            <div className="flex items-center justify-center gap-2 mt-6 pb-4">
              <button
                onClick={(e) => {
                  e.preventDefault()
                  handlePageChange(currentPage - 1)
                }}
                disabled={currentPage === 1}
                className="flex items-center gap-1 px-3 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] disabled:opacity-50 disabled:cursor-not-allowed text-[#e8e8e8] rounded text-sm transition-colors cursor-pointer"
              >
                <ChevronLeft size={16} />
                Previous
              </button>

              <div className="flex items-center gap-1">
                {currentPage > 2 && (
                  <>
                    <button
                      onClick={(e) => {
                        e.preventDefault()
                        handlePageChange(1)
                      }}
                      className="px-3 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] text-[#e8e8e8] rounded text-sm transition-colors cursor-pointer"
                    >
                      1
                    </button>
                    {currentPage > 3 && (
                      <span className="px-2 text-[#4a4a4a]">...</span>
                    )}
                  </>
                )}

                {currentPage > 1 && (
                  <button
                    onClick={(e) => {
                      e.preventDefault()
                      handlePageChange(currentPage - 1)
                    }}
                    className="px-3 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] text-[#e8e8e8] rounded text-sm transition-colors cursor-pointer"
                  >
                    {currentPage - 1}
                  </button>
                )}

                <button
                  className="px-3 py-2 bg-[#16a34a] text-white rounded text-sm font-medium"
                >
                  {currentPage}
                </button>

                {currentPage < totalPages && (
                  <button
                    onClick={(e) => {
                      e.preventDefault()
                      handlePageChange(currentPage + 1)
                    }}
                    className="px-3 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] text-[#e8e8e8] rounded text-sm transition-colors cursor-pointer"
                  >
                    {currentPage + 1}
                  </button>
                )}

                {currentPage < totalPages - 1 && (
                  <>
                    {currentPage < totalPages - 2 && (
                      <span className="px-2 text-[#4a4a4a]">...</span>
                    )}
                    <button
                      onClick={(e) => {
                        e.preventDefault()
                        handlePageChange(totalPages)
                      }}
                      className="px-3 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] text-[#e8e8e8] rounded text-sm transition-colors cursor-pointer"
                    >
                      {totalPages}
                    </button>
                  </>
                )}
              </div>

              <button
                onClick={(e) => {
                  e.preventDefault()
                  handlePageChange(currentPage + 1)
                }}
                disabled={currentPage === totalPages}
                className="flex items-center gap-1 px-3 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] disabled:opacity-50 disabled:cursor-not-allowed text-[#e8e8e8] rounded text-sm transition-colors cursor-pointer"
              >
                Next
                <ChevronRight size={16} />
              </button>
            </div>
          )}
        </>
      ) : null}
    </div>
  )
}