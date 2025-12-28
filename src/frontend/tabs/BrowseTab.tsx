import { useState } from "react"
import { ModsTab, ModsSelector } from "./ModsTab"
import { ModpacksTab, ModpackVersionSelector } from "./ModpacksTab"
import type { Instance } from "../../types"

interface BrowseTabProps {
  selectedInstance: Instance | null
  instances: Instance[]
  onSetSelectedInstance: (instance: Instance) => void
  onRefreshInstances?: () => void
}

export function BrowseTab({ selectedInstance, instances, onSetSelectedInstance, onRefreshInstances }: BrowseTabProps) {
  const [activeSubTab, setActiveSubTab] = useState<"mods" | "modpacks">("mods")
  
  // State for modpack version selector
  const [selectedModpackVersion, setSelectedModpackVersion] = useState<string | null>(null)
  const [availableModpackVersions, setAvailableModpackVersions] = useState<string[]>([])
  const [isLoadingModpackVersions, setIsLoadingModpackVersions] = useState(false)

  return (
    <div className="p-6 space-y-4">
      <div className="max-w-7xl mx-auto">
        <div className="flex items-start justify-between mb-4">
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
            <ModpackVersionSelector
              selectedVersion={selectedModpackVersion}
              onSetSelectedVersion={setSelectedModpackVersion}
              versions={availableModpackVersions}
              isLoading={isLoadingModpackVersions}
            />
          )}
        </div>
      </div>

      {activeSubTab === "mods" ? (
        <ModsTab
          selectedInstance={selectedInstance}
          instances={instances}
          onSetSelectedInstance={onSetSelectedInstance}
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
        />
      )}
    </div>
  )
}