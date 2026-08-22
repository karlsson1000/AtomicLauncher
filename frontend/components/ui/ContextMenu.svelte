<script lang="ts">
  import { portal } from "../../lib/portal"
  interface ContextMenuItem {
    label?: string
    icon?: new (...args: any[]) => any
    onClick?: () => void
    danger?: boolean
    separator?: boolean
  }

  let { x, y, items, onClose }: {
    x: number
    y: number
    items: ContextMenuItem[]
    onClose: () => void
  } = $props()

  let menuEl: HTMLDivElement | undefined = $state()
  let position = $state({ x: 0, y: 0 })
  let ready = $state(false)

  $effect(() => {
    function handleClick(e: MouseEvent) {
      if (menuEl && !menuEl.contains(e.target as Node)) onClose()
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose()
    }

    document.addEventListener("mousedown", handleClick)
    document.addEventListener("keydown", handleKey)
    return () => {
      document.removeEventListener("mousedown", handleClick)
      document.removeEventListener("keydown", handleKey)
    }
  })

  $effect(() => {
    if (!menuEl) return
    const maxX = window.innerWidth - menuEl.offsetWidth - 8
    const maxY = window.innerHeight - menuEl.offsetHeight - 8
    position = {
      x: Math.max(8, Math.min(x, maxX)),
      y: Math.max(8, Math.min(y, maxY)),
    }
    ready = true
  })
</script>

<div use:portal>
  <div
    bind:this={menuEl}
    role="menu"
    class="context-menu fixed z-[100]"
    class:visible={ready}
    style="left: {position.x}px; top: {position.y}px"
  >
    <div class="blur-border bg-[var(--bg-elevated)] rounded-lg overflow-hidden min-w-[180px] max-w-[280px] px-1.5 py-2 flex flex-col gap-y-0.5">
      {#each items as item}
        {#if item.separator}
          <div class="h-px bg-[var(--border-subtle)] my-1 mx-1"></div>
        {:else}
          <button
            role="menuitem"
            onclick={() => { item.onClick?.(); onClose() }}
            class="w-full flex items-center gap-2.5 px-2 py-1.5 text-[0.95rem] rounded-md transition-colors cursor-pointer select-none
              {item.danger
                ? 'text-red-400 hover:bg-red-500/10'
                : 'text-[var(--text-primary)] hover:bg-[var(--bg-hover-light)]'}"
          >
            {#if item.icon}
              <span class="flex-shrink-0 w-5 h-5 flex items-center justify-center">
                <item.icon size={17} strokeWidth={2} />
              </span>
            {:else}
              <span class="flex-shrink-0 w-5"></span>
            {/if}
            <span class="truncate">{item.label}</span>
          </button>
        {/if}
      {/each}
    </div>
  </div>
</div>

<style>
  .context-menu {
    transform-origin: top left;
    visibility: hidden;
  }

  .context-menu.visible {
    visibility: visible;
    animation: contextMenuIn 0.12s ease-out forwards;
  }

  @keyframes contextMenuIn {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>