<script lang="ts">
  import { X, Pencil } from "@lucide/svelte";
  import { getPane } from "$lib/dashboard/registry/paneRegistry";
  import { removeNode, renameNode, setActiveTab } from "$lib/stores/dashboardStore.svelte";
  import { dashboardState } from "$lib/stores/dashboardStore.svelte";
  import { dragState, draggable } from "$lib/dashboard/layout/dragState.svelte";
  import { maximizeStore } from "$lib/dashboard/layout/maximizeStore.svelte";
  import { createPaneContext } from "$lib/dashboard/registry/paneContext.svelte";
  import PaneActions from "$lib/dashboard/chrome/PaneActions.svelte";
  import GroupedPane from "$lib/dashboard/chrome/GroupedPane.svelte";
  import InlineRename from "$lib/components/common/InlineRename.svelte";
  import type { GroupNode, PaneNode } from "$lib/dashboard/layout/layout";

  let { layoutId, node }: { layoutId: string; node: GroupNode } = $props();

  function paneOf(id: string): PaneNode | undefined {
    const found = dashboardState.layouts[layoutId]?.nodes[id];
    return found?.kind === "pane" ? found : undefined;
  }

  const activeId = $derived(node.tabs.includes(node.activeTab) ? node.activeTab : node.tabs[0]);
  const activePane = $derived(paneOf(activeId));
  const maximized = $derived(
    activePane ? maximizeStore.isMaximized(layoutId, activePane.id) : false,
  );
  const dragging = $derived(
    dragState.active?.kind === "move-node" &&
      (dragState.active.nodeId === node.id || node.tabs.includes(dragState.active.nodeId)),
  );

  const ctx = createPaneContext(
    () => layoutId,
    () => activePane!,
  );

  let editingId = $state<string | null>(null);

  function label(id: string): string {
    const pane = paneOf(id);
    return pane?.title ?? getPane(pane?.paneType ?? "")?.displayName ?? "Pane";
  }
  function closeActive() {
    if (activePane) {
      if (maximized) maximizeStore.clear();
      removeNode(layoutId, activePane.id);
    }
  }
</script>

<div class="group" class:dragging data-node-id={node.id}>
  <header
    class="chrome"
    role="toolbar"
    tabindex="-1"
    aria-label="Group header, drag to move"
    use:draggable={{
      payload: () => ({ kind: "move-node", nodeId: node.id }),
      label: () => label(activeId),
    }}
  >
    <div class="tabs" role="tablist">
      {#each node.tabs as tabId (tabId)}
        <div
          class="tab"
          class:active={tabId === activeId}
          role="tab"
          tabindex={tabId === activeId ? 0 : -1}
          aria-selected={tabId === activeId}
          use:draggable={{
            payload: () => ({ kind: "move-node", nodeId: tabId }),
            label: () => label(tabId),
          }}
          onclick={() => setActiveTab(layoutId, node.id, tabId)}
          ondblclick={() => (editingId = tabId)}
          onkeydown={(event) => {
            if (event.key === "Enter" || event.key === " ") setActiveTab(layoutId, node.id, tabId);
          }}
        >
          <InlineRename
            value={label(tabId)}
            editing={editingId === tabId}
            oneditingchange={(next) => (editingId = next ? tabId : null)}
            oncommit={(value) => renameNode(layoutId, tabId, value)}
          />
          {#if editingId !== tabId}
            <button
              class="tab-icon-btn"
              title="Rename tab"
              aria-label="Rename tab"
              draggable="false"
              onpointerdown={(event) => event.stopPropagation()}
              onclick={(event) => {
                event.stopPropagation();
                editingId = tabId;
              }}
            >
              <Pencil size={10} />
            </button>
            <button
              class="tab-icon-btn"
              aria-label="Close tab"
              draggable="false"
              onclick={(event) => {
                event.stopPropagation();
                if (maximizeStore.isMaximized(layoutId, tabId)) maximizeStore.clear();
                removeNode(layoutId, tabId);
              }}
            >
              <X size={11} />
            </button>
          {/if}
        </div>
      {/each}
      <div class="filler"></div>
    </div>
    {#if activePane}
      <PaneActions
        pane={activePane}
        {ctx}
        {maximized}
        onmaximize={() => activePane && maximizeStore.toggle(layoutId, activePane.id)}
        onclose={closeActive}
      />
    {/if}
  </header>
  <div class="body">
    {#each node.tabs as tabId (tabId)}
      {@const tabPane = paneOf(tabId)}
      {#if tabPane}
        <GroupedPane {layoutId} pane={tabPane} active={tabId === activeId} />
      {/if}
    {/each}
  </div>
</div>

<style>
  .group {
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
  .group.dragging {
    opacity: 0.4;
    filter: grayscale(0.7);
  }
  .body {
    position: relative;
    flex: 1;
    min-height: 0;
    min-width: 0;
  }
  .chrome {
    display: flex;
    align-items: stretch;
    gap: 4px;
    padding: 0 6px;
    background: var(--color-bg-sidebar);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
    min-height: 30px;
    cursor: grab;
    user-select: none;
  }
  .chrome:active {
    cursor: grabbing;
  }
  .tabs {
    display: flex;
    align-items: stretch;
    flex: 1;
    min-width: 0;
    overflow-x: auto;
  }
  .filler {
    flex: 1;
    min-width: 16px;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px 4px 10px;
    border-bottom: 2px solid transparent;
    color: var(--color-text-dimmer);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }
  .tab:hover {
    color: var(--color-text-main);
  }
  .tab.active {
    color: var(--color-accent);
    border-bottom-color: var(--color-accent);
    background: var(--color-bg-main);
  }
  .tab-icon-btn {
    display: inline-flex;
    align-items: center;
    background: transparent;
    border: 0;
    color: inherit;
    opacity: 0;
    padding: 1px;
    border-radius: 3px;
    cursor: pointer;
  }
  .tab:hover .tab-icon-btn,
  .tab.active .tab-icon-btn {
    opacity: 0.7;
  }
  .tab-icon-btn:hover {
    opacity: 1;
    background: var(--color-bg-hover);
  }
  .chrome :global(.actions) {
    align-self: center;
  }
</style>
