<script lang="ts">
  import { Pencil } from "@lucide/svelte";
  import { getPane } from "$lib/dashboard/registry/paneRegistry";
  import { removeNode, renameNode } from "$lib/stores/dashboardStore.svelte";
  import { dragState, draggable } from "$lib/dashboard/layout/dragState.svelte";
  import { maximizeStore } from "$lib/dashboard/layout/maximizeStore.svelte";
  import { createPaneContext } from "$lib/dashboard/registry/paneContext.svelte";
  import PaneActions from "$lib/dashboard/chrome/PaneActions.svelte";
  import PaneBody from "$lib/dashboard/chrome/PaneBody.svelte";
  import InlineRename from "$lib/components/common/InlineRename.svelte";
  import type { PaneNode } from "$lib/dashboard/layout/layout";

  let { layoutId, pane }: { layoutId: string; pane: PaneNode } = $props();

  const descriptor = $derived(getPane(pane.paneType));
  const maximized = $derived(maximizeStore.isMaximized(layoutId, pane.id));
  const dragging = $derived(
    dragState.active?.kind === "move-node" && dragState.active.nodeId === pane.id,
  );
  const ctx = createPaneContext(
    () => layoutId,
    () => pane,
  );
  const topic = $derived((pane.config as { topic?: string }).topic ?? "");
  const title = $derived(pane.title ?? descriptor?.displayName ?? pane.paneType);

  let renaming = $state(false);

  function close() {
    if (maximized) maximizeStore.clear();
    removeNode(layoutId, pane.id);
  }
</script>

<div class="pane-host" class:dragging data-node-id={pane.id}>
  <header
    class="chrome"
    role="toolbar"
    tabindex="-1"
    aria-label="Pane header, drag to move"
    use:draggable={{
      payload: () => ({ kind: "move-node", nodeId: pane.id }),
      label: () => pane.title ?? descriptor?.displayName ?? pane.paneType,
    }}
  >
    <span class="title-wrap">
      <InlineRename
        value={title}
        editing={renaming}
        oneditingchange={(next) => (renaming = next)}
        oncommit={(next) => renameNode(layoutId, pane.id, next)}
      />
      {#if !renaming}
        <button
          class="rename"
          title="Rename pane"
          aria-label="Rename pane"
          draggable="false"
          onpointerdown={(event) => event.stopPropagation()}
          onclick={() => (renaming = true)}
        >
          <Pencil size={11} />
        </button>
      {/if}
    </span>
    {#if topic}<span class="caption">{topic}</span>{/if}
    <PaneActions
      {pane}
      {ctx}
      {maximized}
      onmaximize={() => maximizeStore.toggle(layoutId, pane.id)}
      onclose={close}
    />
  </header>
  <PaneBody {pane} {ctx} />
</div>

<style>
  .pane-host {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    min-width: 0;
    background: var(--color-bg-main);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    overflow: hidden;
  }
  .pane-host.dragging {
    opacity: 0.4;
    filter: grayscale(0.7);
  }
  .chrome {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 6px 3px 10px;
    background: var(--color-bg-sidebar);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
    cursor: grab;
    user-select: none;
  }
  .chrome:active {
    cursor: grabbing;
  }
  .title-wrap {
    display: inline-flex;
    align-items: center;
    gap: 1px;
    flex: none;
    min-width: 0;
    max-width: 60%;
    font-size: 12px;
    font-weight: 700;
    color: var(--color-text-main);
  }
  .rename {
    display: inline-flex;
    align-items: center;
    flex: none;
    padding: 2px;
    border: 0;
    border-radius: 3px;
    background: transparent;
    color: var(--color-text-disabled);
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.12s ease;
  }
  .chrome:hover .rename {
    opacity: 0.65;
  }
  .rename:hover {
    opacity: 1;
    color: var(--color-text-main);
    background: var(--color-bg-hover);
  }
  .caption {
    flex: 1;
    font-size: 11px;
    color: var(--color-text-disabled);
    font-family: var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
  }
  .chrome :global(.actions) {
    margin-left: auto;
  }
</style>
