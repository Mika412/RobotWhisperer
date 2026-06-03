<script lang="ts">
  import { untrack } from "svelte";

  let {
    value,
    editing = false,
    oncommit,
    oneditingchange,
    placeholder = "",
  }: {
    value: string;
    editing?: boolean;
    oncommit: (next: string) => void;
    oneditingchange?: (next: boolean) => void;
    placeholder?: string;
  } = $props();

  let draft = $state("");

  $effect(() => {
    if (editing) untrack(() => (draft = value));
  });

  function begin() {
    draft = value;
    oneditingchange?.(true);
  }
  function commit() {
    oncommit(draft.trim());
    oneditingchange?.(false);
  }
  function cancel() {
    oneditingchange?.(false);
  }
</script>

{#if editing}
  <!-- svelte-ignore a11y_autofocus -->
  <input
    class="rename-input"
    bind:value={draft}
    {placeholder}
    draggable="false"
    autofocus
    onblur={commit}
    onpointerdown={(event) => event.stopPropagation()}
    onkeydown={(event) => {
      if (event.key === "Enter") commit();
      if (event.key === "Escape") cancel();
    }}
  />
{:else}
  <span
    class="rename-label"
    role="button"
    tabindex="0"
    title="Double-click to rename"
    ondblclick={begin}
    onkeydown={(event) => event.key === "F2" && begin()}
  >
    {value || placeholder}
  </span>
{/if}

<style>
  .rename-label {
    font: inherit;
    color: inherit;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
    cursor: text;
    padding: 2px 4px;
    border-radius: 3px;
  }
  .rename-label:hover {
    background: color-mix(in srgb, var(--color-bg-input) 60%, transparent);
  }
  .rename-input {
    font: inherit;
    color: var(--color-text-main);
    background: var(--color-bg-input);
    border: 1px solid var(--color-accent);
    border-radius: 3px;
    padding: 1px 4px;
    max-width: 160px;
    min-width: 0;
  }
  .rename-input:focus {
    outline: none;
  }
</style>
