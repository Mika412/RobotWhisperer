<script lang="ts">
  import { valuePreviewText, type Value } from "$lib/core/types";
  import type { VisualizerProps } from "../types";

  let { source, overlay }: VisualizerProps = $props();

  const text = $derived.by(() => {
    if (source.value == null) return "";
    return valuePreviewText(source.value as Value);
  });
</script>

<div class="raw">
  <div class="status">
    <span
      class="status-dot {source.status === 'active'
        ? 'running'
        : source.status === 'error'
          ? 'danger'
          : ''}"
    ></span>
    <span class="label">{source.status}</span>
    {#if source.schemaName}<span class="schema">{source.schemaName}</span>{/if}
    {#if source.error}<span class="err">{source.error}</span>{/if}
  </div>
  <pre class="body scrollbar-custom">{text || "No data yet."}</pre>
  {@render overlay?.()}
</div>

<style>
  .raw {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--color-border);
    font-size: 11px;
    color: var(--color-text-dimmer);
    text-transform: capitalize;
  }
  .schema {
    color: var(--color-text-disabled);
    font-family: var(--font-mono);
    text-transform: none;
  }
  .err {
    color: var(--color-danger);
    font-family: var(--font-mono);
    text-transform: none;
  }
  .body {
    flex: 1;
    min-height: 0;
    margin: 0;
    padding: 10px;
    overflow: auto;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    color: var(--color-text-main);
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
