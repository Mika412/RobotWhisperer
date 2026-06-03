<script lang="ts">
  import { Check } from "@lucide/svelte";

  let {
    checked,
    onchange,
    label = "",
  }: {
    checked: boolean;
    onchange: (checked: boolean) => void;
    label?: string;
  } = $props();
</script>

<button
  type="button"
  class="checkbox"
  role="checkbox"
  aria-checked={checked}
  onclick={() => onchange(!checked)}
>
  <span class="box" class:checked>
    {#if checked}<Check size={12} strokeWidth={3} />{/if}
  </span>
  {#if label}<span class="label">{label}</span>{/if}
</button>

<style>
  .checkbox {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--color-text-main);
    font-size: 13px;
    cursor: pointer;
  }
  .box {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    flex: none;
    border: 1px solid var(--color-border-strong);
    border-radius: 5px;
    background: var(--color-bg-input);
    color: #fff;
  }
  .box.checked {
    background: var(--color-accent);
    border-color: var(--color-accent);
  }
  .label {
    user-select: none;
  }
</style>
