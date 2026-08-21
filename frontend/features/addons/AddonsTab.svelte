<script lang="ts">
  import { Search, Check, Puzzle, Layers, Image, Sparkles, Package } from "lucide-svelte"
  import { invoke } from "@tauri-apps/api/core"
  import AddonList from "./AddonList.svelte"
  import ProjectDetail from "./ProjectDetail.svelte"
  import { ADDON_CATEGORIES, type ContentSource } from "./adapters"
  import {
    store, setSelectedInstance, loadInstances,
    handleStartCreating, setAddonsSubTab
  } from "../../lib/launcherStore.svelte"
  import type { Instance } from "../../types"
  import { getMinecraftVersion } from "../../lib/version"

  let viewDetail = $state<{
    source: "modrinth" | "curseforge"
    projectId: string
    projectSlug?: string
    projectType: string
    author?: string
  } | null>(null)

  let contentSource = $state<ContentSource>("modrinth")
  let showSourceDropdown = $state(false)
  let searchQuery = $state("")
  let instanceIcons = $state<Record<string, string | null>>({})
  const moddedInstances = $derived(store.instances.filter(i => i.loader === "fabric" || i.loader === "neoforge" || i.loader === "forge"))

  const tabs = [
    { id: "mods" as const, label: "Mods", icon: Puzzle, color: "text-[#16a34a]" },
    { id: "modpacks" as const, label: "Modpacks", icon: Layers, color: "text-[#3b82f6]" },
    { id: "resourcepacks" as const, label: "Resource Packs", icon: Image, color: "text-[#8b5cf6]" },
    { id: "shaderpacks" as const, label: "Shader Packs", icon: Sparkles, color: "text-[#f59e0b]" },
  ]

  $effect(() => {
    if (store.instances.length === 0) return
    const loadIcons = async () => {
      const icons: Record<string, string | null> = {}
      for (const instance of store.instances) {
        try {
          icons[instance.name] = await invoke<string | null>("get_instance_icon", { instanceName: instance.name })
        } catch {
          icons[instance.name] = null
        }
      }
      instanceIcons = icons
    }
    loadIcons()
  })

  const getLoaderDisplay = (instance: Instance): { name: string; color: string } => {
    if (instance.loader === "fabric") return { name: "Fabric", color: "text-[#3b82f6]" }
    if (instance.loader === "neoforge") return { name: "NeoForge", color: "text-[#f97316]" }
    if (instance.loader === "forge") return { name: "Forge", color: "text-[#e05d2e]" }
    return { name: "Vanilla", color: "text-[#16a34a]" }
  }

  function handleViewProjectDetail(
    source: "modrinth" | "curseforge",
    projectId: string,
    projectSlug: string | undefined,
    projectType: string,
    author?: string,
  ) {
    viewDetail = { source, projectId, projectSlug, projectType, author }
  }
</script>

<div class="flex flex-col h-full overflow-hidden">
  <div class="flex-shrink-0 px-8 pt-8 pb-4">
    <div class="max-w-7xl mx-auto">
      <div class="flex items-center gap-4">
        {#each tabs as tab, index}
          {@const Icon = tab.icon}
          {@const isActive = store.addonsSubTab === tab.id}
          <button
            onclick={() => { setAddonsSubTab(tab.id); viewDetail = null }}
            class="flex items-center gap-2 text-2xl font-semibold tracking-tight transition-colors cursor-pointer {isActive ? tab.color : 'text-[var(--text-muted)] hover:text-[var(--text-primary)]'}"
          >
            <Icon size={24} strokeWidth={2} />
            <span>{tab.label}</span>
          </button>
          {#if index < tabs.length - 1}
            <div class="h-8 w-px bg-[var(--bg-hover-strong)]"></div>
          {/if}
        {/each}
      </div>
    </div>
  </div>

  <div class="flex-shrink-0 px-8 pb-4">
    <div class="max-w-7xl mx-auto">
      <div class="flex gap-2 items-stretch">
        <div class="relative">
          <button
            onclick={() => showSourceDropdown = !showSourceDropdown}
            class="w-10 h-10 flex items-center justify-center bg-[var(--bg-tertiary)] hover:bg-[var(--bg-hover)] rounded-md transition-colors cursor-pointer"
          >
            <img
              src={contentSource === "modrinth" ? "/modrinth.svg" : "/curseforge.svg"}
              alt={contentSource}
              class="w-6 h-6"
            />
          </button>
          {#if showSourceDropdown}
            <div class="absolute top-full mt-2 left-0 bg-[var(--bg-tertiary)] rounded-md overflow-hidden z-50 min-w-[140px] shadow-lg">
              <button
                onclick={() => { contentSource = "modrinth"; showSourceDropdown = false }}
                class="w-full flex items-center gap-2 px-3 py-2 text-sm transition-colors cursor-pointer {contentSource === 'modrinth' ? 'bg-[#16a34a]/10 text-[var(--text-primary)]' : 'text-[var(--text-muted)] hover:bg-[var(--bg-hover)]'}"
              >
                <img src="/modrinth.svg" alt="Modrinth" class="w-6 h-6" />
                Modrinth
              </button>
              <button
                onclick={() => { contentSource = "curseforge"; showSourceDropdown = false }}
                class="w-full flex items-center gap-2 px-3 py-2 text-sm transition-colors cursor-pointer {contentSource === 'curseforge' ? 'bg-[#f97316]/20 text-[var(--text-primary)]' : 'text-[var(--text-muted)] hover:bg-[var(--bg-hover)]'}"
              >
                <img src="/curseforge.svg" alt="CurseForge" class="w-6 h-6" />
                CurseForge
              </button>
            </div>
          {/if}
        </div>

        <div class="relative flex-1 rounded-md bg-[var(--bg-tertiary)]">
          <Search size={16} class="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--text-muted)] z-20 pointer-events-none" strokeWidth={2} />
          <input
            type="text"
            placeholder={ADDON_CATEGORIES[store.addonsSubTab].placeholder}
            bind:value={searchQuery}
            class="w-full bg-transparent rounded-md pl-10 pr-4 py-2.5 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none transition-all relative z-10"
          />
        </div>

      </div>
    </div>
  </div>

  <div class="flex-1 min-h-0 px-8 overflow-hidden">
    <div class="h-full max-w-7xl mx-auto grid grid-cols-1 {viewDetail ? '' : 'lg:grid-cols-11 gap-2'}">
      <div class="{viewDetail ? '' : 'lg:col-span-8'} overflow-y-auto">
        {#if viewDetail}
          <ProjectDetail
            source={viewDetail.source}
            projectId={viewDetail.projectId}
            projectSlug={viewDetail.projectSlug}
            projectType={viewDetail.projectType}
            author={viewDetail.author}
            selectedInstance={store.selectedInstance}
            instances={store.instances}
            onShowCreationToast={handleStartCreating}
            onRefreshInstances={loadInstances}
            onBack={() => viewDetail = null}
          />
        {:else}
          {#key `${contentSource}:${store.addonsSubTab}`}
            <AddonList
              source={contentSource}
              category={ADDON_CATEGORIES[store.addonsSubTab]}
              searchQuery={searchQuery}
              onViewProjectDetail={handleViewProjectDetail}
              selectedInstance={store.selectedInstance}
              instances={store.instances}
              onSetSelectedInstance={setSelectedInstance}
            />
          {/key}
        {/if}
      </div>

      <!-- Right: Instance list sidebar -->
      {#if !viewDetail}
        <div class="lg:col-span-3 overflow-y-auto hidden lg:block">
          <div class="flex flex-col gap-3">
            <h2 class="text-lg font-semibold text-[var(--text-primary)] px-3">Install content to</h2>
            <div class="space-y-1.5">
              {#if moddedInstances.length === 0}
                <p class="text-xs text-[#3a3f4b]">No modded instances</p>
              {:else}
                {#each moddedInstances as instance}
                  {@const icon = instanceIcons[instance.name]}
                  {@const loader = getLoaderDisplay(instance)}
                  <button
                    onclick={() => setSelectedInstance(instance)}
                    class="w-full flex items-center gap-3 px-3 py-1.5 text-left text-sm rounded-md transition-colors cursor-pointer {store.selectedInstance?.name === instance.name ? 'bg-[var(--bg-tertiary)]' : 'hover:bg-[var(--bg-tertiary)]'}"
                  >
                    {#if icon}
                      <img src={icon} alt={instance.name} class="w-11 h-11 rounded object-cover flex-shrink-0" />
                    {:else}
                      <div class="w-11 h-11 flex items-center justify-center flex-shrink-0">
                        <Package size={28} class="text-[var(--text-muted)]" strokeWidth={1.5} />
                      </div>
                    {/if}
                    <div class="flex-1 min-w-0">
                      <div class="font-medium text-base text-[var(--text-primary)] truncate leading-tight">{instance.name}</div>
                      <div class="text-xs text-[var(--text-muted)] truncate mt-0.5">
                        {getMinecraftVersion(instance)} <span class="text-[#3a3f4b]">•</span> <span class={loader.color}>{loader.name}</span>
                      </div>
                    </div>
                    {#if store.selectedInstance?.name === instance.name}
                      <Check size={18} class="flex-shrink-0 text-[#16a34a]" strokeWidth={3} />
                    {/if}
                  </button>
                {/each}
              {/if}
            </div>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>
