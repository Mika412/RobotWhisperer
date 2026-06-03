<script lang="ts">
  import type { Snippet } from "svelte";
  import { portal } from "./portal";

  let {
    open = $bindable(false),
    align = "end",
    portalled = false,
    trigger,
    children,
  }: {
    open?: boolean;
    align?: "start" | "end";
    portalled?: boolean;
    trigger: Snippet<[{ toggle: () => void; open: boolean }]>;
    children: Snippet;
  } = $props();

  let root = $state<HTMLDivElement>();
  let panel = $state<HTMLDivElement>();
  let pos = $state<{ left: number; top: number } | null>(null);

  function toggle() {
    open = !open;
  }

  function place() {
    if (!root || !panel) return;
    const margin = 8;
    const gap = 6;
    const t = root.getBoundingClientRect();
    const p = panel.getBoundingClientRect();
    let left = align === "end" ? t.right - p.width : t.left;
    left = Math.max(margin, Math.min(left, window.innerWidth - margin - p.width));
    let top = t.bottom + gap;
    if (top + p.height > window.innerHeight - margin && t.top - gap - p.height > margin) {
      top = t.top - gap - p.height;
    }
    pos = { left, top };
  }

  $effect(() => {
    if (!open || !panel) {
      pos = null;
      return;
    }
    const id = requestAnimationFrame(place);
    window.addEventListener("scroll", place, true);
    window.addEventListener("resize", place);
    return () => {
      cancelAnimationFrame(id);
      window.removeEventListener("scroll", place, true);
      window.removeEventListener("resize", place);
    };
  });
</script>

<svelte:window
  onpointerdown={(event) => {
    if (!open) return;
    const target = event.target as Node | null;
    if (!target) return;
    if (root?.contains(target) || panel?.contains(target)) return;
    open = false;
  }}
  onkeydown={(event) => {
    if (event.key === "Escape") open = false;
  }}
/>

<div class="popover" bind:this={root}>
  {@render trigger({ toggle, open })}
  {#if open}
    {#if portalled}
      <div
        class="panel"
        bind:this={panel}
        use:portal
        style="left: {pos?.left ?? 0}px; top: {pos?.top ?? 0}px; visibility: {pos
          ? 'visible'
          : 'hidden'}"
      >
        {@render children()}
      </div>
    {:else}
      <div
        class="panel"
        bind:this={panel}
        style="left: {pos?.left ?? 0}px; top: {pos?.top ?? 0}px; visibility: {pos
          ? 'visible'
          : 'hidden'}"
      >
        {@render children()}
      </div>
    {/if}
  {/if}
</div>

<style>
  .popover {
    position: relative;
    display: inline-flex;
  }
  .panel {
    position: fixed;
    z-index: 100;
    padding: 8px;
    background: var(--color-bg-main);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    box-shadow: var(--shadow-soft, 0 10px 30px rgba(0, 0, 0, 0.4));
  }
</style>
