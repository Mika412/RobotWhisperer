<script lang="ts">
  import { dragState, registerDropController } from "$lib/dashboard/layout/dragState.svelte";
  import { dropNewPane, moveNode, dashboardState } from "$lib/stores/dashboardStore.svelte";
  import { pickZone } from "$lib/dashboard/layout/dropZone";
  import { canDrop, type DropZone } from "$lib/dashboard/layout/layout";

  let { layoutId, canvasEl }: { layoutId: string; canvasEl: HTMLElement | null } = $props();

  interface Hover {
    nodeId: string;
    rect: DOMRect;
    zone: DropZone;
    allowed: boolean;
  }
  let hover = $state<Hover | null>(null);

  function tileUnder(x: number, y: number): { id: string; rect: DOMRect } | null {
    const host = (document.elementFromPoint(x, y) as HTMLElement | null)?.closest<HTMLElement>(
      "[data-node-id]",
    );
    if (!host?.dataset.nodeId) return null;
    return { id: host.dataset.nodeId, rect: host.getBoundingClientRect() };
  }

  function withinCanvas(x: number, y: number): boolean {
    if (!canvasEl) return false;
    const rect = canvasEl.getBoundingClientRect();
    return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
  }

  function onHover(x: number, y: number) {
    const payload = dragState.active;
    const layout = dashboardState.layouts[layoutId];
    if (!payload || !layout || !withinCanvas(x, y)) {
      hover = null;
      return;
    }
    const target = tileUnder(x, y);
    if (!target || (payload.kind === "move-node" && payload.nodeId === target.id)) {
      hover = null;
      return;
    }
    const zone = pickZone(target.rect, x, y);
    const allowed = canDrop(layout, payload, target.id, zone);
    if (hover && hover.nodeId === target.id && hover.zone === zone && hover.allowed === allowed) {
      hover.rect = target.rect;
      return;
    }
    hover = { nodeId: target.id, rect: target.rect, zone, allowed };
  }

  function onCommit() {
    const payload = dragState.active;
    const target = hover;
    if (!payload || !target || !target.allowed) return;
    if (payload.kind === "new-pane") {
      dropNewPane(layoutId, target.nodeId, target.zone, {
        paneType: payload.paneType,
        title: payload.title,
        config: payload.defaultConfig,
      });
    } else {
      moveNode(layoutId, payload.nodeId, target.nodeId, target.zone);
    }
  }

  $effect(() =>
    registerDropController({
      hover: onHover,
      commit: onCommit,
      clear: () => {
        hover = null;
      },
    }),
  );

  const frameStyle = $derived.by(() => {
    if (!hover || !canvasEl) return "display:none;";
    const canvas = canvasEl.getBoundingClientRect();
    const r = hover.rect;
    return `left:${r.left - canvas.left}px; top:${r.top - canvas.top}px; width:${r.width}px; height:${r.height}px;`;
  });

  function zoneStyle(zone: DropZone): string {
    switch (zone) {
      case "left":
        return "top:0; bottom:0; left:0; width:50%;";
      case "right":
        return "top:0; bottom:0; right:0; width:50%;";
      case "top":
        return "left:0; right:0; top:0; height:50%;";
      case "bottom":
        return "left:0; right:0; bottom:0; height:50%;";
      case "center":
        return "inset:14%;";
    }
  }
</script>

{#if hover}
  <div class="frame" style={frameStyle} aria-hidden="true">
    {#if hover.allowed}
      <div class="strip" style={zoneStyle(hover.zone)}></div>
    {:else}
      <div class="forbidden">
        <span>Can't drop here</span>
      </div>
    {/if}
  </div>
{/if}

<style>
  .frame {
    position: absolute;
    pointer-events: none;
    z-index: 50;
  }
  .strip {
    position: absolute;
    background: color-mix(in srgb, var(--color-accent) 22%, transparent);
    border: 2px solid var(--color-accent);
    border-radius: 6px;
    transition: all 0.06s ease-out;
  }
  .forbidden {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--color-danger) 14%, transparent);
    border: 2px dashed var(--color-danger);
    border-radius: 6px;
  }
  .forbidden span {
    font-size: 11px;
    font-weight: 600;
    color: var(--color-danger);
    background: var(--color-bg-main);
    padding: 2px 8px;
    border-radius: 6px;
  }
</style>
