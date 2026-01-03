import { useState, useEffect, useRef } from "react"
import { invoke } from "@tauri-apps/api/core"
import { open } from '@tauri-apps/plugin-dialog'
import { Search, Download, Loader2, Package, ChevronLeft, ChevronRight, CheckCircle, AlertCircle, X } from "lucide-react"
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

const CUSTOM_MODPACK_SLUG = "stellarmc-enhanced"
const CUSTOM_MODPACK_AUTHOR = "StellarMC"

export function ModpacksTab({ 
  instances, 
  onRefreshInstances,
  selectedVersion,
  onSetAvailableVersions,
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
  const [installationStatus, setInstallationStatus] = useState<Record<string, 'success' | 'error'>>({})
  const [modpackVersions, setModpackVersions] = useState<Record<string, ModrinthVersion[]>>({})
  const [loadingVersions, setLoadingVersions] = useState<Set<string>>(new Set())
  const [modpackGalleries, setModpackGalleries] = useState<Record<string, string[]>>({})
  const [showInstalledOnly, setShowInstalledOnly] = useState(false)
  const [customModpack, setCustomModpack] = useState<ModrinthProject | null>(null)
  
  const [selectedModpack, setSelectedModpack] = useState<ModrinthProject | null>(null)
  const [selectedModpackVersion, setSelectedModpackVersion] = useState<string>("")

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
        idOrSlug: CUSTOM_MODPACK_SLUG,
      })

      const modpackData: ModrinthProject = {
        project_id: projectDetails.id,
        slug: projectDetails.slug,
        title: projectDetails.title,
        description: projectDetails.description,
        author: CUSTOM_MODPACK_AUTHOR,
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
      console.error("Failed to load custom modpack:", error)
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

    const shouldShowCustomModpack = currentPage === 1 && !searchQuery.trim() && customModpack
    
    if (shouldShowCustomModpack) {
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

  const openModpackModal = async (modpack: ModrinthProject) => {
    setSelectedModpack(modpack)
    const projectId = modpack.project_id
    
    if (!modpackVersions[projectId]) {
      setLoadingVersions(prev => new Set(prev).add(projectId))
      try {
        const versions = await invoke<ModrinthVersion[]>("get_modpack_versions", {
          idOrSlug: modpack.slug,
          gameVersion: selectedVersion,
        })
        setModpackVersions(prev => ({ ...prev, [projectId]: versions }))
        if (versions.length > 0) {
          setSelectedModpackVersion(versions[0].id)
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
    } else {
      setSelectedModpackVersion(modpackVersions[projectId][0]?.id || "")
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

  const handleInstallModpack = async () => {
    if (!selectedModpack) return
    
    try {
      const projectId = selectedModpack.project_id
      const versions = modpackVersions[projectId]

      if (!versions || versions.length === 0) {
        alert("No versions available for this modpack")
        return
      }

      const versionId = selectedModpackVersion || versions[0].id
      const instanceName = selectedModpack.title

      const existingInstance = instances.find(i => i.name === instanceName)
      const finalName = existingInstance ? `${instanceName}-${Date.now()}` : instanceName
      
      setInstallingModpacks(prev => new Set(prev).add(selectedModpack.project_id))
      setSelectedModpack(null)
      
      console.log("Installing modpack:", {
        slug: selectedModpack.slug,
        instanceName: finalName,
        versionId: versionId,
        gameVersion: selectedVersion
      })
      
      if (onShowCreationToast) {
        onShowCreationToast(finalName)
      }
      
      await invoke("install_modpack", {
        modpackSlug: selectedModpack.slug,
        instanceName: finalName,
        versionId: versionId,
        preferredGameVersion: selectedVersion,
      })
      
      setInstallationStatus(prev => ({ ...prev, [selectedModpack.project_id]: 'success' }))

      if (onRefreshInstances) {
        setTimeout(() => {
          onRefreshInstances()
        }, 500)
      }

      setTimeout(() => {
        setInstallingModpacks(prev => {
          const newSet = new Set(prev)
          newSet.delete(projectId)
          return newSet
        })
        setInstallationStatus(prev => {
          const newStatus = { ...prev }
          delete newStatus[projectId]
          return newStatus
        })
      }, 3000)

    } catch (error) {
      console.error("Failed to install modpack:", error)
      if (selectedModpack) {
        setInstallationStatus(prev => ({ ...prev, [selectedModpack.project_id]: 'error' }))
        
        setInstallingModpacks(prev => {
          const newSet = new Set(prev)
          newSet.delete(selectedModpack.project_id)
          return newSet
        })

        setTimeout(() => {
          setInstallationStatus(prev => {
            const newStatus = { ...prev }
            delete newStatus[selectedModpack.project_id]
            return newStatus
          })
        }, 5000)
      }
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
        const modpackName = await invoke<string>("get_modpack_name_from_file", {
          filePath: filePath
        })
        
        console.log("Extracted modpack name:", modpackName)
        
        let finalName = modpackName
        const existingInstance = instances.find(
          i => i.name.toLowerCase() === modpackName.toLowerCase()
        )
        
        if (existingInstance) {
          finalName = `${modpackName}-${Date.now()}`
          console.log("Instance exists, using name:", finalName)
        }
        
        if (onShowCreationToast) {
          onShowCreationToast(finalName)
        }
        
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
    <div className="max-w-7xl mx-auto">
      <div className="flex items-center gap-3 mb-4">
        <div className="relative flex-1">
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

        <button
          onClick={() => {
            setShowInstalledOnly(!showInstalledOnly)
            setCurrentPage(1)
          }}
          className={`px-4 py-2.5 rounded-lg text-sm font-medium transition-all cursor-pointer whitespace-nowrap ${
            showInstalledOnly
              ? "bg-[#16a34a] text-white"
              : "bg-[#1a1a1a] text-[#808080] hover:bg-[#1f1f1f] hover:text-[#e8e8e8]"
          }`}
        >
          {showInstalledOnly ? "Show All" : "Installed Only"}
        </button>
      </div>

      {searchResults && (
        <>
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {getPaginatedModpacks().map((modpack) => {
              const isInstalling = installingModpacks.has(modpack.project_id)
              const status = installationStatus[modpack.project_id]
              const gallery = modpackGalleries[modpack.project_id] || []
              const backgroundImage = gallery.length > 0 ? gallery[0] : null
              const installed = isModpackInstalled(modpack.title)
              
              return (
                <div
                  key={modpack.project_id}
                  className="relative bg-[#1a1a1a] rounded-xl overflow-hidden transition-all group"
                >
                  <div className="relative h-48 overflow-hidden">
                    {backgroundImage ? (
                      <img
                        src={backgroundImage}
                        alt={modpack.title}
                        className="w-full h-full object-cover"
                      />
                    ) : (
                      <div className="w-full h-full bg-gradient-to-br from-[#1a1a1a] via-[#141414] to-[#0f0f0f] flex items-center justify-center">
                        <Package size={64} className="text-[#3a3a3a]" strokeWidth={1.5} />
                      </div>
                    )}
                    <div className="absolute inset-0 bg-gradient-to-t from-black/80 via-black/20 to-transparent" />
                  </div>

                  <div className="relative bg-[#1a1a1a] p-4 flex items-center gap-4">
                    <div className="w-14 h-14 rounded-xl overflow-hidden flex-shrink-0">
                      {modpack.icon_url ? (
                        <img
                          src={modpack.icon_url}
                          alt={modpack.title}
                          className="w-full h-full object-cover"
                        />
                      ) : (
                        <div className="w-full h-full bg-[#1a1a1a] flex items-center justify-center">
                          <Package size={28} className="text-[#4a4a4a]" strokeWidth={1.5} />
                        </div>
                      )}
                    </div>

                    <div className="flex-1 min-w-0">
                      <h3 className="text-lg font-bold text-white leading-tight truncate">
                        {modpack.title}
                      </h3>
                      <p className="text-sm text-[#808080] leading-tight">
                        by {modpack.author}
                      </p>
                      <p className="text-sm text-[#4a4a4a] leading-tight">
                        {formatDownloads(modpack.downloads)} downloads
                      </p>
                    </div>

                    {installed ? (
                      <div className="flex-shrink-0 px-4 py-2 rounded-lg font-medium text-sm bg-[#16a34a] text-white">
                        <span className="flex items-center gap-1.5">
                          <CheckCircle size={16} />
                          Installed
                        </span>
                      </div>
                    ) : status === 'success' ? (
                      <div className="flex-shrink-0 px-4 py-2 rounded-lg font-medium text-sm bg-[#16a34a] text-white">
                        <span className="flex items-center gap-1.5">
                          <CheckCircle size={16} />
                          Success
                        </span>
                      </div>
                    ) : status === 'error' ? (
                      <div className="flex-shrink-0 px-4 py-2 rounded-lg font-medium text-sm bg-red-600 text-white">
                        <span className="flex items-center gap-1.5">
                          <AlertCircle size={16} />
                          Error
                        </span>
                      </div>
                    ) : isInstalling ? (
                      <div className="flex-shrink-0 px-4 py-2 rounded-lg font-medium text-sm bg-[#2a2a2a] text-[#808080]">
                        <span className="flex items-center gap-1.5">
                          <Loader2 size={16} className="animate-spin" />
                          Installing
                        </span>
                      </div>
                    ) : (
                      <button
                        onClick={(e) => {
                          e.stopPropagation()
                          openModpackModal(modpack)
                        }}
                        className="flex-shrink-0 px-4 py-2 rounded-lg font-medium text-sm bg-[#16a34a] hover:bg-[#15803d] text-white transition-colors cursor-pointer"
                      >
                        <span className="flex items-center gap-1.5">
                          <Download size={16} />
                          Install
                        </span>
                      </button>
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

      {selectedModpack && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4" onClick={() => setSelectedModpack(null)}>
          <div className="bg-[#1a1a1a] rounded-xl w-full max-w-2xl overflow-hidden" onClick={(e) => e.stopPropagation()}>
            <div className="relative h-64">
              {modpackGalleries[selectedModpack.project_id]?.[0] ? (
                <img
                  src={modpackGalleries[selectedModpack.project_id][0]}
                  alt={selectedModpack.title}
                  className="w-full h-full object-cover"
                />
              ) : (
                <div className="w-full h-full bg-gradient-to-br from-[#1a1a1a] via-[#141414] to-[#0f0f0f] flex items-center justify-center">
                  <Package size={80} className="text-[#3a3a3a]" strokeWidth={1.5} />
                </div>
              )}
              <div className="absolute inset-0 bg-gradient-to-t from-black/90 via-black/30 to-transparent" />
              <button
                onClick={() => setSelectedModpack(null)}
                className="absolute top-4 right-4 w-8 h-8 bg-black/50 hover:bg-black/70 rounded-lg flex items-center justify-center transition-colors cursor-pointer"
              >
                <X size={20} className="text-white" />
              </button>
            </div>

            <div className="p-6 space-y-4">
              <div className="flex items-start gap-4">
                <div className="w-16 h-16 rounded-xl overflow-hidden flex-shrink-0">
                  {selectedModpack.icon_url ? (
                    <img src={selectedModpack.icon_url} alt={selectedModpack.title} className="w-full h-full object-cover" />
                  ) : (
                    <div className="w-full h-full bg-[#0f0f0f] flex items-center justify-center">
                      <Package size={32} className="text-[#4a4a4a]" strokeWidth={1.5} />
                    </div>
                  )}
                </div>

                <div className="flex-1 min-w-0">
                  <h2 className="text-xl font-bold text-white leading-tight">{selectedModpack.title}</h2>
                  <p className="text-sm text-[#808080] leading-tight">by {selectedModpack.author}</p>
                  <p className="text-sm text-[#4a4a4a] leading-tight">
                    {formatDownloads(selectedModpack.downloads)} downloads
                  </p>
                </div>
              </div>

              {selectedModpack.description && (
                <p className="text-sm text-[#b0b0b0] leading-relaxed">{selectedModpack.description}</p>
              )}

              <div className="space-y-4">
                <label className="text-sm font-medium text-[#e8e8e8]">Select Version</label>
                {loadingVersions.has(selectedModpack.project_id) ? (
                  <div className="flex items-center justify-center py-2">
                    <Loader2 size={24} className="animate-spin text-[#16a34a]" />
                  </div>
                ) : modpackVersions[selectedModpack.project_id]?.length > 0 ? (
                  <div className="relative">
                    <select
                      value={selectedModpackVersion}
                      onChange={(e) => setSelectedModpackVersion(e.target.value)}
                      className="w-full bg-[#0f0f0f] text-[#e8e8e8] rounded-lg px-4 py-2.5 pr-10 text-sm focus:outline-none focus:ring-2 focus:ring-[#2a2a2a] cursor-pointer appearance-none"
                    >
                      {modpackVersions[selectedModpack.project_id].map((version) => (
                        <option key={version.id} value={version.id}>
                          {version.name} - {version.game_versions?.join(', ')}
                        </option>
                      ))}
                    </select>
                    <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">
                      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#808080" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <polyline points="6 9 12 15 18 9"></polyline>
                      </svg>
                    </div>
                  </div>
                ) : (
                  <p className="text-sm text-[#808080] py-3">No versions available</p>
                )}
              </div>

              <div className="flex justify-end gap-3">
                <button
                  onClick={() => setSelectedModpack(null)}
                  className="px-8 py-2.5 bg-[#0f0f0f] hover:bg-[#1f1f1f] text-[#e8e8e8] rounded-lg font-medium text-sm transition-colors cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  onClick={handleInstallModpack}
                  disabled={!selectedModpackVersion || loadingVersions.has(selectedModpack.project_id)}
                  className="px-8 py-2.5 bg-[#16a34a] hover:bg-[#15803d] disabled:bg-[#2a2a2a] disabled:cursor-not-allowed text-white rounded-lg font-medium text-sm transition-colors cursor-pointer flex items-center justify-center gap-2"
                >
                  <Download size={18} />
                  Install
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}