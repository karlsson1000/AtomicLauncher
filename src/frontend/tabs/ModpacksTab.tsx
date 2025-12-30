import { useState, useEffect, useRef } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { open } from '@tauri-apps/plugin-dialog'
import { Search, Download, Loader2, Package, ChevronLeft, ChevronRight, CheckCircle, AlertCircle } from "lucide-react"
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
  onImport?: () => void
  onShowCreationToast?: (instanceName: string) => void
  scrollContainerRef?: React.RefObject<HTMLDivElement | null>
}

interface ModpackInstallProgress {
  instance: string
  progress: number
  stage: string
}

const YOUR_MODPACK_SLUG = "stellarmc-enhanced"
const YOUR_MODPACK_AUTHOR = "StellarMC"

export function ModpacksTab({ 
  instances, 
  onRefreshInstances,
  selectedVersion,
  onSetSelectedVersion,
  availableVersions,
  onSetAvailableVersions,
  isLoadingVersions,
  onSetIsLoadingVersions,
  onImport,
  onShowCreationToast,
  scrollContainerRef
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
  const [showInstalledOnly, setShowInstalledOnly] = useState(false)
  const [customModpack, setCustomModpack] = useState<ModrinthProject | null>(null)

  useEffect(() => {
    if (onImport) {
      window.uploadModpackHandler = handleUploadModpack
    }
  }, [onImport])

  useEffect(() => {
    loadCustomModpack()
    loadPopularModpacks()
    loadAvailableVersions()
  }, [])

  useEffect(() => {
    const unlisten = listen<ModpackInstallProgress>('modpack-install-progress', (event) => {
      const progress = event.payload
      console.log("Modpack progress event:", progress)
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

  const loadCustomModpack = async () => {
    try {
      const projectDetails = await invoke<any>("get_project_details", {
        idOrSlug: YOUR_MODPACK_SLUG,
      })

      const modpackData: ModrinthProject = {
        project_id: projectDetails.id,
        slug: projectDetails.slug,
        title: projectDetails.title,
        description: projectDetails.description,
        author: YOUR_MODPACK_AUTHOR,
        icon_url: projectDetails.icon_url,
        downloads: projectDetails.downloads || 0,
        follows: 0,
        date_created: "",
        date_modified: "",
        latest_version: "",
        license: "",
        client_side: "required",
        server_side: "optional",
        project_type: "modpack",
        categories: [],
        versions: [],
        gallery: [],
        display_categories: []
      }
      
      setCustomModpack(modpackData)
      loadModpackGallery(projectDetails.id)
    } catch (error) {
      console.error("Failed to load your modpack:", error)
    }
  }

  const loadAvailableVersions = async () => {
    onSetIsLoadingVersions(true)
    try {
      const versions = await invoke<string[]>("get_modpack_game_versions")
      const stableVersions = versions.filter(version => {
        return /^1\.\d+(\.\d+)?$/.test(version)
      })
      onSetAvailableVersions(stableVersions)
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
    if (scrollContainerRef?.current) {
      scrollContainerRef.current.scrollIntoView({ behavior: 'smooth', block: 'start' })
    }
    window.scrollTo({ top: 0, behavior: 'smooth' })
    
    setCurrentPage(newPage)
    setTimeout(() => handleSearch(newPage), 100)
  }

  const isModpackInstalled = (modpackTitle: string): boolean => {
    if (!selectedVersion) {
      return instances.some(instance => instance.name === modpackTitle)
    }
    
    return instances.some(instance => {
      if (instance.name !== modpackTitle) return false

      let instanceMcVersion = instance.version
      if (instance.loader === "fabric") {
        const parts = instance.version.split('-')
        instanceMcVersion = parts[parts.length - 1]
      } else if (instance.loader === "forge") {
        const parts = instance.version.split('-')
        instanceMcVersion = parts[0]
      }
      
      return instanceMcVersion === selectedVersion
    })
  }

  const getFilteredModpacks = () => {
    if (!searchResults) return []
    
    const modpacks = showInstalledOnly 
      ? searchResults.hits.filter(modpack => isModpackInstalled(modpack.title))
      : searchResults.hits

    const shouldShowYourModpack = currentPage === 1 && !searchQuery.trim() && customModpack
    
    if (shouldShowYourModpack) {
      const filteredModpacks = modpacks.filter(m => m.project_id !== customModpack!.project_id)
      return [customModpack!, ...filteredModpacks]
    }
    
    return modpacks
  }

  const getPaginatedModpacks = () => {
    const filtered = getFilteredModpacks()
    if (!showInstalledOnly && !searchQuery.trim() && currentPage === 1 && customModpack) {
      return filtered.slice(0, itemsPerPage)
    }
    
    if (!showInstalledOnly) return filtered
    
    const startIndex = (currentPage - 1) * itemsPerPage
    const endIndex = startIndex + itemsPerPage
    return filtered.slice(startIndex, endIndex)
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

  const formatDownloads = (downloads: number): string => {
    if (downloads >= 1000000) return `${(downloads / 1000000).toFixed(1)}M`
    if (downloads >= 1000) return `${(downloads / 1000).toFixed(1)}K`
    return downloads.toString()
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
      
      console.log("Installing modpack:", {
        slug: modpack.slug,
        instanceName: finalName,
        versionId: versionId,
        gameVersion: selectedVersion
      })
      
      if (onShowCreationToast) {
        onShowCreationToast(finalName)
      }
      
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

  const handleUploadModpack = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'Modpack Files',
          extensions: ['mrpack', 'zip']
        }]
      })

      if (!selected) return

      const filePath = selected as string
      
      try {
        // Extract modpack name from the file
        const modpackName = await invoke<string>("get_modpack_name_from_file", {
          filePath: filePath
        })
        
        console.log("Extracted modpack name:", modpackName)
        
        // Check if instance already exists and generate unique name if needed
        let finalName = modpackName
        const existingInstance = instances.find(
          i => i.name.toLowerCase() === modpackName.toLowerCase()
        )
        
        if (existingInstance) {
          finalName = `${modpackName}-${Date.now()}`
          console.log("Instance exists, using name:", finalName)
        }
        
        // Show toast notification
        if (onShowCreationToast) {
          onShowCreationToast(finalName)
        }
        
        // Import directly without modal
        await invoke("install_modpack_from_file", {
          filePath: filePath,
          instanceName: finalName,
          preferredGameVersion: selectedVersion,
        })

        if (onRefreshInstances) {
          setTimeout(() => {
            onRefreshInstances()
          }, 500)
        }
        
      } catch (error) {
        console.error("Failed to import modpack:", error)
        alert(`Failed to install modpack: ${error}`)
      }
      
    } catch (error) {
      console.error("Failed to select modpack file:", error)
      alert(`Failed to select file: ${error}`)
    }
  }

  const filteredModpacks = getFilteredModpacks()
  const totalPages = showInstalledOnly 
    ? Math.ceil(filteredModpacks.length / itemsPerPage)
    : searchResults ? Math.ceil(searchResults.total_hits / itemsPerPage) : 1
  const showPagination = showInstalledOnly 
    ? filteredModpacks.length > itemsPerPage
    : searchResults && searchResults.total_hits > itemsPerPage

  return (
    <div className="max-w-7xl mx-auto flex gap-4">
      <div className="flex-1 min-w-0">
        <div className="relative mb-4">
          <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-[#4a4a4a]" strokeWidth={2} />
          <input
            type="text"
            placeholder="Search modpacks..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-[#1a1a1a] rounded-lg pl-10 pr-4 py-2.5 text-sm text-[#e8e8e8] placeholder-[#4a4a4a] focus:outline-none focus:ring-2 focus:ring-[#2a2a2a] transition-all"
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
              {getPaginatedModpacks().map((modpack) => {
                const isInstalling = installingModpacks.has(modpack.project_id)
                const status = installationStatus[modpack.project_id]
                const progress = modpackProgress[modpack.title]
                const isLoadingVersionsForThis = loadingVersions.has(modpack.project_id)
                const gallery = modpackGalleries[modpack.project_id] || []
                const backgroundImage = gallery.length > 0 ? gallery[0] : modpack.icon_url
                const installed = isModpackInstalled(modpack.title)
                
                return (
                  <div
                    key={modpack.project_id}
                    className="relative bg-[#1a1a1a] rounded-2xl overflow-hidden transition-all group h-56"
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

                    <div className="relative h-full flex flex-col p-4">
                      <div className="mt-auto flex items-end justify-between gap-3">
                        <div className="flex items-center gap-2.5 flex-1 min-w-0">
                          <div className="w-14 h-14 rounded-xl overflow-hidden flex-shrink-0">
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
                            <h3 className="text-base font-bold text-white mb-0.5 truncate drop-shadow-lg">
                              {modpack.title}
                            </h3>
                            <p className="text-xs text-white/80 truncate drop-shadow-md">
                              by {modpack.author}
                            </p>
                            <p className="text-xs text-white/70 truncate drop-shadow-md mt-0.5">
                              {formatDownloads(modpack.downloads)} downloads
                            </p>
                          </div>
                        </div>

                        <button
                          onClick={() => {
                            loadModpackVersions(modpack)
                            handleInstallModpack(modpack)
                          }}
                          disabled={isInstalling || isLoadingVersionsForThis || installed}
                          className="flex-shrink-0 flex items-center justify-center transition-all cursor-pointer disabled:cursor-not-allowed"
                        >
                          {installed ? (
                            <CheckCircle size={28} className="text-green-500" strokeWidth={2} />
                          ) : status === 'success' ? (
                            <CheckCircle size={28} className="text-green-500" strokeWidth={2} />
                          ) : status === 'error' ? (
                            <AlertCircle size={28} className="text-red-500" strokeWidth={2} />
                          ) : isInstalling || isLoadingVersionsForThis ? (
                            <Loader2 size={28} className="text-white/60 animate-spin" strokeWidth={2} />
                          ) : (
                            <Download size={28} className="text-[#16a34a] hover:text-[#15803d]" strokeWidth={2} />
                          )}
                        </button>
                      </div>

                      {isInstalling && progress && (
                        <div className="mt-2.5">
                          <div className="flex items-center justify-between mb-1">
                            <span className="text-xs text-white font-medium truncate drop-shadow-md">{progress.stage}</span>
                            <span className="text-xs text-white/80 drop-shadow-md">{progress.progress}%</span>
                          </div>
                          <div className="w-full bg-black/40 rounded-full h-1.5 backdrop-blur-sm">
                            <div
                              className="bg-[#16a34a] h-1.5 rounded-full transition-all duration-300 shadow-lg"
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

      <div className="w-64 flex-shrink-0">
        <div className="sticky top-4">
          <div className="bg-[#1a1a1a] rounded-lg overflow-hidden">
            <div className="px-4 pt-3 pb-2">
              <h3 className="font-semibold text-[#e8e8e8] text-center">Filter by Version</h3>
            </div>
            
            <div className="p-3">
              {isLoadingVersions ? (
                <div className="flex items-center justify-center py-8">
                  <Loader2 size={20} className="animate-spin text-[#16a34a]" />
                </div>
              ) : availableVersions.length > 0 ? (
                <div className="max-h-[calc(100vh-240px)] overflow-y-auto pr-1">
                  <div className="grid grid-cols-2 gap-1.5">
                    {availableVersions.map((version) => (
                      <button
                        key={version}
                        onClick={() => onSetSelectedVersion(version)}
                        className={`px-2 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer ${
                          selectedVersion === version
                            ? "bg-[#16a34a] text-white"
                            : "bg-[#0d0d0d] text-[#808080] hover:bg-[#1f1f1f] hover:text-[#e8e8e8]"
                        }`}
                      >
                        {version}
                      </button>
                    ))}
                  </div>
                </div>
              ) : null}
            </div>
          </div>

          <div className="bg-[#1a1a1a] rounded-lg overflow-hidden mt-3">
            <div className="p-3">
              <button
                onClick={() => {
                  setShowInstalledOnly(!showInstalledOnly)
                  setCurrentPage(1)
                }}
                className={`w-full px-3 py-2 rounded-lg text-sm font-medium transition-all cursor-pointer ${
                  showInstalledOnly
                    ? "bg-[#16a34a] text-white"
                    : "bg-[#0d0d0d] text-[#808080] hover:bg-[#1f1f1f] hover:text-[#e8e8e8]"
                }`}
              >
                {showInstalledOnly ? "Show All Modpacks" : "Show Installed Only"}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}