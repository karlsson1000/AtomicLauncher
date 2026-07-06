<script lang="ts">
  import { Terminal, Trash2, Upload, ExternalLink, Loader2, X, PictureInPicture } from "lucide-svelte"
  import { invoke } from "@tauri-apps/api/core"
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow"
  import { store, handleClearConsole } from "../../lib/launcherStore.svelte"
  import type { ConsoleLog } from "../../types"

  const ASCII = `
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣠⠴⠒⠒⠒⠲⢤⣄
⠀⠀⠀⠀⠀⠀⣀⡠⠤⠤⠤⠤⠤⠤⠔⠋⠁⠀⠀⠀⠀⠀⠀⠈⢧⡀
⠀⠀⠀⠀⢀⡾⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⢷
⠀⠀⠀⢠⠏⠀⠀⠀⠀⠀⠀⠀⠀⠀⣦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⡇
⠀⡔⠒⡿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠁⠀⠀⠀⠀⠀⠀⢀⢄⣴⣮⣼⠸⡄
⠘⣴⣿⣷⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣰⢣⣿⣟⣿⣿⡏⡇
⠀⠀⠉⠳⣄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣧⣾⣿⣿⣿⣿⡇⡇
⠀⠀⠀⠀⠀⠉⠒⠠⢄⣀⠀⠀⠀⠀⠀⠀⠀⠀⢸⢻⣻⣿⣿⣿⢿⡹⠁
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠲⣄⠀⠀⠀⣠⠔⠛⣎⡻⠿⢿⣫⠞⠁
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢰⣿⣿⣿⣿⠁⠀⠀⠀⠉⠉⠉
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⡴⠃⠀⠈⠉⢻⠁
⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⠞⠀⠀⠀⡄⠂⠈⡇
⠀⠀⠀⠀⠀⠀⠀⠀⢠⡏⠀⠀⠀⠸⠀⠀⠀⣿
⠀⠀⠀⠀⠀⠀⠀⠀⣾⠁⠀⠀⠀⡇⠀⠀⠘⡟⡇
⠀⠀⠀⠀⠀⠀⠀⠀⠻⣦⠀⠀⠀⣇⠀⢠⣸⣷⣧⢀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠙⠲⠀⠈⡉⠉⠀⡏⠣⠬⠭⠤⠄
⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⣀⣰⠁⠀⡇⠀⠀⣇
⠀⠀⠀⠀⠀⠀⡔⢊⢉⠤⠒⠛⠒⠲⠃⠀⠀⠻⣄
⠀⠀⠀⠀⠀⠀⠧⣧⣧⢀⡔⠀⡄⠀⠀⠀⠀⠀⠀⣱
⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠑⠚⠛⠒⠓⠚⠈⠊⠉⠁
`;

  let consoleEndEl = $state<HTMLDivElement>()
  let containerEl = $state<HTMLDivElement>()
  let uploadState = $state<{ loading: boolean; url: string | null; error: string | null }>({ loading: false, url: null, error: null })
  let isPopedOut = $state(false)
  let activeInstance = $state<string | null>(null)
  let previousInstances: string[] = []
  let scrollPositions: Record<string, number> = {}

  let instances = $derived([...new Set(store.consoleLogs.map(l => l.instance))].sort())

  let activeLogs = $derived(activeInstance ? store.consoleLogs.filter(l => l.instance === activeInstance) : [])

  $effect(() => {
    let unlisten: (() => void) | undefined

    WebviewWindow.getByLabel("console").then((win) => {
      if (win) {
        isPopedOut = true
        win.once("tauri://destroyed", () => { isPopedOut = false }).then((fn) => {
          unlisten = fn
        })
      }
    })

    return () => { unlisten?.() }
  })

  $effect(() => {
    const newInstances = instances.filter(i => !previousInstances.includes(i))

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
    store.consoleLogs.length;
    if (consoleEndEl && activeInstance && containerEl) {
      consoleEndEl.scrollIntoView({ behavior: "smooth" })
    }
  })

  $effect(() => {
    if (containerEl && activeInstance) {
      const savedPosition = scrollPositions[activeInstance]
      if (savedPosition !== undefined) {
        containerEl.scrollTop = savedPosition
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

  let logTimestamps = $state(new Map<ConsoleLog, string>())
  function getLogTime(log: ConsoleLog): string {
    if (!logTimestamps.has(log)) {
      logTimestamps.set(log, new Date().toLocaleTimeString())
    }
    return logTimestamps.get(log)!
  }

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
        setTimeout(() => { uploadState = { loading: false, url: null, error: null } }, 3000)
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
        setTimeout(() => { uploadState = { loading: false, url: null, error: null } }, 3000)
      } else {
        uploadState = { loading: false, url: null, error: data.error || "Upload failed" }
        setTimeout(() => { uploadState = { loading: false, url: null, error: null } }, 3000)
      }
    } catch (err) {
      console.error('Failed to upload logs:', err)
      uploadState = { loading: false, url: null, error: "Network error occurred" }
      setTimeout(() => { uploadState = { loading: false, url: null, error: null } }, 3000)
    }
  }

  function onClear() {
    if (activeInstance) {
      handleClearConsole(activeInstance)
    }
  }

  async function handlePopOut() {
    const existing = await WebviewWindow.getByLabel("console")
    if (existing) {
      await existing.show()
      await existing.setFocus()
      isPopedOut = true
      return
    }

    const win = new WebviewWindow("console", {
      url: "console.html",
      title: "Console",
      width: 600,
      height: 800,
      minWidth: 600,
      minHeight: 400,
      decorations: false,
      center: true,
    })

    win.once("tauri://created", () => {
      isPopedOut = true
    })

    win.once("tauri://error", (e) => {
      console.error("Failed to open console window:", e)
      isPopedOut = false
    })

    win.once("tauri://destroyed", () => {
      isPopedOut = false
    })
  }
</script>

<div class="p-8 space-y-4">
  <div class="max-w-7xl mx-auto">
    <div class="mb-4">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-semibold text-[var(--text-primary)] tracking-tight">Console</h1>
          <p class="text-sm text-[var(--text-muted)] mt-0.5">View game output and logs</p>
        </div>

        <div class="flex items-center gap-2">
          {#if !isPopedOut && instances.length > 0}
            <div class="flex items-center gap-1.5 bg-[var(--bg-secondary)] rounded-lg p-1">
              {#each instances as instance}
                <button
                  onclick={() => activeInstance = instance}
                  class="px-3 py-1.5 rounded-md text-sm font-medium whitespace-nowrap transition-all cursor-pointer {activeInstance === instance ? 'bg-[var(--accent-primary)] text-white' : 'text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)]'}"
                >
                  {instance}
                </button>
              {/each}
            </div>
          {/if}

          {#if !isPopedOut}
            <button
              onclick={handlePopOut}
              title="Open in new window"
              class="h-10 w-10 flex items-center justify-center rounded-md transition-colors cursor-pointer text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)]"
            >
              <PictureInPicture size={24} strokeWidth={2} />
            </button>
          {/if}
        </div>
      </div>
    </div>

    <div class="bg-[var(--bg-tertiary)] rounded-lg overflow-hidden" style="height: calc(100vh - 250px)">
      {#if isPopedOut}
        <div class="h-full flex items-center justify-center">
          <div class="text-center">
            <PictureInPicture size={48} class="text-[var(--accent-primary)] mx-auto mb-3" strokeWidth={1.5} />
            <p class="text-base font-medium text-[var(--text-primary)] mb-1">Console detached</p>
            <p class="text-sm text-[var(--text-muted)]">Logs are showing in the separate window</p>
          </div>
        </div>
      {:else if store.consoleLogs.length === 0}
        <div class="h-full flex items-center justify-center">
          <div class="text-center">
            <pre class="font-mono text-xs text-[var(--accent-primary)] select-none text-left">{ASCII}</pre>
            <p class="text-base text-[var(--text-primary)] mt-4 mb-1">No console output yet</p>
            <p class="text-sm text-[var(--text-muted)]">Launch an instance to see logs</p>
          </div>
        </div>
      {:else if !activeInstance}
        <div class="h-full flex items-center justify-center">
          <div class="text-center">
            <Terminal size={48} class="text-[var(--text-muted)] mx-auto mb-3" strokeWidth={1.5} />
            <p class="text-base text-[var(--text-primary)] mb-1">No instance selected</p>
            <p class="text-sm text-[var(--text-muted)]">Select an instance tab above</p>
          </div>
        </div>
      {:else}
        <div class="h-full overflow-y-auto p-4 font-mono text-sm" bind:this={containerEl} onscroll={handleScroll}>
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

    {#if !isPopedOut}
      <div class="flex items-center justify-end gap-2 mt-4">
        <button
          onclick={handleUploadToMcLogs}
          disabled={!activeInstance || activeLogs.length === 0 || uploadState.loading}
          class="px-4 py-2 rounded-md font-medium text-sm flex items-center gap-2 transition-all cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed {uploadState.url ? 'bg-[#16a34a]/10 hover:bg-[#16a34a]/20 text-[#16a34a]' : uploadState.error ? 'bg-red-500/10 hover:bg-red-500/20 text-red-400' : 'bg-[var(--bg-tertiary)] hover:bg-[var(--bg-hover-strong)] text-[var(--text-primary)]'}"
          title="Upload logs to mclo.gs"
        >
          {#if uploadState.loading}
            <Loader2 size={16} strokeWidth={2} class="animate-spin" />
          {:else if uploadState.url}
            <ExternalLink size={16} strokeWidth={2} />
          {:else if uploadState.error}
            <X size={16} strokeWidth={2} />
          {:else}
            <Upload size={16} strokeWidth={2} />
          {/if}
          <span>
            {uploadState.loading ? "Uploading..." : uploadState.url ? "Uploaded & Copied!" : uploadState.error ? uploadState.error : "Upload to mclo.gs"}
          </span>
        </button>
        <button
          onclick={onClear}
          disabled={!activeInstance || activeLogs.length === 0}
          class="px-4 py-2 bg-[var(--bg-tertiary)] hover:bg-[var(--bg-hover-strong)] text-[var(--text-primary)] rounded-md font-medium text-sm flex items-center gap-2 transition-all cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <Trash2 size={16} strokeWidth={2} />
          <span>Clear {activeInstance || "Console"}</span>
        </button>
      </div>
    {/if}
  </div>
</div>
