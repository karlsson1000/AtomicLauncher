import { useState, useEffect, useRef } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { Search, Download, Loader2, Package, ChevronLeft, ChevronRight, CheckCircle, AlertCircle, ChevronDown } from "lucide-react"
import type { Instance, ModrinthSearchResult, ModrinthProject, ModrinthVersion } from "../../types"

interface ModpacksTabProps {
  instances: Instance[]
  onRefreshInstances?: () => void
  selectedVersion: string | null
  onSetSelectedVersion: (version: string | null) => void
  availableVersions: string[]
  onSetAvailableVersions: (versions: string[]) => void
  isLoadingVersions: boolean
  onSetIsLoadingVersions: (loading: boolean) => void
}

interface ModpackInstallProgress {
  instance: string
  progress: number
  stage: string
}

// Export the version selector component
interface VersionSelectorProps {
  selectedVersion: string | null
  onSetSelectedVersion: (version: string | null) => void
  versions: string[]
  isLoading: boolean
}

export function ModpackVersionSelector({ selectedVersion, onSetSelectedVersion, versions, isLoading }: VersionSelectorProps) {
  const [showVersionSelector, setShowVersionSelector] = useState(false)
  const versionSelectorRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (versionSelectorRef.current && !versionSelectorRef.current.contains(event.target as Node)) {
        setShowVersionSelector(false)
      }
    }

    if (showVersionSelector) {
      document.addEventListener('mousedown', handleClickOutside)
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [showVersionSelector])

  return (
    <div className="relative self-center" ref={versionSelectorRef}>
      <button
        onClick={() => setShowVersionSelector(!showVersionSelector)}
        disabled={isLoading}
        className="flex items-center gap-2 px-3 py-2 bg-[#1a1a1a] hover:bg-[#1f1f1f] rounded-lg text-sm transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
      >
        <Package size={20} className="text-[#4a4a4a]" strokeWidth={1.5} />
        <div className="text-left min-w-0">
          <div className="font-semibold text-[#e8e8e8] whitespace-nowrap leading-tight">
            {selectedVersion ? `Minecraft ${selectedVersion}` : "All Versions"}
          </div>
          <div className="text-xs text-[#808080] leading-tight mt-0.5">
            {selectedVersion ? "Filter by version" : "Show all"}
          </div>
        </div>
        <ChevronDown size={16} className={`text-[#808080] ml-auto transition-transform ${showVersionSelector ? 'rotate-180' : ''}`} strokeWidth={2} />
      </button>
      
      {showVersionSelector && (
        <div className="absolute top-full mt-1 right-0 bg-[#1a1a1a] rounded-lg overflow-hidden z-10 min-w-[240px] max-h-[400px] overflow-y-auto">
          <button
            onClick={() => {
              onSetSelectedVersion(null)
              setShowVersionSelector(false)
            }}
            className={`w-full flex items-center gap-3 px-3 py-2.5 text-left text-sm cursor-pointer transition-colors ${
              !selectedVersion
                ? "bg-[#3b82f6]/10 text-[#e8e8e8]"
                : "text-[#808080] hover:bg-[#0d0d0d]"
            }`}
          >
            <div className="w-8 h-8 flex items-center justify-center flex-shrink-0">
              <Package size={24} className="text-[#4a4a4a]" strokeWidth={1.5} />
            </div>
            <div className="flex-1 min-w-0">
              <div className="font-semibold text-[#e8e8e8]">All Versions</div>
              <div className="text-xs text-[#808080]">Show all modpacks</div>
            </div>
          </button>
          
          {versions.length > 0 && (
            <>
              <div className="px-3 py-2 text-xs font-semibold text-[#4a4a4a] border-t border-[#2a2a2a]">
                MINECRAFT VERSIONS
              </div>
              {versions.map((version) => (
                <button
                  key={version}
                  onClick={() => {
                    onSetSelectedVersion(version)
                    setShowVersionSelector(false)
                  }}
                  className={`w-full flex items-center gap-3 px-3 py-2.5 text-left text-sm cursor-pointer transition-colors ${
                    selectedVersion === version
                      ? "bg-[#3b82f6]/10 text-[#e8e8e8]"
                      : "text-[#808080] hover:bg-[#0d0d0d]"
                  }`}
                >
                  <div className="flex-1 min-w-0">
                    <div className="font-semibold text-[#e8e8e8]">{version}</div>
                  </div>
                </button>
              ))}
            </>
          )}
        </div>
      )}
    </div>
  )
}

export function ModpacksTab({ 
  instances, 
  onRefreshInstances,
  selectedVersion,
  onSetSelectedVersion,
  availableVersions,
  onSetAvailableVersions,
  isLoadingVersions,
  onSetIsLoadingVersions
}: ModpacksTabProps) {
  const [searchQuery, setSearchQuery] = useState("")
  const [searchResults, setSearchResults] = useState<ModrinthSearchResult | null>(null)
  const [isSearching, setIsSearching] = useState(false)
  const [currentPage, setCurrentPage] = useState(1)
  const [itemsPerPage] = useState(20)
  const searchTimeoutRef = useRef<NodeJS.Timeout | null>(null)
  
  const [installingModpacks, setInstallingModpacks] = useState<Set<string>>(new Set())
  const [modpackProgress, setModpackProgress] = useState<Record<string, ModpackInstallProgress>>({})
  const [installationStatus, setInstallationStatus] = useState<Record<string, 'success' | 'error'>>({})
  const [selectedVersions, setSelectedVersions] = useState<Record<string, string>>({})
  const [modpackVersions, setModpackVersions] = useState<Record<string, ModrinthVersion[]>>({})
  const [loadingVersions, setLoadingVersions] = useState<Set<string>>(new Set())
  const [modpackGalleries, setModpackGalleries] = useState<Record<string, string[]>>({})

  useEffect(() => {
    loadPopularModpacks()
    loadAvailableVersions()
  }, [])

  useEffect(() => {
    const unlisten = listen<ModpackInstallProgress>('modpack-install-progress', (event) => {
      const progress = event.payload
      setModpackProgress(prev => ({
        ...prev,
        [progress.instance]: progress
      }))
    })

    return () => {
      unlisten.then(fn => fn())
    }
  }, [])

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
  }, [searchQuery, selectedVersion])

  const loadAvailableVersions = async () => {
    onSetIsLoadingVersions(true)
    try {
      const versions = await invoke<string[]>("get_modpack_game_versions")
      onSetAvailableVersions(versions)
    } catch (error) {
      console.error("Failed to load available versions:", error)
    } finally {
      onSetIsLoadingVersions(false)
    }
  }

  const loadPopularModpacks = async () => {
    setIsSearching(true)
    try {
      let facets = [["project_type:modpack"]]
      if (selectedVersion) {
        facets.push([`versions:${selectedVersion}`])
      }
      
      const result = await invoke<ModrinthSearchResult>("search_mods", {
        query: "",
        facets: JSON.stringify(facets),
        index: "downloads",
        offset: 0,
        limit: itemsPerPage,
      })
      setSearchResults(result)
      
      for (const modpack of result.hits) {
        loadModpackGallery(modpack.project_id)
      }
    } catch (error) {
      console.error("Failed to load popular modpacks:", error)
    } finally {
      setIsSearching(false)
    }
  }

  const handleSearch = async (page: number = currentPage) => {
    const query = searchQuery.trim()
    setIsSearching(true)
    try {
      let facets = [["project_type:modpack"]]
      if (selectedVersion) {
        facets.push([`versions:${selectedVersion}`])
      }
      
      const offset = (page - 1) * itemsPerPage
      const result = await invoke<ModrinthSearchResult>("search_mods", {
        query: query || "",
        facets: JSON.stringify(facets),
        index: query ? "relevance" : "downloads",
        offset,
        limit: itemsPerPage,
      })
      setSearchResults(result)
      
      for (const modpack of result.hits) {
        loadModpackGallery(modpack.project_id)
      }
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

  const loadModpackVersions = async (modpack: ModrinthProject) => {
    const projectId = modpack.project_id
    if (modpackVersions[projectId] || loadingVersions.has(projectId)) {
      return
    }

    setLoadingVersions(prev => new Set(prev).add(projectId))
    try {
      const versions = await invoke<ModrinthVersion[]>("get_modpack_versions", {
        idOrSlug: modpack.slug,
        gameVersion: selectedVersion,
      })
      setModpackVersions(prev => ({ ...prev, [projectId]: versions }))
      if (versions.length > 0) {
        setSelectedVersions(prev => ({ ...prev, [projectId]: versions[0].id }))
      }
    } catch (error) {
      console.error("Failed to load versions:", error)
    } finally {
      setLoadingVersions(prev => {
        const newSet = new Set(prev)
        newSet.delete(projectId)
        return newSet
      })
    }
  }

  const loadModpackGallery = async (projectId: string) => {
    try {
      const projectDetails = await invoke<any>("get_project_details", {
        idOrSlug: projectId,
      })
      
      if (projectDetails.gallery && projectDetails.gallery.length > 0) {
        const sortedGallery = projectDetails.gallery
          .map((img: any) => ({
            url: img.url || img,
            featured: img.featured || false,
            title: (img.title || '').toLowerCase(),
            description: (img.description || '').toLowerCase(),
          }))
          .sort((a: any, b: any) => {
            if (a.featured && !b.featured) return -1
            if (!a.featured && b.featured) return 1
            
            const aHasBanner = a.title.includes('banner') || a.description.includes('banner')
            const bHasBanner = b.title.includes('banner') || b.description.includes('banner')
            if (aHasBanner && !bHasBanner) return -1
            if (!aHasBanner && bHasBanner) return 1
            
            const aHasHeader = a.title.includes('header') || a.description.includes('header')
            const bHasHeader = b.title.includes('header') || b.description.includes('header')
            if (aHasHeader && !bHasHeader) return -1
            if (!aHasHeader && bHasHeader) return 1
            
            return 0
          })
          .map((img: any) => img.url)
        
        setModpackGalleries(prev => ({ ...prev, [projectId]: sortedGallery }))
      }
    } catch (error) {
      console.error(`Failed to load gallery for ${projectId}:`, error)
    }
  }

  const handleInstallModpack = async (modpack: ModrinthProject) => {
    try {
      const projectId = modpack.project_id
      let versions = modpackVersions[projectId]
      
      if (!versions) {
        setLoadingVersions(prev => new Set(prev).add(projectId))
        versions = await invoke<ModrinthVersion[]>("get_modpack_versions", {
          idOrSlug: modpack.slug,
          gameVersion: selectedVersion,
        })
        setModpackVersions(prev => ({ ...prev, [projectId]: versions }))
        setLoadingVersions(prev => {
          const newSet = new Set(prev)
          newSet.delete(projectId)
          return newSet
        })
      }

      if (versions.length === 0) {
        alert("No versions available for this modpack")
        return
      }

      const versionId = selectedVersions[projectId] || versions[0].id
      const instanceName = modpack.title

      const existingInstance = instances.find(i => i.name === instanceName)
      const finalName = existingInstance ? `${instanceName}-${Date.now()}` : instanceName
      
      setInstallingModpacks(prev => new Set(prev).add(modpack.project_id))
      
      await invoke("install_modpack", {
        modpackSlug: modpack.slug,
        instanceName: finalName,
        versionId: versionId,
        preferredGameVersion: selectedVersion,
      })
      
      setInstallationStatus(prev => ({ ...prev, [modpack.project_id]: 'success' }))

      if (onRefreshInstances) {
        setTimeout(() => {
          onRefreshInstances()
        }, 500)
      }

      setTimeout(() => {
        setInstallingModpacks(prev => {
          const newSet = new Set(prev)
          newSet.delete(modpack.project_id)
          return newSet
        })
        setInstallationStatus(prev => {
          const newStatus = { ...prev }
          delete newStatus[modpack.project_id]
          return newStatus
        })
        setModpackProgress(prev => {
          const newProgress = { ...prev }
          delete newProgress[instanceName]
          return newProgress
        })
      }, 3000)

    } catch (error) {
      console.error("Failed to install modpack:", error)
      setInstallationStatus(prev => ({ ...prev, [modpack.project_id]: 'error' }))
      
      setInstallingModpacks(prev => {
        const newSet = new Set(prev)
        newSet.delete(modpack.project_id)
        return newSet
      })

      setTimeout(() => {
        setInstallationStatus(prev => {
          const newStatus = { ...prev }
          delete newStatus[modpack.project_id]
          return newStatus
        })
      }, 5000)
    }
  }

  const totalPages = searchResults ? Math.ceil(searchResults.total_hits / itemsPerPage) : 1
  const showPagination = searchResults && searchResults.total_hits > itemsPerPage

  return (
    <div className="max-w-7xl mx-auto">
      <div className="relative mb-4">
        <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-[#4a4a4a]" strokeWidth={2} />
        <input
          type="text"
          placeholder="Search modpacks..."
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
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {searchResults.hits.map((modpack) => {
              const isInstalling = installingModpacks.has(modpack.project_id)
              const status = installationStatus[modpack.project_id]
              const progress = modpackProgress[modpack.title]
              const isLoadingVersionsForThis = loadingVersions.has(modpack.project_id)
              const gallery = modpackGalleries[modpack.project_id] || []
              const backgroundImage = gallery.length > 0 ? gallery[0] : modpack.icon_url
              
              return (
                <div
                  key={modpack.project_id}
                  className="relative bg-[#1a1a1a] rounded-2xl overflow-hidden transition-all group h-64"
                >
                  <div className="absolute inset-0">
                    {backgroundImage ? (
                      <>
                        <img
                          src={backgroundImage}
                          alt={modpack.title}
                          className="w-full h-full object-cover"
                          style={{ 
                            objectPosition: 'center center'
                          }}
                        />
                        <div className="absolute inset-x-0 bottom-0 h-32 bg-gradient-to-t from-black/90 to-transparent" />
                      </>
                    ) : (
                      <div className="w-full h-full bg-gradient-to-br from-[#16a34a]/20 to-[#15803d]/20 flex items-center justify-center">
                        <Package size={80} className="text-[#16a34a]/40" strokeWidth={1.5} />
                      </div>
                    )}
                  </div>

                  <div className="relative h-full flex flex-col p-5">
                    <div className="mt-auto flex items-end justify-between gap-4">
                      <div className="flex items-center gap-3 flex-1 min-w-0">
                        <div className="w-16 h-16 rounded-xl overflow-hidden flex-shrink-0">
                          {modpack.icon_url ? (
                            <img
                              src={modpack.icon_url}
                              alt={modpack.title}
                              className="w-full h-full object-cover"
                            />
                          ) : (
                            <div className="w-full h-full flex items-center justify-center">
                              <Package size={32} className="text-white/60" strokeWidth={1.5} />
                            </div>
                          )}
                        </div>
                        <div className="flex-1 min-w-0">
                          <h3 className="text-lg font-bold text-white mb-0.5 truncate drop-shadow-lg">
                            {modpack.title}
                          </h3>
                          <p className="text-sm text-white/80 truncate drop-shadow-md">
                            by {modpack.author}
                          </p>
                        </div>
                      </div>

                      <button
                        onClick={() => {
                          loadModpackVersions(modpack)
                          handleInstallModpack(modpack)
                        }}
                        disabled={isInstalling || isLoadingVersionsForThis}
                        className="flex-shrink-0 flex items-center justify-center transition-all cursor-pointer disabled:cursor-not-allowed"
                      >
                        {status === 'success' ? (
                          <CheckCircle size={32} className="text-green-500" strokeWidth={2} />
                        ) : status === 'error' ? (
                          <AlertCircle size={32} className="text-red-500" strokeWidth={2} />
                        ) : isInstalling || isLoadingVersionsForThis ? (
                          <Loader2 size={32} className="text-white/60 animate-spin" strokeWidth={2} />
                        ) : (
                          <Download size={32} className="text-[#16a34a] hover:text-[#15803d]" strokeWidth={2} />
                        )}
                      </button>
                    </div>

                    {isInstalling && progress && (
                      <div className="mt-3">
                        <div className="flex items-center justify-between mb-1.5">
                          <span className="text-xs text-white font-medium truncate drop-shadow-md">{progress.stage}</span>
                          <span className="text-xs text-white/80 drop-shadow-md">{progress.progress}%</span>
                        </div>
                        <div className="w-full bg-black/40 rounded-full h-2 backdrop-blur-sm">
                          <div
                            className="bg-[#16a34a] h-2 rounded-full transition-all duration-300 shadow-lg"
                            style={{ width: `${progress.progress}%` }}
                          />
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              )
            })}
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