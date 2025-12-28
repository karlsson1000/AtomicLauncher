import { useState, useEffect, useRef } from "react"
import { invoke } from "@tauri-apps/api/core"
import { Search, Download, Loader2, Package, ChevronDown, ChevronLeft, ChevronRight } from "lucide-react"
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
        className="flex items-center gap-2 px-3 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] rounded-lg text-sm transition-colors cursor-pointer"
      >
        {instanceIcons[selectedInstance.name] ? (
          <img
            src={instanceIcons[selectedInstance.name]!}
            alt={selectedInstance.name}
            className="w-7 h-7 rounded-lg object-cover flex-shrink-0"
          />
        ) : (
          <div className="w-7 h-7 flex items-center justify-center flex-shrink-0">
            <Package size={20} className="text-[#4a4a4a]" strokeWidth={1.5} />
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
        <div className="absolute top-full mt-1 right-0 bg-[#1a1a1a] rounded-lg overflow-hidden z-10 min-w-[240px] max-h-[400px] overflow-y-auto">
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
                        className="w-8 h-8 rounded-lg object-cover flex-shrink-0"
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
}

export function ModsTab({ selectedInstance, instances, onSetSelectedInstance }: ModsTabProps) {
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

  useEffect(() => {
    loadPopularMods()
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
      <div className="relative mb-4">
        <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-[#4a4a4a]" strokeWidth={2} />
        <input
          type="text"
          placeholder="Search mods..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="w-full bg-[#1a1a1a] rounded-lg pl-10 pr-4 py-2.5 text-sm text-[#e8e8e8] placeholder-[#4a4a4a] focus:outline-none focus:ring-2 focus:ring-[#16a34a] transition-all"
        />
        {isSearching && (
          <div className="absolute right-3 top-1/2 -translate-y-1/2">
            <Loader2 size={16} className="animate-spin text-[#16a34a]" />
          </div>
        )}
      </div>

      {searchResults && (
        <>
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
            <div className="lg:col-span-2 space-y-3">
              {searchResults.hits.map((mod) => (
                <div
                  key={mod.project_id}
                  onClick={() => handleModSelect(mod)}
                  className={`bg-[#1a1a1a] hover:bg-[#1f1f1f] rounded-xl p-4 cursor-pointer transition-all ${
                    selectedMod?.project_id === mod.project_id ? "ring-2 ring-[#16a34a]" : ""
                  }`}
                >
                  <div className="flex gap-4">
                    {mod.icon_url ? (
                      <img src={mod.icon_url} alt={mod.title} className="w-16 h-16 rounded-lg flex-shrink-0" />
                    ) : (
                      <div className="w-16 h-16 bg-gradient-to-br from-[#16a34a]/10 to-[#15803d]/10 rounded-lg flex items-center justify-center flex-shrink-0">
                        <Package size={24} className="text-[#16a34a]/60" strokeWidth={1.5} />
                      </div>
                    )}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-start justify-between gap-2 mb-0">
                        <h3 className="font-semibold text-base text-[#e8e8e8] truncate">{mod.title}</h3>
                        <span className="text-xs text-[#808080] whitespace-nowrap">by {mod.author}</span>
                      </div>
                      <p className="text-sm text-[#808080] line-clamp-2 mb-2">{mod.description}</p>
                      <div className="flex items-center gap-3 text-xs text-[#4a4a4a]">
                        <span className="flex items-center gap-1">
                          <Download size={12} />
                          {formatDownloads(mod.downloads)}
                        </span>
                        <span>•</span>
                        <span className="truncate">{mod.categories.slice(0, 2).join(', ')}</span>
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>

            {selectedMod && (
              <div className="bg-[#1a1a1a] rounded-xl p-5 sticky top-4 self-start">
                <div className="flex gap-3 mb-4">
                  {selectedMod.icon_url && (
                    <img src={selectedMod.icon_url} alt={selectedMod.title} className="w-16 h-16 rounded-lg" />
                  )}
                  <div className="flex-1 min-w-0">
                    <h2 className="text-xl font-semibold text-[#e8e8e8] truncate">{selectedMod.title}</h2>
                    <p className="text-sm text-[#808080]">by {selectedMod.author}</p>
                  </div>
                </div>
                
                <p className="text-sm text-[#808080] mb-4 leading-relaxed">{selectedMod.description}</p>
                
                <div className="flex gap-4 mb-5 text-xs text-[#4a4a4a]">
                  <span className="flex items-center gap-1">
                    <Download size={12} />
                    {formatDownloads(selectedMod.downloads)}
                  </span>
                  <span>{selectedMod.follows.toLocaleString()} followers</span>
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
                            className="bg-[#0d0d0d] rounded-lg p-3 flex items-center justify-between gap-2"
                          >
                            <div className="flex-1 min-w-0">
                              <div className="text-sm font-medium text-[#e8e8e8] truncate">{version.name}</div>
                              <div className="text-xs text-[#4a4a4a] truncate mt-0.5">
                                {version.loaders.join(', ')} • {version.game_versions[0]}
                              </div>
                            </div>
                            <button
                              onClick={() => handleDownloadMod(version)}
                              disabled={!selectedInstance || downloading || installed}
                              className="px-3 py-2 bg-[#16a34a] hover:bg-[#15803d] disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-md text-xs font-medium whitespace-nowrap transition-all shadow-sm cursor-pointer"
                            >
                              {downloading ? (
                                <Loader2 size={14} className="animate-spin" />
                              ) : installed ? (
                                "Installed"
                              ) : (
                                "Install"
                              )}
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

          {showPagination && (
            <div className="flex items-center justify-center gap-2 mt-6 pb-4">
              <button
                onClick={(e) => {
                  e.preventDefault()
                  handlePageChange(currentPage - 1)
                }}
                disabled={currentPage === 1}
                className="flex items-center gap-1 px-3 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] disabled:opacity-50 disabled:cursor-not-allowed text-[#e8e8e8] rounded-lg text-sm transition-colors cursor-pointer"
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
                      className="px-3 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] text-[#e8e8e8] rounded-lg text-sm transition-colors cursor-pointer"
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
                    className="px-3 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] text-[#e8e8e8] rounded-lg text-sm transition-colors cursor-pointer"
                  >
                    {currentPage - 1}
                  </button>
                )}

                <button
                  className="px-3 py-2 bg-[#16a34a] text-white rounded-lg text-sm font-medium"
                >
                  {currentPage}
                </button>

                {currentPage < totalPages && (
                  <button
                    onClick={(e) => {
                      e.preventDefault()
                      handlePageChange(currentPage + 1)
                    }}
                    className="px-3 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] text-[#e8e8e8] rounded-lg text-sm transition-colors cursor-pointer"
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
                      className="px-3 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] text-[#e8e8e8] rounded-lg text-sm transition-colors cursor-pointer"
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
                className="flex items-center gap-1 px-3 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] disabled:opacity-50 disabled:cursor-not-allowed text-[#e8e8e8] rounded-lg text-sm transition-colors cursor-pointer"
              >
                Next
                <ChevronRight size={16} />
              </button>
            </div>
          )}
        </>
      )}
    </div>
  )
}