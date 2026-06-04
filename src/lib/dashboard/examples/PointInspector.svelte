<script lang="ts">
  import type { PaneComponentProps } from "$lib/dashboard/registry/paneRegistry";
  import { useTopicSource } from "$lib/visualizers/frameSource.svelte";
  import { parsePath, readNumber } from "$lib/visualizers/data/valuePath";
  import ActionButton from "$lib/components/editor/ActionButton.svelte";
  import { EMPTY_STRUCT } from "$lib/core/types";

  interface InspectorConfig {
    connectionId: number | null;
    topic: string;
    field: string;
    service: string;
  }

  let { config, ctx }: PaneComponentProps<InspectorConfig> = $props();

  const source = useTopicSource(
    () => config.connectionId ?? null,
    () => config.topic ?? "",
  );

  const steps = $derived(parsePath(config.field ?? ""));
  const value = $derived(config.field ? readNumber(source.value, steps) : undefined);

  const connectionId = $derived(
    config.connectionId ??
      ctx.connections.find((connection) => connection.status === "connected")?.id ??
      null,
  );

  let response = $state<string | null>(null);
  async function callService() {
    response = null;
    if (connectionId == null) throw new Error("no connection");
    const result = await ctx.callService(connectionId, config.service, EMPTY_STRUCT);
    response = JSON.stringify(result);
  }
</script>

<div class="inspector">
  {#if !config.topic}
    <p class="hint">Open the pane settings (gear) to choose a connection and topic.</p>
  {:else}
    <div class="readout">
      <div class="value">{value ?? "-"}</div>
      <div class="field">{config.field || "pick a field"}</div>
      <div class="status" class:live={source.status === "active"}>{source.status}</div>
    </div>
    {#if config.service}
      <div class="action-row">
        <ActionButton label="Call {config.service}" run={callService} />
        {#if response}<code class="resp">{response}</code>{/if}
      </div>
    {/if}
  {/if}
</div>

<style>
  .inspector {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 16px;
    gap: 16px;
  }
  .hint {
    margin: auto;
    text-align: center;
    font-size: 12px;
    color: var(--color-text-disabled);
  }
  .readout {
    margin: auto;
    text-align: center;
  }
  .value {
    font-family: var(--font-mono);
    font-size: 44px;
    font-weight: 700;
    color: var(--color-text-main);
    line-height: 1.1;
  }
  .field {
    margin-top: 6px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--color-text-dimmer);
  }
  .status {
    margin-top: 4px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-disabled);
  }
  .status.live {
    color: var(--color-accent);
  }
  .action-row {
    display: flex;
    align-items: center;
    gap: 10px;
    justify-content: center;
  }
  .resp {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--color-text-dimmer);
  }
</style>
