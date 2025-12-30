import { useState, useRef } from "react"
import { ModsTab, ModsSelector } from "./ModsTab"
import { ModpacksTab } from "./ModpacksTab"
import { FolderDown } from "lucide-react"
import type { Instance } from "../../types"

interface BrowseTabProps {
  selectedInstance: Instance | null
  instances: Instance[]
  onSetSelectedInstance: (instance: Instance) => void
  onRefreshInstances?: () => void
  onShowCreationToast?: (instanceName: string) => void
}

declare global {
  interface Window {
    uploadModpackHandler?: () => Promise<void>
  }
}

export function BrowseTab({ 
  selectedInstance, 
  instances, 
  onSetSelectedInstance, 
  onRefreshInstances, 
  onShowCreationToast 
}: BrowseTabProps) {
  const [activeSubTab, setActiveSubTab] = useState<"mods" | "modpacks">("mods")
  const scrollContainerRef = useRef<HTMLDivElement>(null)  // ADD THIS

  const [selectedModpackVersion, setSelectedModpackVersion] = useState<string | null>(null)
  const [availableModpackVersions, setAvailableModpackVersions] = useState<string[]>([])
  const [isLoadingModpackVersions, setIsLoadingModpackVersions] = useState(false)

  const handleImport = async () => {
    console.log("handleImport called, onShowCreationToast is:", onShowCreationToast)
    if (window.uploadModpackHandler) {
      await window.uploadModpackHandler()
    } else {
      console.log("Upload handler not ready")
    }
  }

  return (
    <div className="p-6 space-y-4" ref={scrollContainerRef}>  {/* ADD ref HERE */}
      <div className="max-w-7xl mx-auto">
        <div className="flex items-center justify-between mb-4">
          <div>
            <div className="flex items-center gap-4">
              <button
                onClick={() => setActiveSubTab("mods")}
                className={`text-2xl font-semibold tracking-tight transition-colors cursor-pointer ${
                  activeSubTab === "mods" ? "text-[#e8e8e8]" : "text-[#4a4a4a] hover:text-[#808080]"
                }`}
              >
                Mods
              </button>
              <button
                onClick={() => setActiveSubTab("modpacks")}
                className={`text-2xl font-semibold tracking-tight transition-colors cursor-pointer ${
                  activeSubTab === "modpacks" ? "text-[#e8e8e8]" : "text-[#4a4a4a] hover:text-[#808080]"
                }`}
              >
                Modpacks
              </button>
            </div>
            <p className="text-sm text-[#808080] mt-0.5">
              {activeSubTab === "mods" ? "Browse and install mods from Modrinth" : "Browse and install modpacks from Modrinth"}
            </p>
          </div>
          
          {activeSubTab === "mods" ? (
            <ModsSelector 
              instances={instances}
              selectedInstance={selectedInstance}
              onSetSelectedInstance={onSetSelectedInstance}
            />
          ) : (
            <button
              onClick={handleImport}
              className="w-10 h-10 hover:bg-[#1a1a1a] text-[#e8e8e8] rounded-lg flex items-center justify-center transition-all cursor-pointer"
              title="Import modpack"
            >
              <FolderDown size={24} strokeWidth={2} />
            </button>
          )}
        </div>
      </div>

      {activeSubTab === "mods" ? (
        <ModsTab
          selectedInstance={selectedInstance}
          instances={instances}
          onSetSelectedInstance={onSetSelectedInstance}
          scrollContainerRef={scrollContainerRef}
        />
      ) : (
        <ModpacksTab
          instances={instances}
          onRefreshInstances={onRefreshInstances}
          selectedVersion={selectedModpackVersion}
          onSetSelectedVersion={setSelectedModpackVersion}
          availableVersions={availableModpackVersions}
          onSetAvailableVersions={setAvailableModpackVersions}
          isLoadingVersions={isLoadingModpackVersions}
          onSetIsLoadingVersions={setIsLoadingModpackVersions}
          onImport={handleImport}
          onShowCreationToast={onShowCreationToast}
          scrollContainerRef={scrollContainerRef}
        />
      )}
    </div>
  )
}