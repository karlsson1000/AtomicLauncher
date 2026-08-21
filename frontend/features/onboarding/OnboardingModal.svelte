<script lang="ts">
  import { Loader2, CheckCircle2, XCircle, ArrowRight, ArrowLeft } from "lucide-svelte"
  import { onMount } from "svelte"
  import { fade } from "svelte/transition"
  import { invoke } from "@tauri-apps/api/core"
  import type { ImportableInstance } from "../../types"
  import { formatFileSize } from "../../lib/format"
  import {
    store,
    loadAccounts,
    loadInstances,
    setShowOnboarding,
    generateUniqueName,
  } from "../../lib/launcherStore.svelte"

  let step = $state<"welcome" | "import" | "done">("welcome")
  let detected = $state<ImportableInstance[]>([])
  let isDetecting = $state(true)
  let detectFailed = $state(false)
  let isSigningIn = $state(false)
  let isImporting = $state(false)
  let importedCount = $state(0)

  let selected = $state<Record<string, boolean>>({})
  let itemStatus = $state<Record<string, "importing" | "done" | "error">>({})

  const sourceLabels: Record<string, string> = {
    prism: "Prism",
    modrinth: "Modrinth",
    curseforge: "CurseForge",
  }

  const sourceColors: Record<string, string> = {
    prism: "#f472b6",
    modrinth: "#16a34a",
    curseforge: "#f97316",
  }

  onMount(() => {
    detect()
  })

  async function detect() {
    isDetecting = true
    detectFailed = false
    try {
      detected = await invoke<ImportableInstance[]>("detect_importable_instances")
      const nextSelected: Record<string, boolean> = {}
      for (const item of detected) {
        nextSelected[item.path] = !!item.mc_version
      }
      selected = nextSelected
    } catch {
      detectFailed = true
    } finally {
      isDetecting = false
    }
  }

  function getSelectedItems(): ImportableInstance[] {
    return detected.filter(d => selected[d.path] && d.mc_version)
  }

  async function handleSignIn() {
    isSigningIn = true
    try {
      await invoke("microsoft_login_and_store")
      await loadAccounts()
      if (store.activeAccount) step = "import"
    } catch {}
    isSigningIn = false
  }

  let usedNames = new Set<string>()

  function uniqueTargetName(base: string): string {
    let candidate = generateUniqueName(base)
    let n = 2
    while (usedNames.has(candidate)) {
      candidate = `${base} (${n++})`
    }
    usedNames.add(candidate)
    return candidate
  }

  async function handleImport() {
    const items = getSelectedItems()
    if (items.length === 0) return

    isImporting = true
    usedNames = new Set(store.instances.map(i => i.name))
    importedCount = 0

    for (const item of items) {
      const targetName = uniqueTargetName(item.name)
      itemStatus[item.path] = "importing"
      store.creatingInstanceName = targetName

      try {
        await invoke("import_instance", {
          source: item.source,
          sourcePath: item.path,
          name: item.name,
          targetName,
          mcVersion: item.mc_version,
          loader: item.loader,
          loaderVersion: item.loader_version,
          iconPath: item.icon_path,
        })
        itemStatus[item.path] = "done"
        importedCount++
      } catch (error) {
        console.error(`Failed to import ${item.name}:`, error)
        itemStatus[item.path] = "error"
      }
    }

    await loadInstances()
    store.creatingInstanceName = null
    isImporting = false
    step = "done"
  }

  function finish() {
    setShowOnboarding(false)
  }

  function skipStep() {
    if (step === "welcome") step = "import"
    else if (step === "import") step = "done"
  }

  let selectedCount = $derived(getSelectedItems().length)

  function instanceMeta(item: ImportableInstance): string {
    if (!item.mc_version) return "Unknown version"
    const parts = [`Minecraft ${item.mc_version}`]
    if (item.loader && item.loader !== "vanilla") {
      parts.push(`${item.loader.charAt(0).toUpperCase() + item.loader.slice(1)}${item.loader_version ? ` ${item.loader_version}` : ""}`)
    }
    parts.push(formatFileSize(item.size_bytes))
    return parts.join("  ·  ")
  }
</script>

<div transition:fade={{ duration: 150 }} class="fixed inset-0 z-50 bg-[var(--bg-primary)] flex flex-col overflow-hidden">
  <header class="flex items-center justify-between px-8 py-6 flex-shrink-0">
    <div class="flex items-center gap-2">
      <img src="/logo.png" alt="Octane" class="h-5 w-5" />
      <span class="text-sm font-semibold text-[var(--text-secondary)]">Octane Launcher</span>
    </div>
    <div class="flex items-center gap-1.5">
      {#each ["welcome", "import", "done"] as s (s)}
        <div
          class="h-1 rounded-full transition-all duration-300 {step === s ? 'w-6 bg-[var(--accent-primary)]' : 'w-2 bg-[var(--bg-hover)]'}"
        ></div>
      {/each}
    </div>
  </header>

  <main class="flex-1 min-h-0 flex items-center justify-center px-8 pb-16">
    <div class="w-full max-w-lg">
      {#if step === "welcome"}
        <div class="text-center">
          <h1 class="text-3xl font-semibold text-[var(--text-primary)] tracking-tight">Welcome to Octane</h1>
          <p class="mt-3 text-sm text-[var(--text-muted)] leading-relaxed">
            Sign in with Microsoft to play with friends, or continue without an account.
            You can sign in any time.
          </p>

          <button
            onclick={handleSignIn}
            disabled={isSigningIn}
            class="mt-10 mx-auto w-64 px-4 py-1.5 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] disabled:opacity-60 text-white text-sm font-medium transition-colors cursor-pointer flex items-center justify-center gap-2"
          >
            {#if isSigningIn}
              <Loader2 size={15} class="animate-spin" />
              Waiting for browser...
            {:else}
              Sign in with Microsoft
            {/if}
          </button>
        </div>
      {:else if step === "import"}
        <div>
          <h1 class="text-2xl font-semibold text-[var(--text-primary)] tracking-tight text-center">
            {#if isDetecting}
              Looking for your instances
            {:else if detected.length > 0}
              Found {detected.length} {detected.length === 1 ? "instance" : "instances"}
            {:else}
              Nothing found
            {/if}
          </h1>
          <p class="mt-2 text-sm text-[var(--text-muted)] text-center leading-relaxed">
            {#if detectFailed}
              Couldn't scan this PC. You can create instances manually instead.
            {:else if detected.length > 0}
              From Modrinth, Prism and CurseForge. Pick what to bring over, nothing is moved or deleted.
            {:else}
              We looked for Modrinth, Prism and CurseForge installations on this PC.
            {/if}
          </p>

          {#if isDetecting}
            <div class="mt-12 flex justify-center">
              <Loader2 size={22} class="animate-spin text-[var(--accent-primary)]" />
            </div>
          {:else if detected.length > 0}
            <div class="mt-8 max-h-[45vh] overflow-y-auto custom-scrollbar -mx-2 px-2 space-y-0.5">
              {#each detected as item (item.path)}
                <label
                  class="flex items-center gap-3 px-3 py-2.5 rounded-md transition-colors {!item.mc_version || isImporting
                    ? 'opacity-40'
                    : 'cursor-pointer hover:bg-[var(--bg-secondary)]'}"
                >
                  <input
                    type="checkbox"
                    bind:checked={selected[item.path]}
                    disabled={!item.mc_version || isImporting}
                    class="accent-[#16a34a] w-4 h-4 flex-shrink-0"
                  />
                  <div class="min-w-0 flex-1">
                    <div class="text-sm font-medium text-[var(--text-primary)] truncate">{item.name}</div>
                    <div class="text-xs text-[var(--text-muted)] truncate mt-0.5">{instanceMeta(item)}</div>
                  </div>
                  <span class="flex items-center gap-1.5 text-xs text-[var(--text-muted)] flex-shrink-0">
                    <span class="w-1.5 h-1.5 rounded-full" style="background: {sourceColors[item.source]}"></span>
                    {sourceLabels[item.source]}
                  </span>
                  {#if itemStatus[item.path] === "importing"}
                    <Loader2 size={14} class="animate-spin text-[var(--accent-primary)] flex-shrink-0" />
                  {:else if itemStatus[item.path] === "done"}
                    <CheckCircle2 size={14} class="text-[#16a34a] flex-shrink-0" />
                  {:else if itemStatus[item.path] === "error"}
                    <XCircle size={14} class="text-red-500 flex-shrink-0" />
                  {/if}
                </label>
              {/each}
            </div>
          {/if}

          <div class="mt-10 flex items-center justify-between">
            <button
              onclick={() => (step = "welcome")}
              disabled={isImporting}
              class="flex items-center gap-1.5 text-sm text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors cursor-pointer disabled:opacity-40"
            >
              <ArrowLeft size={14} />
              Back
            </button>
            <div class="flex items-center gap-4">
              {#if detected.length > 0 && !detectFailed}
                <button
                  onclick={handleImport}
                  disabled={selectedCount === 0 || isImporting}
                  class="px-5 py-1.5 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] disabled:opacity-30 disabled:cursor-not-allowed text-white text-sm font-medium transition-colors cursor-pointer flex items-center gap-2"
                >
                  {#if isImporting}
                    <Loader2 size={14} class="animate-spin" />
                    Importing...
                  {:else}
                    Import{selectedCount > 0 ? ` ${selectedCount}` : ""}
                  {/if}
                </button>
              {:else}
                <button
                  onclick={() => (step = "done")}
                  class="flex items-center gap-1.5 px-5 py-1.5 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white text-sm font-medium transition-colors cursor-pointer"
                >
                  Get started
                  <ArrowRight size={14} />
                </button>
              {/if}
            </div>
          </div>
        </div>
      {:else}
        <div class="text-center">
          <CheckCircle2 size={36} class="mx-auto text-[#16a34a]" strokeWidth={1.5} />
          <h1 class="mt-6 text-2xl font-semibold text-[var(--text-primary)] tracking-tight">You're all set</h1>
          <p class="mt-3 text-sm text-[var(--text-muted)] leading-relaxed">
            {#if importedCount > 0}
              Imported {importedCount} {importedCount === 1 ? "instance" : "instances"} from your other launchers.
            {:else}
              Create your first instance from the Instances tab, or browse mods in the Addons tab.
            {/if}
          </p>
          <button
            onclick={finish}
            class="mt-10 px-8 py-1.5 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white text-sm font-medium transition-colors cursor-pointer"
          >
            Get started
          </button>
        </div>
      {/if}
    </div>
  </main>

  {#if step !== "done"}
    <button
      onclick={skipStep}
      disabled={isImporting}
      class="fixed bottom-6 right-8 text-sm text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors cursor-pointer disabled:opacity-40"
    >
      Skip
    </button>
  {/if}
</div>