<script lang="ts">
  import { slide } from "svelte/transition";
  import { ChevronDown, ChevronRight, RotateCcw, SlidersHorizontal } from "@lucide/svelte";
  import IconButton from "$lib/components/common/IconButton.svelte";
  import Slider from "$lib/components/common/Slider.svelte";
  import { formatJointValue, jointStep, type JointHandle } from "$lib/robotkit";

  let {
    joints,
    values,
    collapsed,
    oncollapse,
    onjoint,
    onreset,
  }: {
    joints: JointHandle[];
    values: Record<string, number>;
    collapsed: boolean;
    oncollapse: (collapsed: boolean) => void;
    onjoint: (name: string, value: number) => void;
    onreset: () => void;
  } = $props();
</script>

<div class="overlay" class:collapsed>
  <div class="bar">
    <button class="toggle" onclick={() => oncollapse(!collapsed)}>
      <span class="chevron">
        {#if collapsed}
          <ChevronRight size={14} />
        {:else}
          <ChevronDown size={14} />
        {/if}
      </span>
      <SlidersHorizontal size={13} class="glyph" />
      <span class="title">Joint Controls</span>
    </button>
    {#if !collapsed && joints.length > 0}
      <IconButton label="Reset pose" title="Reset pose" size="sm" onclick={onreset}>
        <RotateCcw size={13} />
      </IconButton>
    {/if}
  </div>

  {#if !collapsed}
    <div class="body" transition:slide={{ duration: 180 }}>
      {#if joints.length === 0}
        <p class="empty">This model has no controllable joints.</p>
      {:else}
        <div class="joints scrollbar-custom">
          {#each joints as joint (joint.name)}
            <Slider
              label={joint.label}
              display={formatJointValue(joint, values[joint.name] ?? joint.value)}
              value={values[joint.name] ?? joint.value}
              min={joint.lower}
              max={joint.upper}
              step={jointStep(joint)}
              oninput={(value) => onjoint(joint.name, value)}
            />
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .overlay {
    width: 100%;
    display: flex;
    flex-direction: column;
    border-radius: 12px;
    background: color-mix(in srgb, var(--color-bg-main) 70%, transparent);
    backdrop-filter: blur(16px) saturate(1.1);
    -webkit-backdrop-filter: blur(16px) saturate(1.1);
    border: 1px solid color-mix(in srgb, var(--color-text-main) 11%, transparent);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.16);
    overflow: hidden;
  }
  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 9px 9px 9px 11px;
  }
  .overlay:not(.collapsed) .bar {
    border-bottom: 1px solid color-mix(in srgb, var(--color-text-main) 8%, transparent);
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    background: transparent;
    border: 0;
    padding: 0;
    color: var(--color-text-main);
    cursor: pointer;
    min-width: 0;
  }
  .chevron {
    display: grid;
    place-items: center;
    color: var(--color-text-dimmer);
  }
  .toggle :global(.glyph) {
    color: var(--color-text-dimmer);
    flex: none;
  }
  .title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--color-text-main);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .body {
    overflow: hidden;
  }
  .joints {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 11px 11px 12px;
    max-height: min(62vh, 440px);
    overflow-y: auto;
  }
  .empty {
    margin: 0;
    padding: 11px;
    font-size: 12px;
    color: var(--color-text-dimmer);
  }
</style>
