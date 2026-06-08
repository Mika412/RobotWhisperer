<script lang="ts">
  import { modalStore } from "$lib/stores/modalStore.svelte";
  import { onMount, onDestroy } from "svelte";

  let { component, props } = $derived(modalStore);

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      modalStore.close();
    }
  }

  function handleOverlayKeydown(event: KeyboardEvent) {
    if (event.target !== event.currentTarget) return;
    if (event.key === "Enter" || event.key === " ") {
      modalStore.close();
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
  });
</script>

{#if component}
  {@const Rendered = component}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4 backdrop-blur-sm"
    onclick={modalStore.close}
    onkeydown={handleOverlayKeydown}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    <div
      class="bg-bg-main border-border w-full max-w-md rounded-xl border shadow-xl"
      onclick={(e) => e.stopPropagation()}
      onkeydown={handleOverlayKeydown}
      role="dialog"
      tabindex="-1"
    >
      <Rendered {...props} />
    </div>
  </div>
{/if}
