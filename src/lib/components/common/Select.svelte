<script lang="ts">
  import { ChevronDown } from "@lucide/svelte";

  type Option = { value: string; label: string };
  let {
    value,
    options,
    onchange,
    placeholder = "Select…",
    disabled = false,
    compact = false,
    bare = false,
  }: {
    value: string;
    options: Option[];
    onchange: (value: string) => void;
    placeholder?: string;
    disabled?: boolean;
    compact?: boolean;
    bare?: boolean;
  } = $props();

  let open = $state(false);
  let root = $state<HTMLDivElement>();
  const selected = $derived(options.find((option) => option.value === value));

  function choose(next: string) {
    open = false;
    if (next !== value) onchange(next);
  }
</script>

<svelte:window
  onclick={(event) => {
    if (root && !root.contains(event.target as Node)) open = false;
  }}
/>

<div class="select" bind:this={root}>
  <button
    type="button"
    class="trigger"
    class:compact
    class:bare
    {disabled}
    aria-haspopup="listbox"
    aria-expanded={open}
    onclick={() => (open = !open)}
  >
    <span class="value" class:placeholder={!selected}>{selected?.label ?? placeholder}</span>
    <ChevronDown size={15} class="chev" />
  </button>
  {#if open}
    <ul class="menu scrollbar-custom" role="listbox" tabindex="-1">
      {#each options as option (option.value)}
        <li>
          <button
            type="button"
            class="option"
            class:active={option.value === value}
            role="option"
            aria-selected={option.value === value}
            onclick={() => choose(option.value)}
          >
            {option.label}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .select {
    position: relative;
    width: 100%;
  }
  .trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    height: 40px;
    padding: 0 10px;
    border: 1px solid var(--color-border);
    border-radius: 8px;
    background: var(--color-bg-input);
    color: var(--color-text-main);
    font-size: 13px;
    cursor: pointer;
  }
  .trigger.compact {
    height: 28px;
    padding: 0 8px;
    font-size: 12px;
  }
  .trigger.bare {
    border-color: transparent;
    border-radius: 0;
    background: transparent;
  }
  .trigger:hover:not(:disabled) {
    border-color: var(--color-border-strong);
  }
  .trigger.bare:hover:not(:disabled) {
    border-color: transparent;
    color: var(--color-text-main);
  }
  .trigger:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .value.placeholder {
    color: var(--color-text-disabled);
  }
  .trigger :global(.chev) {
    color: var(--color-text-dimmer);
    flex: none;
  }
  .menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    z-index: 30;
    max-height: 280px;
    overflow-y: auto;
    margin: 0;
    padding: 4px;
    list-style: none;
    background: var(--color-bg-main);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    box-shadow: var(--shadow-soft, 0 10px 30px rgba(0, 0, 0, 0.4));
  }
  .option {
    width: 100%;
    text-align: left;
    padding: 7px 8px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--color-text-main);
    font-size: 13px;
    cursor: pointer;
  }
  .option:hover {
    background: var(--color-bg-hover);
  }
  .option.active {
    background: color-mix(in srgb, var(--color-accent) 16%, transparent);
  }
</style>
