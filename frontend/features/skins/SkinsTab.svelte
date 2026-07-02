<script lang="ts">
  import { invoke } from "@tauri-apps/api/core"
  import { Upload, Loader2, User, RotateCcw, Save, Plane, RectangleVertical } from "lucide-svelte"
  import * as skinview3d from "skinview3d"
  import type { RecentSkin, Cape } from "../../types"
  import { storeGet, storeSet, storeRemove } from "../../lib/store"
  import { store } from "../../lib/launcherStore.svelte"
  import { untrack } from "svelte"

  const SKIN_CACHE_KEY = "octane_skin_cache"
  const CAPE_CACHE_KEY = "octane_cape_cache"
  const CACHE_DURATION = 15 * 60 * 1000
  const MIN_FETCH_INTERVAL = 10 * 1000

  const CAPE_IMAGE_MAP: Record<string, string> = {
    "migrator": "migrator",
    "pan": "pan",
    "15th anniversary": "15th",
    "common": "common",
    "vanilla": "vanilla",
    "cherry blossom": "cherry",
    "purple heart": "twitch",
    "follower's": "tiktok",
    "menace": "menace",
    "copper": "copper",
    "home": "home",
    "mojang office": "mojangoffice",
    "yearn": "yearn",
    "founders": "founders",
    "zombie horse": "zombiehorse",
    "mcc 15th year": "mcc",
    "builder": "builder",
    "minecraft experience": "mcexp",
    "minecon 2016": "2016",
    "minecon 2015": "2015",
    "minecon 2013": "2013",
    "minecon 2012": "2012",
    "crafter": "crafter",
    "minecon 2011": "2011",
    "moonlight trail": "moonlighttrail",
    "realms mapmaker": "realms",
    "mojang": "mojang",
    "mojang studios": "mojangstudios",
  }

  function loadCache<T>(key: string): Promise<T | null> {
    return storeGet<T>(key).then(v => v ?? null)
  }

  function saveCache<T>(key: string, data: T) {
    return storeSet(key, data)
  }

  const canFetch = (last: number) => Date.now() - last >= MIN_FETCH_INTERVAL

  type PersistedSkinCache = { uuid: string; url: string; variant: string; timestamp: number }
  type PersistedCapeCache = { uuid: string; capes: Cape[]; activeCapeId: string | null; timestamp: number }

  let loading = $state(true)
  let error = $state<string | null>(null)
  let uploading = $state(false)
  let skinVariant = $state<"classic" | "slim">("classic")
  let capes = $state<Cape[]>([])
  let activeCape = $state<string | null>(null)
  let loadingCapes = $state(false)
  let showElytra = $state(false)
  let recentSkins = $state<RecentSkin[]>([])
  let currentSkinUrl = $state<string | null>(null)
  let containerSize = $state({ width: 800, height: 600 })
  let hasPendingChanges = $state(false)
  let saving = $state(false)
  let hoveredCapeId = $state<string | null>(null)
  let hoveredRecentIdx = $state<number | null>(null)
  let elytraHovered = $state(false)
  let uploadTooltipHover = $state(false)
  let resetTooltipHover = $state(false)

  let containerEl: HTMLDivElement | undefined = $state()
  let canvasEl: HTMLCanvasElement | undefined = $state()
  let fileInputEl: HTMLInputElement | undefined = $state()

  let viewer = $state<skinview3d.SkinViewer | null>(null)
  let skinResetTimeout: ReturnType<typeof setTimeout> | undefined
  let dragState = { active: false, startX: 0, startRot: 0.3 }
  let lastProfileFetch = 0
  let lastCapeFetch: Record<string, number> = {}
  let originalState = { skinUrl: null as string | null, variant: 'classic' as 'classic' | 'slim', activeCape: null as string | null }
  let pendingSkinOp: { type: 'upload'; base64: string; variant: 'classic' | 'slim' } | { type: 'recent'; url: string; variant: 'classic' | 'slim' } | { type: 'reset' } | null = null
  let pendingCapeOp: { type: 'select'; capeId: string } | { type: 'remove' } | null = null
  let loadUserSkinFn: (forceRefresh?: boolean) => Promise<void> = async () => {}

  let prevSkinUrl: string | undefined
  $effect(() => {
    store.activeAccount
    untrack(() => {
      if (prevSkinUrl?.startsWith('blob:')) URL.revokeObjectURL(prevSkinUrl)
      prevSkinUrl = currentSkinUrl ?? undefined
      pendingSkinOp = null
      pendingCapeOp = null
      hasPendingChanges = false
    })
  })

  $effect(() => {
    if (!store.activeAccount) { recentSkins = []; return }
    loadRecentSkins()
  })

  $effect(() => { loadUserSkin() })

  async function loadRecentSkins() {
    if (!store.activeAccount) return
    try {
      const result = await invoke<RecentSkin[]>("load_recent_skins", { accountUuid: store.activeAccount.uuid })
      recentSkins = Array.isArray(result) ? result : []
    } catch { recentSkins = [] }
  }

  async function addToRecentSkins(url: string, variant: "classic" | "slim") {
    if (!store.activeAccount) return
    recentSkins = [{ url, variant, timestamp: Date.now() }, ...recentSkins.filter(s => s.url !== url)].slice(0, 3)
    try { await invoke<void>("save_recent_skin", { accountUuid: store.activeAccount.uuid, skinUrl: url, variant }) }
    catch { console.error("Failed to save recent skin") }
  }

  function applyUploadedSkin(result: { url: string; variant: string }): "classic" | "slim" | null {
    if (!result?.url) return null
    const variant: "classic" | "slim" = result.variant === "slim" ? "slim" : "classic"
    skinVariant = variant
    currentSkinUrl = result.url
    if (store.activeAccount) {
      saveCache<PersistedSkinCache>(SKIN_CACHE_KEY, {
        uuid: store.activeAccount.uuid, url: result.url, variant, timestamp: Date.now(),
      })
    }
    return variant
  }

  function getCapeImageName(alias: string) { return CAPE_IMAGE_MAP[alias.toLowerCase()] ?? "unknown" }

  async function loadUserSkin(forceRefresh = false) {
    if (!store.isAuthenticated || !store.activeAccount || !invoke) {
      loading = false; error = "Please sign in to view your skin"; return
    }
    try {
      loading = true; error = null
      const now = Date.now()
      const persisted = await loadCache<PersistedSkinCache>(SKIN_CACHE_KEY)
      const cacheValid =
        persisted && persisted.uuid === store.activeAccount.uuid && (now - persisted.timestamp) < CACHE_DURATION

      const applyCached = () => {
        if (!cacheValid) return
        currentSkinUrl = persisted!.url
        skinVariant = persisted!.variant as "classic" | "slim"
        originalState = { ...originalState, skinUrl: persisted!.url, variant: persisted!.variant as "classic" | "slim" }
      }

      if (!forceRefresh && cacheValid) { applyCached(); loading = false; loadCapes(); return }
      if (!canFetch(lastProfileFetch)) { applyCached(); loading = false; loadCapes(); return }

      lastProfileFetch = now
      const skinData = await invoke<{ url: string; variant: string }>("get_current_skin")
      if (skinData?.url) {
        const variant = skinData.variant === "slim" ? "slim" : "classic"
        currentSkinUrl = skinData.url
        skinVariant = variant
        originalState = { ...originalState, skinUrl: skinData.url, variant }
        await saveCache<PersistedSkinCache>(SKIN_CACHE_KEY, {
          uuid: store.activeAccount.uuid, url: skinData.url, variant, timestamp: now,
        })
      } else {
        currentSkinUrl = null
      }
      loading = false; loadCapes()
    } catch (err: unknown) {
      if (typeof err === "string" && err.includes("429")) {
        error = "Rate limited — please wait before refreshing."
        const persisted = await loadCache<PersistedSkinCache>(SKIN_CACHE_KEY)
        if (persisted && persisted.uuid === store.activeAccount?.uuid) {
          currentSkinUrl = persisted.url
          skinVariant = persisted.variant as "classic" | "slim"
          originalState = { ...originalState, skinUrl: persisted.url, variant: persisted.variant as "classic" | "slim" }
        }
      } else {
        error = `Failed to load skin: ${String(err)}`; currentSkinUrl = null
      }
      loading = false
    }
  }
  loadUserSkinFn = loadUserSkin

  async function loadCapes() {
    if (!store.isAuthenticated || !store.activeAccount) return
    const now = Date.now()
    const persisted = await loadCache<PersistedCapeCache>(CAPE_CACHE_KEY)
    const cacheValid =
      persisted && persisted.uuid === store.activeAccount.uuid && (now - persisted.timestamp) < CACHE_DURATION
    if (cacheValid) {
      capes = persisted!.capes; activeCape = persisted!.activeCapeId
      originalState = { ...originalState, activeCape: persisted!.activeCapeId }
      return
    }

    capes = []; activeCape = null
    if (!canFetch(lastCapeFetch[store.activeAccount.uuid] ?? 0)) return
    try {
      loadingCapes = true
      lastCapeFetch[store.activeAccount.uuid] = now
      const capeData = await invoke<{ capes: Cape[] }>("get_user_capes")
      if (capeData?.capes) {
        capes = capeData.capes
        const activeId = capeData.capes.find((c: Cape) => c.state === "ACTIVE")?.id ?? null
        activeCape = activeId
        originalState = { ...originalState, activeCape: activeId }
        await saveCache<PersistedCapeCache>(CAPE_CACHE_KEY, {
          uuid: store.activeAccount.uuid, capes: capeData.capes, activeCapeId: activeId, timestamp: now,
        })
      }
    } catch { console.error("Failed to load capes") } finally { loadingCapes = false }
  }

  function handleCapeSelect(capeId: string) {
    activeCape = capeId
    pendingCapeOp = { type: 'select', capeId }
    hasPendingChanges = true
  }

  function handleCapeRemove() {
    activeCape = null
    pendingCapeOp = { type: 'remove' }
    hasPendingChanges = true
  }

  function handleFileSelect(event: Event) {
    const input = event.target as HTMLInputElement
    const file = input.files?.[0]
    if (!file) return
    const localUrl = URL.createObjectURL(file)
    uploading = true
    currentSkinUrl = localUrl
    const reader = new FileReader()
    reader.onload = (e) => {
      const base64 = ((e.target?.result) as string).split(",")[1]
      pendingSkinOp = { type: 'upload', base64, variant: skinVariant }
      hasPendingChanges = true
      uploading = false
    }
    reader.readAsDataURL(file)
  }

  function handleReset() {
    pendingSkinOp = { type: 'reset' }
    hasPendingChanges = true
  }

  function handleRecentSkinSelect(skin: RecentSkin) {
    currentSkinUrl = skin.url
    skinVariant = skin.variant
    pendingSkinOp = { type: 'recent', url: skin.url, variant: skin.variant }
    hasPendingChanges = true
  }

  async function handleSave() {
    saving = true; error = null
    let finalSkinUrl = currentSkinUrl
    let finalVariant = skinVariant
    let finalCape = activeCape
    try {
      const skinOp = pendingSkinOp
      if (skinOp) {
        if (skinOp.type === 'upload') {
          const result = await invoke<{ url: string; variant: string }>('upload_skin', { skinData: skinOp.base64, variant: skinVariant })
          const variant = applyUploadedSkin(result)
          finalSkinUrl = result.url
          finalVariant = variant ?? finalVariant
          if (variant) await addToRecentSkins(result.url, variant)
        } else if (skinOp.type === 'recent') {
          const response = await fetch(skinOp.url)
          const blob = await response.blob()
          const base64 = await new Promise<string>((resolve, reject) => {
            const reader = new FileReader()
            reader.onload = () => resolve((reader.result as string).split(',')[1])
            reader.onerror = reject
            reader.readAsDataURL(blob)
          })
          const result = await invoke<{ url: string; variant: string }>('upload_skin', { skinData: base64, variant: skinVariant })
          const variant = applyUploadedSkin(result)
          finalSkinUrl = result.url
          finalVariant = variant ?? finalVariant
          if (variant) await addToRecentSkins(result.url, variant)
        } else if (skinOp.type === 'reset') {
          await invoke<void>('reset_skin')
          await storeRemove(SKIN_CACHE_KEY)
          skinResetTimeout = setTimeout(() => loadUserSkinFn(false), 2000)
        }
      }

      const capeOp = pendingCapeOp
      if (capeOp) {
        if (capeOp.type === 'select') {
          await invoke<void>('equip_cape', { capeId: capeOp.capeId })
          finalCape = capeOp.capeId
        } else if (capeOp.type === 'remove') {
          await invoke<void>('remove_cape')
          finalCape = null
        }
        if (store.activeAccount) {
          const cached = await loadCache<PersistedCapeCache>(CAPE_CACHE_KEY)
          if (cached?.uuid === store.activeAccount.uuid) {
            await saveCache(CAPE_CACHE_KEY, { ...cached, activeCapeId: finalCape, timestamp: Date.now() })
          }
        }
      }

      if (currentSkinUrl?.startsWith('blob:')) URL.revokeObjectURL(currentSkinUrl)
      originalState = { skinUrl: finalSkinUrl, variant: finalVariant, activeCape: finalCape }
      pendingSkinOp = null
      pendingCapeOp = null
      hasPendingChanges = false
    } catch (err) {
      error = `Save failed: ${err}`
    } finally {
      saving = false
    }
  }

  function handleDiscard() {
    if (currentSkinUrl?.startsWith('blob:')) URL.revokeObjectURL(currentSkinUrl)
    pendingSkinOp = null
    pendingCapeOp = null
    currentSkinUrl = originalState.skinUrl
    skinVariant = originalState.variant
    activeCape = originalState.activeCape
    hasPendingChanges = false
    error = null
  }

  let activeCapeObj = $derived(capes.find(c => c.id === activeCape) ?? null)
  let activeCapeUrl = $derived<string | null>(activeCapeObj?.url ?? null)

  // SkinViewer3D initialization
  $effect(() => {
    const canvas = canvasEl
    const container = containerEl
    if (!canvas || !container) return

    const { width: cw, height: ch } = container.getBoundingClientRect()
    const v = new skinview3d.SkinViewer({ canvas, width: Math.round(cw), height: Math.round(ch) })

    v.renderer.setClearColor(0x000000, 0)
    v.globalLight.intensity = 3.0
    v.cameraLight.intensity = 0.0
    v.animation = new skinview3d.IdleAnimation()
    v.animation.speed = 1
    v.controls.enabled = false
    v.fov = 70
    v.zoom = 0.6
    v.playerObject.rotation.y = 0.3
    v.camera.position.y += 8
    v.camera.lookAt(v.playerObject.position)

    const ro = new ResizeObserver(entries => {
      const { width, height } = entries[0].contentRect
      containerSize = { width: Math.round(width), height: Math.round(height) }
    })
    ro.observe(container)

    viewer = v

    return () => {
      ro.disconnect()
      v.dispose()
      viewer = null
    }
  })

  $effect(() => {
    const v = viewer
    if (!v) return
    const { width, height } = containerSize
    if (width < 1 || height < 1) return
    v.setSize(width, height)
    v.render()
  })

  $effect(() => {
    const v = viewer
    if (!v) return
    if (currentSkinUrl) {
      v.loadSkin(currentSkinUrl, { model: skinVariant === "slim" ? "slim" : "default" })
    } else {
      v.loadSkin(null)
    }
  })

  $effect(() => {
    const v = viewer
    if (!v) return
    if (activeCapeUrl) {
      v.loadCape(activeCapeUrl, showElytra ? { backEquipment: "elytra" } : undefined)
    } else {
      v.loadCape(null)
    }
  })

  $effect(() => {
    const timeout = skinResetTimeout
    return () => clearTimeout(timeout)
  })

  function handlePointerDown(e: PointerEvent) {
    if ((e.target as HTMLElement).closest("button, input")) return
    dragState = {
      active: true,
      startX: e.clientX,
      startRot: viewer?.playerObject.rotation.y ?? 0.3,
    }
    ;(e.currentTarget as HTMLElement)?.setPointerCapture?.(e.pointerId)
  }

  function handlePointerMove(e: PointerEvent) {
    if (!dragState.active || !viewer) return
    const dx = e.clientX - dragState.startX
    viewer.playerObject.rotation.y = dragState.startRot + dx * 0.01
  }

  function handlePointerUp() { dragState.active = false }
  function handlePointerCancel() { dragState.active = false }
</script>

{#if !store.isAuthenticated}
  <div class="p-8 space-y-4">
    <div class="max-w-7xl mx-auto">
      <div class="mb-6">
        <h1 class="text-2xl font-semibold text-[var(--text-primary)] tracking-tight">Sign In</h1>
      </div>
      <div class="flex flex-col items-center justify-center min-h-[calc(100vh-300px)]">
        <User size={64} class="text-[#4572e3] mb-4" strokeWidth={1.5} />
        <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-1">Sign In Required</h3>
        <p class="text-sm text-[var(--text-muted)]">
          Please sign in with your Microsoft account to manage your skin
        </p>
      </div>
    </div>
  </div>
{:else}
  <div
    role="presentation"
    bind:this={containerEl}
    style="position: relative; overflow: hidden; height: 100%; box-sizing: border-box; user-select: none; -webkit-user-select: none;"
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    onpointercancel={handlePointerCancel}
  >
    <div style="position: absolute; inset: 0; z-index: 0; pointer-events: none;">
      <div style="width: 100%; height: 100%; position: relative; overflow: hidden; background-image: url(/skinstab/background.webp); background-size: cover; background-position: center; background-repeat: no-repeat;">
        <div style="position: absolute; inset: 0; background: rgba(0,0,0,0.35); z-index: 1;"></div>
        <div style="position: absolute; inset: 0; z-index: 3; display: flex; flex-direction: column; align-items: center; justify-content: center; pointer-events: none; opacity: {loading ? 1 : 0}; transition: opacity 0.15s;">
          <Loader2 size={32} style="animation: spin 1s linear infinite; color: #3b82f6; margin-bottom: 12px;" />
          <p style="font-size: 14px; color: var(--text-muted); margin: 0;">Loading skin…</p>
        </div>
        <canvas
          bind:this={canvasEl}
          style="display: block; position: relative; z-index: 2; opacity: {loading ? 0 : 1}; transition: opacity 0.2s; width: 100%; height: 100%;"
        ></canvas>
      </div>
    </div>

    <div style="position: relative; z-index: 1; height: 100%; box-sizing: border-box; padding: 32px 32px 88px;">
      <div class="max-w-7xl mx-auto w-full">
        <div class="flex items-center justify-between mb-6">
          <h1 class="text-2xl font-semibold text-[var(--text-primary)] tracking-tight">
            {store.activeAccount?.username ?? "Skins"}
          </h1>
        </div>
      </div>

      {#if error}
        <div style="position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); pointer-events: auto;">
          <div class="bg-[var(--bg-tertiary)] rounded-md p-4 max-w-sm w-full">
            <p class="text-xs text-red-400 leading-relaxed text-center">{error}</p>
          </div>
        </div>
      {/if}
    </div>

    <input bind:this={fileInputEl} type="file" accept="image/png" onchange={handleFileSelect} class="hidden" />

    <div
      role="presentation"
      onpointerdown={(e) => e.stopPropagation()}
      style="position: absolute; bottom: 32px; left: 50%; transform: translateX(-50%); z-index: 10; display: flex; align-items: center; gap: 12px; max-width: calc(100% - 64px); flex-wrap: wrap; justify-content: center;"
    >
      <div style="display: flex; align-items: center; gap: 6px; padding: 6px 10px; border-radius: 12px; background: var(--bg-elevated); backdrop-filter: blur(12px); justify-content: center;">
        <!-- SkinTooltip: Upload -->
        <div role="presentation" style="position: relative; display: inline-flex;" onmouseenter={() => uploadTooltipHover = true} onmouseleave={() => uploadTooltipHover = false}>
          <button
            onclick={() => fileInputEl?.click()}
            disabled={uploading || loading}
            style="display: flex; align-items: center; justify-content: center; width: 34px; height: 34px; border-radius: 6px; border: none; cursor: {uploading || loading ? 'not-allowed' : 'pointer'}; opacity: {uploading || loading ? 0.4 : 1}; background: transparent; color: #16a34a; transition: background 0.15s, color 0.15s; flex-shrink: 0;"
            onmouseenter={(e) => { if (!(uploading || loading)) (e.currentTarget as HTMLButtonElement).style.background = 'var(--bg-hover)' }}
            onmouseleave={(e) => { (e.currentTarget as HTMLButtonElement).style.background = 'transparent' }}
          >
            {#if uploading}
              <Loader2 size={20} style="animation: spin 1s linear infinite;" />
            {:else}
              <Upload size={20} strokeWidth={2.5} />
            {/if}
          </button>
          {#if uploadTooltipHover}
            <div style="position: absolute; bottom: calc(100% + 6px); left: 50%; transform: translateX(-50%); padding: 3px 7px; border-radius: 4px; background: var(--bg-secondary, #1a1a2e); color: var(--text-primary, #eee); font-size: 11px; white-space: nowrap; pointer-events: none; z-index: 100; border: 1px solid var(--border-color, #333);">
              Upload skin
            </div>
          {/if}
        </div>

        <!-- SkinTooltip: Reset -->
        <div role="presentation" style="position: relative; display: inline-flex;" onmouseenter={() => resetTooltipHover = true} onmouseleave={() => resetTooltipHover = false}>
          <button
            onclick={handleReset}
            disabled={loading}
            style="display: flex; align-items: center; justify-content: center; width: 34px; height: 34px; border-radius: 6px; border: none; cursor: {loading ? 'not-allowed' : 'pointer'}; opacity: {loading ? 0.4 : 1}; background: transparent; color: var(--text-muted); transition: background 0.15s, color 0.15s; flex-shrink: 0;"
            onmouseenter={(e) => { if (!loading) (e.currentTarget as HTMLButtonElement).style.background = 'var(--bg-hover)' }}
            onmouseleave={(e) => { (e.currentTarget as HTMLButtonElement).style.background = 'transparent' }}
          >
            <RotateCcw size={20} strokeWidth={2.5} />
          </button>
          {#if resetTooltipHover}
            <div style="position: absolute; bottom: calc(100% + 6px); left: 50%; transform: translateX(-50%); padding: 3px 7px; border-radius: 4px; background: var(--bg-secondary, #1a1a2e); color: var(--text-primary, #eee); font-size: 11px; white-space: nowrap; pointer-events: none; z-index: 100; border: 1px solid var(--border-color, #333);">
              Reset to default skin
            </div>
          {/if}
        </div>

        <button
          onclick={() => skinVariant = "classic"}
          style="padding: 6px 10px; border-radius: 8px; border: none; cursor: pointer; font-size: 13px; font-weight: 600; letter-spacing: 0.01em; background: {skinVariant === 'classic' ? 'var(--accent-primary, #4572e3)' : 'var(--bg-secondary)'}; color: {skinVariant === 'classic' ? '#fff' : 'var(--text-muted)'}; transition: background 0.15s, color 0.15s; flex-shrink: 0; white-space: nowrap; box-sizing: border-box;"
          onmouseenter={(e) => { if (skinVariant !== 'classic') (e.currentTarget as HTMLButtonElement).style.background = 'var(--bg-hover, #2a2a3e)' }}
          onmouseleave={(e) => { if (skinVariant !== 'classic') (e.currentTarget as HTMLButtonElement).style.background = 'var(--bg-secondary)' }}
        >
          Classic
        </button>
        <button
          onclick={() => skinVariant = "slim"}
          style="padding: 6px 10px; border-radius: 8px; border: none; cursor: pointer; font-size: 13px; font-weight: 600; letter-spacing: 0.01em; background: {skinVariant === 'slim' ? 'var(--accent-primary, #4572e3)' : 'var(--bg-secondary)'}; color: {skinVariant === 'slim' ? '#fff' : 'var(--text-muted)'}; transition: background 0.15s, color 0.15s; flex-shrink: 0; white-space: nowrap; box-sizing: border-box;"
          onmouseenter={(e) => { if (skinVariant !== 'slim') (e.currentTarget as HTMLButtonElement).style.background = 'var(--bg-hover, #2a2a3e)' }}
          onmouseleave={(e) => { if (skinVariant !== 'slim') (e.currentTarget as HTMLButtonElement).style.background = 'var(--bg-secondary)' }}
        >
          Slim
        </button>

        {#if loadingCapes || capes.length > 0}
          <!-- CapeBar -->
          {#if loadingCapes}
            <div style="display: flex; align-items: center; gap: 4px;">
              <Loader2 size={14} style="animation: spin 1s linear infinite; color: var(--text-muted);" />
              <span style="font-size: 11px; color: var(--text-muted);">Capes…</span>
            </div>
          {:else if capes.length === 0}
            <span style="font-size: 11px; color: var(--text-muted);">No capes</span>
          {:else}
            <div style="display: flex; align-items: center; gap: 3px;">
              {#each capes as cape (cape.id)}
                {@const isActive = activeCape === cape.id}
                {@const isHovered = hoveredCapeId === cape.id}
                <div
                  role="presentation"
                  style="position: relative; display: inline-flex;"
                  onmouseenter={() => hoveredCapeId = cape.id}
                  onmouseleave={() => hoveredCapeId = null}
                >
                  <div
                    role="button"
                    tabindex="0"
                    onclick={() => isActive ? handleCapeRemove() : handleCapeSelect(cape.id)}
                    onkeydown={(e) => { if (e.key === 'Enter') isActive ? handleCapeRemove() : handleCapeSelect(cape.id) }}
                    onmouseenter={() => hoveredCapeId = cape.id}
                    onmouseleave={() => hoveredCapeId = null}
                    style="cursor: pointer; flex-shrink: 0; line-height: 0;"
                  >
                    <div style="width: 26px; height: 38px; border-radius: 4px; border: 2px solid {isHovered ? 'var(--text-muted)' : 'transparent'}; overflow: hidden; background: var(--bg-secondary); transition: border-color 0.15s; position: relative;">
                      <img
                        src="/capes/{getCapeImageName(cape.alias)}.webp"
                        alt={cape.alias}
                        draggable={false}
                        style="width: 100%; height: 100%; object-fit: contain; image-rendering: pixelated; display: block;"
                        onerror={(e) => { const img = e.currentTarget as HTMLImageElement; img.onerror = null; img.src = '/logo.png' }}
                      />
                      {#if isActive}
                        <div style="position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; background: rgba(0,0,0,0.45);">
                          <svg width="16" height="16" viewBox="0 0 12 12" fill="none">
                            <path d="M2 6L5 9L10 3" stroke={isHovered ? "#ef4444" : "#22c55e"} stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" />
                          </svg>
                        </div>
                      {/if}
                    </div>
                  </div>
                  {#if isHovered}
                    <div style="position: absolute; bottom: calc(100% + 6px); left: 50%; transform: translateX(-50%); padding: 3px 7px; border-radius: 4px; background: var(--bg-secondary, #1a1a2e); color: var(--text-primary, #eee); font-size: 11px; white-space: nowrap; pointer-events: none; z-index: 100; border: 1px solid var(--border-color, #333);">
                      {cape.alias}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}

          <!-- ElytraToggle -->
          {#if activeCapeUrl}
            <div role="presentation" style="position: relative; display: inline-flex;" onmouseenter={() => elytraHovered = true} onmouseleave={() => elytraHovered = false}>
              <button
                onclick={() => showElytra = !showElytra}
                style="display: flex; align-items: center; justify-content: center; width: 34px; height: 34px; border-radius: 6px; border: none; cursor: pointer; background: {elytraHovered ? 'var(--bg-hover)' : 'transparent'}; color: var(--text-muted); transition: background 0.15s; flex-shrink: 0;"
                onmouseenter={() => elytraHovered = true}
                onmouseleave={() => elytraHovered = false}
              >
                {#if showElytra}
                  <RectangleVertical size={20} strokeWidth={2.5} />
                {:else}
                  <Plane size={20} strokeWidth={2.5} />
                {/if}
              </button>
              {#if elytraHovered}
                <div style="position: absolute; bottom: calc(100% + 6px); left: 50%; transform: translateX(-50%); padding: 3px 7px; border-radius: 4px; background: var(--bg-secondary, #1a1a2e); color: var(--text-primary, #eee); font-size: 11px; white-space: nowrap; pointer-events: none; z-index: 100; border: 1px solid var(--border-color, #333);">
                  {showElytra ? "Show as cape" : "Show as elytra"}
                </div>
              {/if}
            </div>
          {/if}
        {/if}

        {#if hasPendingChanges}
          <div style="display: flex; gap: 8px; margin-left: auto; flex-shrink: 0;">
            <button
              onclick={handleDiscard}
              disabled={saving}
              style="padding: 6px 16px; border-radius: 8px; border: none; background: var(--bg-secondary, #1a1a2e); color: var(--text-muted, #888); cursor: {saving ? 'not-allowed' : 'pointer'}; font-size: 13px; font-weight: 600; opacity: {saving ? 0.5 : 1}; transition: background 0.15s, opacity 0.15s;"
              onmouseenter={(e) => { if (!saving) (e.currentTarget as HTMLButtonElement).style.background = 'var(--bg-hover, #2a2a3e)' }}
              onmouseleave={(e) => { if (!saving) (e.currentTarget as HTMLButtonElement).style.background = 'var(--bg-secondary, #1a1a2e)' }}
            >
              Cancel
            </button>
            <button
              onclick={handleSave}
              disabled={saving}
              style="padding: 6px 16px; border-radius: 8px; border: none; background: #16a34a; color: #fff; cursor: {saving ? 'not-allowed' : 'pointer'}; font-size: 13px; font-weight: 600; opacity: {saving ? 0.6 : 1}; transition: background 0.15s, opacity 0.15s; display: flex; align-items: center; gap: 6px;"
              onmouseenter={(e) => { if (!saving) (e.currentTarget as HTMLButtonElement).style.background = '#15803d' }}
              onmouseleave={(e) => { if (!saving) (e.currentTarget as HTMLButtonElement).style.background = '#16a34a' }}
            >
              {#if saving}
                Saving…
              {:else}
                <Save size={16} strokeWidth={2.5} /> Save
              {/if}
            </button>
          </div>
        {/if}
      </div>

      {#if recentSkins.length > 0}
        <div style="display: flex; align-items: center; gap: 4px; padding: 5px 6px; border-radius: 12px; background: var(--bg-elevated); backdrop-filter: blur(12px); flex-shrink: 0;">
          {#each recentSkins as skin, index}
            {@const match = skin.url.match(/texture\/([a-f0-9]+)/)}
            {@const hash = match ? match[1] : null}
            {@const bustUrl = hash ? `https://renders.stellarmc.gg/bust/${hash}${skin.variant === "slim" ? "?slim" : ""}` : skin.url}
            {@const skinHovered = hoveredRecentIdx === index}
            <div role="presentation" style="position: relative; display: inline-flex;" onmouseenter={() => hoveredRecentIdx = index} onmouseleave={() => hoveredRecentIdx = null}>
              <button
                onclick={() => handleRecentSkinSelect(skin)}
                disabled={uploading}
                style="box-sizing: border-box; width: 40px; height: 40px; border-radius: 6px; padding: 0; border: 2px solid {skinHovered ? 'var(--text-muted)' : 'transparent'}; overflow: hidden; background: transparent; cursor: {uploading ? 'not-allowed' : 'pointer'}; opacity: {uploading ? 0.4 : 1}; transition: border-color 0.15s; flex-shrink: 0;"
                onmouseenter={() => hoveredRecentIdx = index}
                onmouseleave={() => hoveredRecentIdx = null}
              >
                <img
                  src={bustUrl}
                  alt="Recent skin"
                  draggable={false}
                  style="width: 100%; height: 100%; object-fit: cover; image-rendering: pixelated; display: block;"
                />
              </button>
              {#if skinHovered}
                <div style="position: absolute; bottom: calc(100% + 6px); left: 50%; transform: translateX(-50%); padding: 3px 7px; border-radius: 4px; background: var(--bg-secondary, #1a1a2e); color: var(--text-primary, #eee); font-size: 11px; white-space: nowrap; pointer-events: none; z-index: 100; border: 1px solid var(--border-color, #333);">
                  Apply recent skin
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
{/if}
