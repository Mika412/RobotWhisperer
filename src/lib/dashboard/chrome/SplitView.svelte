<script lang="ts">
  import LayoutNodeView from "$lib/dashboard/chrome/LayoutNodeView.svelte";
  import { resizeSplit } from "$lib/stores/dashboardStore.svelte";
  import type { SplitNode } from "$lib/dashboard/layout/layout";

  let { layoutId, node }: { layoutId: string; node: SplitNode } = $props();

  const isRow = $derived(node.direction === "row");
  let gridEl = $state<HTMLDivElement>();

  const MIN_TRACK_PX = 120;

  function trackStyle(): string {
    const tracks: string[] = [];
    for (let i = 0; i < node.children.length; i += 1) {
      const fraction = node.sizes[i] ?? 1 / node.children.length;
      tracks.push(`${fraction}fr`);
      if (i < node.children.length - 1) tracks.push("5px");
    }
    const def = tracks.join(" ");
    return isRow
      ? `grid-template-columns: ${def}; grid-template-rows: 1fr;`
      : `grid-template-rows: ${def}; grid-template-columns: 1fr;`;
  }

  function startResize(gutterIndex: number, event: PointerEvent) {
    if (!gridEl) return;
    event.preventDefault();
    (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
    const rect = gridEl.getBoundingClientRect();
    const total = isRow ? rect.width : rect.height;
    const initial = node.sizes.slice();
    const origin = isRow ? event.clientX : event.clientY;
    const pair = initial[gutterIndex] + initial[gutterIndex + 1];
    const min = Math.min(MIN_TRACK_PX / Math.max(1, total), pair / 2);

    function move(moveEvent: PointerEvent) {
      const current = isRow ? moveEvent.clientX : moveEvent.clientY;
      const delta = (current - origin) / Math.max(1, total);
      const left = Math.min(Math.max(initial[gutterIndex] + delta, min), pair - min);
      const right = pair - left;
      const next = initial.slice();
      next[gutterIndex] = left;
      next[gutterIndex + 1] = right;
      resizeSplit(layoutId, node.id, next);
    }
    function up() {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    }
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }
</script>

<div class="split" class:row={isRow} bind:this={gridEl} style={trackStyle()}>
  {#each node.children as childId, i (childId)}
    <div
      class="cell"
      style={isRow
        ? `grid-column: ${2 * i + 1}; grid-row: 1;`
        : `grid-row: ${2 * i + 1}; grid-column: 1;`}
    >
      <LayoutNodeView {layoutId} nodeId={childId} />
    </div>
    {#if i < node.children.length - 1}
      <div
        class="gutter"
        class:row={isRow}
        style={isRow
          ? `grid-column: ${2 * i + 2}; grid-row: 1;`
          : `grid-row: ${2 * i + 2}; grid-column: 1;`}
        role="separator"
        aria-orientation={isRow ? "vertical" : "horizontal"}
        onpointerdown={(event) => startResize(i, event)}
      ></div>
    {/if}
  {/each}
</div>

<style>
  .split {
    display: grid;
    gap: 0;
    height: 100%;
    width: 100%;
    min-height: 0;
    min-width: 0;
  }
  .cell {
    min-height: 0;
    min-width: 0;
    overflow: hidden;
  }
  .gutter {
    position: relative;
    z-index: 5;
    display: grid;
    place-items: center;
    cursor: row-resize;
    touch-action: none;
  }
  .gutter.row {
    cursor: col-resize;
  }
  .gutter::before {
    content: "";
    position: absolute;
    inset: -7px 0;
  }
  .gutter.row::before {
    inset: 0 -7px;
  }
  .gutter::after {
    content: "";
    background: transparent;
    border-radius: 2px;
    transition: background 0.1s;
  }
  .gutter:not(.row)::after {
    height: 2px;
    width: 28px;
  }
  .gutter.row::after {
    width: 2px;
    height: 28px;
  }
  .gutter:hover::after {
    background: var(--color-accent);
  }
</style>
