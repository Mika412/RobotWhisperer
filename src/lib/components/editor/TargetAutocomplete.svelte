<script lang="ts">
  import type { TargetSuggestion } from "$lib/stores/discoveryStore.svelte";

  let {
    value = $bindable(),
    suggestions,
    placeholder = "",
    onrefresh,
    onchange,
    disabled = false,
    bare = false,
  }: {
    value: string;
    suggestions: TargetSuggestion[];
    placeholder?: string;
    onrefresh?: () => void;
    onchange?: (value: string) => void;
    disabled?: boolean;
    bare?: boolean;
  } = $props();

  let open = $state(false);
  let node = $state<HTMLDivElement>();

  const filtered = $derived(
    value.trim()
      ? suggestions.filter((entry) => entry.name.toLowerCase().includes(value.toLowerCase()))
      : suggestions,
  );

  function pick(name: string) {
    value = name;
    open = false;
    onchange?.(name);
  }

  function commit() {
    if (onchange) onchange(value.trim());
  }
</script>

<svelte:window
  onclick={(event) => {
    if (node && !node.contains(event.target as Node)) open = false;
  }}
/>

<div class="combo" bind:this={node}>
  <input
    class:bare
    bind:value
    {placeholder}
    {disabled}
    spellcheck="false"
    autocomplete="off"
    onfocus={() => {
      if (disabled) return;
      open = true;
      onrefresh?.();
    }}
    oninput={() => {
      open = true;
      onchange?.(value);
    }}
    onblur={commit}
    onkeydown={(event) => {
      if (event.key === "Enter") {
        open = false;
        commit();
      }
    }}
  />
  {#if open && !disabled && filtered.length > 0}
    <ul class="menu scrollbar-custom">
      {#each filtered.slice(0, 50) as entry (entry.name)}
        <li>
          <button type="button" onclick={() => pick(entry.name)}>
            <span class="name">{entry.name}</span>
            <span class="schema">{entry.schemaName}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .combo {
    position: relative;
    flex: 1;
    min-width: 0;
  }
  input {
    width: 100%;
    height: 40px;
    padding: 0 12px;
    border-radius: 8px;
    border: 1px solid var(--color-border);
    background: var(--color-bg-input);
    color: var(--color-text-main);
    font-family: var(--font-mono);
    font-size: 13px;
  }
  input:focus {
    outline: none;
    border-color: var(--color-accent);
  }
  input.bare {
    border-color: transparent;
    border-radius: 0;
    background: transparent;
  }
  input.bare:focus {
    border-color: transparent;
  }
  input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    z-index: 20;
    max-height: 260px;
    overflow-y: auto;
    margin: 0;
    padding: 4px;
    list-style: none;
    background: var(--color-bg-main);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    box-shadow: var(--shadow-soft, 0 10px 30px rgba(0, 0, 0, 0.4));
  }
  .menu button {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
    padding: 6px 8px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    cursor: pointer;
    text-align: left;
  }
  .menu button:hover {
    background: var(--color-bg-hover);
  }
  .name {
    color: var(--color-text-main);
    font-family: var(--font-mono);
    font-size: 13px;
  }
  .schema {
    color: var(--color-text-disabled);
    font-family: var(--font-mono);
    font-size: 11px;
    white-space: nowrap;
  }
</style>
