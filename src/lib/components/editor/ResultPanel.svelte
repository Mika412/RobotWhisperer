<script lang="ts">
  import type { Snippet } from "svelte";
  import { Check, Copy } from "@lucide/svelte";
  import IconButton from "$lib/components/common/IconButton.svelte";

  let {
    label,
    text = "",
    placeholder = "-",
    actions,
  }: {
    label: string;
    text?: string;
    placeholder?: string;
    actions?: Snippet;
  } = $props();

  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;

  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      if (copyTimer) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => (copied = false), 1200);
    } catch {}
  }
</script>

<div class="result section-card">
  <div class="result-head">
    <span class="section-label">{label}</span>
    <div class="head-actions">
      {@render actions?.()}
      {#if text}
        <IconButton size="sm" label="Copy to clipboard" title="Copy to clipboard" onclick={copy}>
          {#if copied}<Check size={13} />{:else}<Copy size={13} />{/if}
        </IconButton>
      {/if}
    </div>
  </div>
  <pre class="result-body scrollbar-custom selectable">{text || placeholder}</pre>
</div>

<style>
  .result {
    flex: 1;
    min-height: 120px;
    display: flex;
    flex-direction: column;
  }
  .result-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 8px 6px 12px;
    border-bottom: 1px solid var(--color-border);
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .result-body {
    flex: 1;
    min-height: 0;
    margin: 0;
    padding: 12px;
    overflow: auto;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    color: var(--color-text-main);
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
