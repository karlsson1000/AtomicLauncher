<script lang="ts">
  import { Loader2, CheckCircle, XCircle } from "lucide-svelte"
  import { listen } from "@tauri-apps/api/event"
  import { showToast } from "../../lib/toastStore.svelte"

  interface ProgressPayload {
    instance: string
    progress: number
    stage?: string
    current_file?: string
  }

  type ProgressKind = "creation" | "duplication" | "modpack"

  const KIND_LABELS: Record<ProgressKind, { done: string; failed: string }> = {
    creation: { done: "created", failed: `Failed to create` },
    duplication: { done: "duplicated", failed: `Failed to duplicate` },
    modpack: { done: "installed", failed: `Failed to install` },
  }

  let { instanceName, onError, onDismiss }: {
    instanceName: string
    onError: () => void
    onDismiss: () => void
  } = $props()

  let progress = $state(0)
  let stage = $state("")
  let status = $state<"creating" | "success" | "error">("creating")
  let kind = $state<ProgressKind>("creation")
  let hasReceivedProgress = $state(false)

  let isCompleting = false
  let errorTimer: ReturnType<typeof setInterval> | undefined
  let lastProgressTime = Date.now()
  let unlistenFns: Array<() => void> = []

  $effect(() => {
    if (status === "success" || status === "error") {
      const t = setTimeout(() => onDismiss(), 3000)
      return () => clearTimeout(t)
    }
  })

  $effect(() => {
    const setupListeners = async () => {
      try {
        const makeHandler = (progressKind: ProgressKind) => (e: { payload: ProgressPayload }) => {
          if (e.payload.instance !== instanceName) return
          hasReceivedProgress = true
          lastProgressTime = Date.now()
          progress = e.payload.progress
          if (e.payload.stage) stage = e.payload.stage
          kind = progressKind
          if (e.payload.progress >= 100 && !isCompleting) {
            isCompleting = true
            status = "success"
            showToast("success", `${instanceName} ${KIND_LABELS[progressKind].done}`)
          }
        }

        const [u1, u2, u3] = await Promise.all([
          listen<ProgressPayload>("duplication-progress", makeHandler("duplication")),
          listen<ProgressPayload>("creation-progress", makeHandler("creation")),
          listen<ProgressPayload>("modpack-install-progress", makeHandler("modpack")),
        ])

        unlistenFns = [u1, u2, u3]

        errorTimer = setInterval(() => {
          if (!isCompleting && Date.now() - lastProgressTime > 60000) {
            clearInterval(errorTimer)
            status = "error"
            stage = "Operation timed out"
            showToast("error", `${KIND_LABELS[kind].failed} ${instanceName}`)
            onError()
          }
        }, 5000)
      } catch {
        status = "error"
        stage = "Failed to initialize"
        showToast("error", `Failed to start ${instanceName}`)
        onError()
      }
    }

    setupListeners()

    return () => {
      if (errorTimer) clearInterval(errorTimer)
      unlistenFns.forEach((fn) => { try { fn() } catch {} })
      unlistenFns = []
    }
  })

  let label = $derived(
    stage ||
    (status === "success"
      ? "Complete"
      : status === "error"
        ? "Failed"
        : hasReceivedProgress
          ? "Processing…"
          : "Starting…")
  )

  let barColor = $derived(
    status === "error"
      ? "bg-red-500"
      : "bg-[#16a34a]"
  )
</script>

<div>
  <div class="flex items-center justify-between px-4 py-1.5 bg-[var(--bg-secondary)]">
    <div class="flex items-center gap-2 min-w-0">
      {#if status === "creating"}
        <Loader2 size={12} class="animate-spin text-[#16a34a] flex-shrink-0" />
      {:else if status === "success"}
        <CheckCircle size={12} class="text-[#16a34a] flex-shrink-0" />
      {:else if status === "error"}
        <XCircle size={12} class="text-red-500 flex-shrink-0" />
      {/if}
      <span class="text-xs text-[var(--text-secondary)] truncate">
        <span class="text-[var(--text-primary)]">{instanceName}</span>
        {" - "}
        {label}
      </span>
    </div>
    <span class="text-xs text-[var(--text-secondary)] flex-shrink-0 ml-4 tabular-nums">
      {Math.round(progress)}%
    </span>
  </div>

  <div class="h-[3px] bg-[var(--bg-tertiary)] w-full">
    <div
      class="h-full transition-all duration-300 ease-out {barColor}"
      style="width: {progress}%"
    ></div>
  </div>
</div>
