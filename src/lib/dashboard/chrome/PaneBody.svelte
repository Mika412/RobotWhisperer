<script lang="ts">
  import { getPane, type PaneContext } from "$lib/dashboard/registry/paneRegistry";
  import type { PaneNode } from "$lib/dashboard/layout/layout";

  let { pane, ctx }: { pane: PaneNode; ctx: PaneContext } = $props();

  const descriptor = $derived(getPane(pane.paneType));
</script>

<div class="pane-body">
  {#if descriptor}
    {@const PaneComponent = descriptor.component}
    <PaneComponent config={pane.config} {ctx} />
  {:else}
    <div class="missing">Unknown pane type: <code>{pane.paneType}</code></div>
  {/if}
</div>

<style>
  .pane-body {
    flex: 1;
    min-height: 0;
    min-width: 0;
    position: relative;
    background: var(--color-bg-deep);
    overflow: hidden;
  }
  .missing {
    padding: 12px;
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--color-danger);
  }
</style>
