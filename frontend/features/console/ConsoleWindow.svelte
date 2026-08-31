<script lang="ts">
  import { listen } from "@tauri-apps/api/event"
  import { invoke } from "@tauri-apps/api/core"
  import { getCurrentWindow } from "@tauri-apps/api/window"
  import { Minus, Square, X, SquareTerminal, Trash2, Upload, ExternalLink, Loader2, X as XIcon } from "lucide-svelte"
  import type { ConsoleLog } from "../../types"

  let consoleLogs = $state<ConsoleLog[]>([])
  let activeInstance = $state<string | null>(null)
  let uploadState = $state<{
    loading: boolean
    url: string | null
    error: string | null
  }>({ loading: false, url: null, error: null })
  let logTimestamps = $state(new Map<ConsoleLog, string>())
  function getLogTime(log: ConsoleLog): string {
    if (!logTimestamps.has(log)) {
      logTimestamps.set(log, new Date().toLocaleTimeString())
    }
    return logTimestamps.get(log)!
  }

  let consoleEndEl: HTMLDivElement | undefined = $state()
  let containerEl: HTMLDivElement | undefined = $state()
  let previousInstances: string[] = []
  let scrollPositions: Record<string, number> = {}
  const appWindow = getCurrentWindow()

  $effect(() => {
    const unlisten = listen<ConsoleLog>("console-log", (event) => {
      consoleLogs = [...consoleLogs, event.payload]
    })
    return () => { unlisten.then((fn) => fn()) }
  })

  let instanceLogs = $derived(consoleLogs.reduce((acc, log) => {
    if (!acc[log.instance]) acc[log.instance] = []
    acc[log.instance].push(log)
    return acc
  }, {} as Record<string, ConsoleLog[]>))

  let instances = $derived(Object.keys(instanceLogs).sort())

  $effect(() => {
    const prev = previousInstances
    const newInstances = instances.filter(i => !prev.includes(i))

    if (newInstances.length > 0) {
      activeInstance = newInstances[newInstances.length - 1]
    } else if (instances.length > 0 && !activeInstance) {
      activeInstance = instances[instances.length - 1]
    } else if (activeInstance && !instances.includes(activeInstance)) {
      activeInstance = instances[instances.length - 1] || null
    }

    previousInstances = instances
  })

  $effect(() => {
    consoleLogs.length
    if (consoleEndEl) {
      consoleEndEl.scrollIntoView({ behavior: "smooth" })
    }
  })

  $effect(() => {
    if (containerEl && activeInstance) {
      const saved = scrollPositions[activeInstance]
      if (saved !== undefined) {
        containerEl.scrollTop = saved
      } else {
        containerEl.scrollTop = containerEl.scrollHeight
      }
    }
  })

  function handleScroll() {
    if (containerEl && activeInstance) {
      scrollPositions[activeInstance] = containerEl.scrollTop
    }
  }

  let activeLogs = $derived(activeInstance ? instanceLogs[activeInstance] || [] : [])

  function formatLogs() {
    if (!activeInstance) return ""
    return activeLogs.map(log =>
      `${getLogTime(log)} [${log.instance}] ${log.message}`
    ).join('\n')
  }

  async function handleUploadToMcLogs() {
    uploadState = { loading: true, url: null, error: null }
    try {
      const logContent = formatLogs()
      if (!logContent.trim()) {
        uploadState = { loading: false, url: null, error: "No logs to upload" }
        setTimeout(() => uploadState = { loading: false, url: null, error: null }, 3000)
        return
      }
      const formData = new URLSearchParams()
      formData.append('content', logContent)
      const response = await fetch('https://api.mclo.gs/1/log', {
        method: 'POST',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body: formData
      })
      const data = await response.json()
      if (data.success) {
        await navigator.clipboard.writeText(data.url)
        invoke('open_url', { url: data.url }).catch(() => {})
        uploadState = { loading: false, url: data.url, error: null }
        setTimeout(() => uploadState = { loading: false, url: null, error: null }, 3000)
      } else {
        uploadState = { loading: false, url: null, error: data.error || "Upload failed" }
        setTimeout(() => uploadState = { loading: false, url: null, error: null }, 3000)
      }
    } catch {
      uploadState = { loading: false, url: null, error: "Network error occurred" }
      setTimeout(() => uploadState = { loading: false, url: null, error: null }, 3000)
    }
  }

  function handleClear() {
    if (!activeInstance) return
    consoleLogs = consoleLogs.filter(log => log.instance !== activeInstance)
  }
</script>

<div class="flex flex-col h-screen bg-[var(--bg-primary)] text-[var(--text-primary)] font-sans overflow-hidden">
  <div
    data-tauri-drag-region
    class="h-9 flex-shrink-0 flex items-center pl-4 pr-2 gap-2 select-none"
    style="-webkit-app-region: drag"
  >
    <SquareTerminal size={16} strokeWidth={2} class="text-[var(--accent-primary)] shrink-0" />
    <span class="text-sm font-medium text-[var(--text-secondary)] flex-1">
      {activeInstance ? `Console - ${activeInstance}` : "Console - Octane Launcher"}
    </span>

    <div class="flex items-center" style="-webkit-app-region: no-drag">
      <button
        onclick={() => appWindow.minimize()}
        class="h-9 w-9 flex items-center justify-center text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors cursor-pointer"
      >
        <Minus size={18} strokeWidth={3} />
      </button>
      <button
        onclick={() => appWindow.toggleMaximize()}
        class="h-9 w-9 flex items-center justify-center text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors cursor-pointer"
      >
        <Square size={14} strokeWidth={3} />
      </button>
      <button
        onclick={() => appWindow.close()}
        class="h-9 w-9 flex items-center justify-center text-[var(--text-secondary)] hover:text-red-500 transition-colors cursor-pointer"
      >
        <X size={18} strokeWidth={3} />
      </button>
    </div>
  </div>

  {#if instances.length > 1}
    <div class="flex items-center gap-1 px-3 pb-2 flex-shrink-0" style="-webkit-app-region: no-drag">
      <div class="flex items-center gap-1.5 bg-[var(--bg-secondary)] rounded-lg p-1">
        {#each instances as instance (instance)}
          <button
            onclick={() => activeInstance = instance}
            class="px-3 py-1.5 rounded-md text-sm font-medium whitespace-nowrap transition-all cursor-pointer {activeInstance === instance ? 'bg-[var(--accent-primary)] text-white' : 'text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)]'}"
          >
            {instance}
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <div class="flex-1 mx-3 bg-[var(--bg-tertiary)] rounded-lg overflow-hidden">
    {#if consoleLogs.length === 0}
      <div class="h-full flex items-center justify-center">
        <div class="text-center">
          <SquareTerminal size={40} class="text-[var(--accent-primary)] mx-auto mb-3" strokeWidth={1.5} />
          <p class="text-sm text-[var(--text-primary)] mb-1">Waiting for output...</p>
          <p class="text-xs text-[var(--text-muted)]">Logs will appear here when an instance launches</p>
        </div>
      </div>
    {:else}
      <div
        class="h-full overflow-y-auto p-4 font-mono text-sm"
        bind:this={containerEl}
        onscroll={handleScroll}
      >
        {#each activeLogs as log}
          {@const isError = log.type === "stderr" || log.message.toLowerCase().includes("error") || log.message.toLowerCase().includes("failed")}
          {@const isWarning = log.message.toLowerCase().includes("warning") || log.message.toLowerCase().includes("warn")}
          <div
            class="py-0.5 leading-relaxed {isError ? 'text-red-400' : isWarning ? 'text-yellow-400' : 'text-[var(--text-primary)]'}"
          >
            <span class="text-[var(--text-muted)] mr-2">{getLogTime(log)}</span>
            <span class="text-[#16a34a] mr-2">[{log.instance}]</span>
            <span>{log.message}</span>
          </div>
        {/each}
        <div bind:this={consoleEndEl}></div>
      </div>
    {/if}
  </div>

  <div class="flex items-center justify-end gap-2 px-3 py-3 flex-shrink-0" style="-webkit-app-region: no-drag">
    <button
      onclick={handleUploadToMcLogs}
      disabled={!activeInstance || activeLogs.length === 0 || uploadState.loading}
      class="px-4 py-2 rounded-md font-medium text-sm flex items-center gap-2 transition-all cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed {uploadState.url ? 'bg-[#16a34a]/10 hover:bg-[#16a34a]/20 text-[#16a34a]' : uploadState.error ? 'bg-red-500/10 hover:bg-red-500/20 text-red-400' : 'bg-[var(--bg-tertiary)] hover:bg-[var(--bg-hover-strong)] text-[var(--text-primary)]'}"
    >
      {#if uploadState.loading}
        <Loader2 size={16} strokeWidth={2} class="animate-spin" />
      {:else if uploadState.url}
        <ExternalLink size={16} strokeWidth={2} />
      {:else if uploadState.error}
        <XIcon size={16} strokeWidth={2} />
      {:else}
        <Upload size={16} strokeWidth={2} />
      {/if}
      <span>
        {#if uploadState.loading}
          Uploading...
        {:else if uploadState.url}
          Uploaded & Copied!
        {:else if uploadState.error}
          {uploadState.error}
        {:else}
          Upload to mclo.gs
        {/if}
      </span>
    </button>
    <button
      onclick={handleClear}
      disabled={!activeInstance || activeLogs.length === 0}
      class="px-4 py-2 bg-[var(--bg-tertiary)] hover:bg-[var(--bg-hover-strong)] text-[var(--text-primary)] rounded-md font-medium text-sm flex items-center gap-2 transition-all cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
    >
      <Trash2 size={16} strokeWidth={2} />
      <span>Clear {activeInstance || "Console"}</span>
    </button>
  </div>
</div>
