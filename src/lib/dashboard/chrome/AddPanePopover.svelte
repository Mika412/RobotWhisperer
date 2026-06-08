<script lang="ts">
  import {
    LineChart,
    Camera,
    Boxes,
    ScrollText,
    Square,
    Hand,
    Bot,
    type Icon,
  } from "@lucide/svelte";
  import { panesByGroup, type PaneDescriptor } from "$lib/dashboard/registry/paneRegistry";
  import { draggable } from "$lib/dashboard/layout/dragState.svelte";

  let {
    onpick,
  }: {
    onpick: (paneType: string, displayName: string, defaultConfig: Record<string, unknown>) => void;
  } = $props();

  const groups = $derived(panesByGroup());

  function iconFor(descriptor: PaneDescriptor): typeof Icon {
    const type = descriptor.type.toLowerCase();
    if (type.includes("plot")) return LineChart;
    if (type.includes("image")) return Camera;
    if (type.includes("point") || type.includes("cloud")) return Boxes;
    if (type.includes("raw")) return ScrollText;
    if (type.includes("robot")) return Bot;
    if (type.includes("hand") || type.includes("finger")) return Hand;
    return Square;
  }
</script>

<div class="add-pane scrollbar-custom" role="menu">
  {#each groups as { group, panes } (group)}
    <div class="eyebrow">{group}</div>
    <div class="grid">
      {#each panes as pane (pane.type)}
        {@const PaneIcon = iconFor(pane)}
        <button
          class="tile"
          title={pane.description}
          use:draggable={{
            payload: () => ({
              kind: "new-pane",
              paneType: pane.type,
              defaultConfig: pane.defaultConfig as Record<string, unknown>,
              title: pane.displayName,
            }),
            label: () => pane.displayName,
          }}
          onclick={() =>
            onpick(pane.type, pane.displayName, pane.defaultConfig as Record<string, unknown>)}
        >
          <PaneIcon size={16} />
          <div class="meta">
            <div class="name">{pane.displayName}</div>
            <div class="kind">{pane.category}</div>
          </div>
        </button>
      {/each}
    </div>
  {/each}
  <p class="tip">
    Drag a tile onto the canvas to dock it; drop one onto a pane's centre to tab them together.
  </p>
</div>

<style>
  .add-pane {
    width: 320px;
    max-height: 70vh;
    overflow-y: auto;
  }
  .grid + .eyebrow {
    margin-top: 12px;
  }
  .eyebrow {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--color-text-disabled);
    font-family: var(--font-mono);
    text-transform: uppercase;
    margin-bottom: 8px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .tile {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    background: var(--color-bg-input);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    color: var(--color-text-main);
    cursor: pointer;
    text-align: left;
  }
  .tile:hover {
    border-color: var(--color-accent);
    background: color-mix(in srgb, var(--color-accent) 6%, var(--color-bg-input));
  }
  .meta {
    flex: 1;
    min-width: 0;
  }
  .name {
    font-size: 13px;
    font-weight: 600;
  }
  .kind {
    font-size: 11px;
    color: var(--color-text-disabled);
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .tip {
    margin: 10px 0 0;
    font-size: 11px;
    color: var(--color-text-disabled);
  }
</style>
