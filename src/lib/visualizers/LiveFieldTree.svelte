<script lang="ts">
  import { ChevronRight, ChevronDown, Check, Plus } from "@lucide/svelte";
  import { SvelteSet } from "svelte/reactivity";
  import type { FrameSource } from "./types";
  import { buildValueTree, shapeSignature, type ValueField } from "./data/valueFieldTree";

  let {
    source,
    selected,
    onpick,
  }: {
    source: FrameSource;
    selected: Set<string>;
    onpick: (path: string) => void;
  } = $props();

  let tree = $state<ValueField[]>([]);
  let signature = "";
  const expanded = new SvelteSet<string>();

  $effect(() => {
    const next = shapeSignature(source.value);
    if (next === signature) return;
    signature = next;
    tree = buildValueTree(source.value);
  });

  function toggleExpand(path: string) {
    if (expanded.has(path)) expanded.delete(path);
    else expanded.add(path);
  }
</script>

<div class="tree scrollbar-custom">
  {#if source.status === "idle"}
    <p class="note">Choose a connection and topic first.</p>
  {:else if tree.length === 0}
    <p class="note">Waiting for a message…</p>
  {:else}
    {#each tree as node (node.path)}
      {@render row(node, 0)}
    {/each}
  {/if}
</div>

{#snippet row(node: ValueField, depth: number)}
  {@const open = expanded.has(node.path)}
  {@const isSelected = node.plottable && selected.has(node.path)}
  <div class="tree-row" class:selected={isSelected} style="padding-left: {depth * 12 + 4}px">
    <button
      class="tree-btn"
      disabled={!node.children && !node.plottable}
      onclick={() => (node.children ? toggleExpand(node.path) : onpick(node.path))}
    >
      <span class="glyph">
        {#if node.children}
          {#if open}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
        {:else if node.plottable}
          {#if isSelected}<Check size={12} />{:else}<Plus size={12} />{/if}
        {/if}
      </span>
      <span class="nm">{node.label}</span>
      <span class="ty">{node.typeLabel}</span>
    </button>
  </div>
  {#if node.children && open}
    {#each node.children as child (child.path)}
      {@render row(child, depth + 1)}
    {/each}
  {/if}
{/snippet}

<style>
  .tree {
    max-height: min(240px, 34vh);
    overflow: auto;
    border: 1px solid var(--color-border);
    border-radius: 8px;
    padding: 4px;
    background: var(--color-bg-input);
  }
  .note {
    margin: 0;
    padding: 12px 8px;
    text-align: center;
    font-size: 11px;
    color: var(--color-text-disabled);
  }
  .tree-row {
    border-radius: 4px;
  }
  .tree-row.selected {
    background: color-mix(in srgb, var(--color-accent) 16%, transparent);
  }
  .tree-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    width: 100%;
    padding: 3px 4px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--color-text-main);
    font-family: var(--font-mono);
    font-size: 11px;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .tree-btn:disabled {
    color: var(--color-text-disabled);
    cursor: default;
  }
  .tree-btn:not(:disabled):hover {
    background: var(--color-bg-hover);
  }
  .glyph {
    width: 13px;
    display: inline-flex;
    justify-content: center;
    flex: none;
    color: var(--color-text-dimmer);
  }
  .tree-row.selected .glyph {
    color: var(--color-accent);
  }
  .nm {
    flex: none;
  }
  .ty {
    margin-left: auto;
    color: var(--color-text-disabled);
    font-size: 10px;
    white-space: nowrap;
    padding-left: 6px;
  }
</style>
