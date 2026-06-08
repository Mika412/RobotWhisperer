<script lang="ts">
  import { LayoutGrid, Maximize, Minimize, Plus } from "@lucide/svelte";
  import LayoutNodeView from "$lib/dashboard/chrome/LayoutNodeView.svelte";
  import PaneHost from "$lib/dashboard/chrome/PaneHost.svelte";
  import DashboardDropTracker from "$lib/dashboard/chrome/DashboardDropTracker.svelte";
  import AddPanePopover from "$lib/dashboard/chrome/AddPanePopover.svelte";
  import Button from "$lib/components/common/Button.svelte";
  import Popover from "$lib/components/common/Popover.svelte";
  import { dashboardStore, dashboardState, addPane } from "$lib/stores/dashboardStore.svelte";
  import { isPlaceholder } from "$lib/dashboard/layout/layout";
  import { maximizeStore } from "$lib/dashboard/layout/maximizeStore.svelte";
  import { fullscreenStore } from "$lib/stores/fullscreenStore.svelte";
  import { dragState } from "$lib/dashboard/layout/dragState.svelte";

  let { dashboardId }: { dashboardId: string } = $props();

  const layout = $derived(dashboardState.layouts[dashboardId]);

  const maximizedPane = $derived.by(() => {
    if (!layout) return null;
    const id = maximizeStore.maximizedNodeFor(layout.id);
    const node = id ? layout.nodes[id] : undefined;
    return node?.kind === "pane" ? node : null;
  });

  const isEmpty = $derived(layout ? isPlaceholder(layout.nodes[layout.root]) : false);

  let canvasEl = $state<HTMLDivElement>();
  let addOpen = $state(false);
  let addOpenEmpty = $state(false);

  function pickPane(type: string, displayName: string, defaultConfig: Record<string, unknown>) {
    addPane(dashboardId, { paneType: type, title: displayName, config: defaultConfig });
    addOpen = false;
    addOpenEmpty = false;
  }
</script>

{#if layout}
  {@const fs = fullscreenStore.isFullscreen}
  <div class="layout-view">
    <div class="header">
      <div class="title-block">
        <LayoutGrid size={18} class="title-icon" />
        <input
          class="title-input"
          value={layout.title}
          aria-label="Dashboard name"
          oninput={(event) => dashboardStore.rename(dashboardId, event.currentTarget.value)}
        />
      </div>
      <div class="actions">
        <Button variant="ghost" onclick={() => fullscreenStore.toggle()}>
          {#if fs}<Minimize size={14} /> Exit fullscreen{:else}<Maximize size={14} /> Fullscreen{/if}
        </Button>
        <Popover bind:open={addOpen} align="end" portalled>
          {#snippet trigger({ toggle })}
            <Button onclick={toggle}>
              <Plus size={14} /> Add pane
            </Button>
          {/snippet}
          <AddPanePopover onpick={pickPane} />
        </Popover>
      </div>
    </div>
    <div class="canvas" bind:this={canvasEl}>
      {#if maximizedPane}
        <div class="maximized">
          <PaneHost layoutId={dashboardId} pane={maximizedPane} />
        </div>
      {:else if isEmpty}
        <div class="empty" data-node-id={layout.root}>
          <div class="empty-card">
            <h2>Empty dashboard</h2>
            <p>Click <strong>Add pane</strong> to start, or drag a pane tile onto the canvas.</p>
            <Popover bind:open={addOpenEmpty} align="start" portalled>
              {#snippet trigger({ toggle })}
                <Button onclick={toggle}>
                  <Plus size={14} /> Add pane
                </Button>
              {/snippet}
              <AddPanePopover onpick={pickPane} />
            </Popover>
          </div>
        </div>
        <DashboardDropTracker layoutId={dashboardId} canvasEl={canvasEl ?? null} />
      {:else}
        <LayoutNodeView layoutId={dashboardId} nodeId={layout.root} />
        <DashboardDropTracker layoutId={dashboardId} canvasEl={canvasEl ?? null} />
      {/if}
    </div>
  </div>

  {#if dragState.active}
    <div class="drag-ghost" style="left: {dragState.x}px; top: {dragState.y}px;">
      {dragState.label}
    </div>
  {/if}
{/if}

<style>
  .layout-view {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--color-bg-deep);
  }
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 14px;
    flex-shrink: 0;
  }
  .title-block {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  :global(.title-icon) {
    color: var(--color-accent);
  }
  .title-input {
    font-size: 16px;
    font-weight: 700;
    color: var(--color-text-main);
    background: transparent;
    border: 0;
    outline: none;
    padding: 2px 4px;
    border-radius: 4px;
    min-width: 0;
  }
  .title-input:hover {
    background: var(--color-bg-hover);
  }
  .title-input:focus {
    background: var(--color-bg-input);
  }
  .actions {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .canvas {
    flex: 1;
    min-height: 0;
    padding: 0 12px 12px;
    position: relative;
  }
  .maximized {
    position: absolute;
    inset: 0;
    padding: 0 12px 12px;
  }
  .empty {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .empty-card {
    max-width: 420px;
    padding: 24px 28px;
    background: var(--color-bg-main);
    border: 1px dashed var(--color-border);
    border-radius: 12px;
    text-align: center;
  }
  .empty-card h2 {
    font-size: 16px;
    font-weight: 700;
    margin: 0 0 6px;
  }
  .empty-card p {
    font-size: 13px;
    color: var(--color-text-dimmer);
    margin: 0 0 14px;
  }
  .empty-card strong {
    color: var(--color-accent);
  }
  .drag-ghost {
    position: fixed;
    z-index: 9999;
    transform: translate(12px, 12px);
    padding: 5px 10px;
    max-width: 240px;
    border-radius: 7px;
    background: var(--color-accent);
    color: #fff;
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    pointer-events: none;
    opacity: 0.95;
  }
</style>
