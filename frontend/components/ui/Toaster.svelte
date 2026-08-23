<script lang="ts">
  import { CheckCheck, TriangleAlert, X } from "lucide-svelte"
  import { fly } from "svelte/transition"
  import { toastStore, dismissToast } from "../../lib/toastStore.svelte"
</script>

{#if toastStore.toasts.length > 0}
  <div class="fixed top-12 right-6 z-[200] flex flex-col items-end gap-2 pointer-events-none">
    {#each toastStore.toasts as toast (toast.id)}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div
        transition:fly|global={{ x: 24, duration: 160 }}
        class="bg-[var(--bg-surface)] rounded-lg px-3.5 py-2.5 flex items-center gap-2.5 text-left pointer-events-auto cursor-pointer max-w-[380px]"
        onclick={() => dismissToast(toast.id)}
      >
        {#if toast.type === "success"}
          <CheckCheck size={19} strokeWidth={2} class="text-[#16a34a] flex-shrink-0" />
        {:else}
          <TriangleAlert size={19} strokeWidth={2} class="text-red-400 flex-shrink-0" />
        {/if}
        <span class="text-sm text-[var(--text-primary)] truncate">{toast.message}</span>
        <button
          title="Dismiss"
          onclick={(e) => { e.stopPropagation(); dismissToast(toast.id) }}
          class="flex-shrink-0 -mr-1 p-0.5 flex items-center justify-center text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors cursor-pointer"
        >
          <X size={16} strokeWidth={2.5} />
        </button>
      </div>
    {/each}
  </div>
{/if}
