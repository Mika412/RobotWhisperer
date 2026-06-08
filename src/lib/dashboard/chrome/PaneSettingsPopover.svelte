<script lang="ts">
  import type { Component } from "svelte";
  import { X } from "@lucide/svelte";
  import type { PaneContext, PaneSettingsProps } from "$lib/dashboard/registry/paneRegistry";
  import { discoveryStore } from "$lib/stores/discoveryStore.svelte";
  import { useTopicSource } from "$lib/visualizers/frameSource.svelte";
  import Select from "$lib/components/common/Select.svelte";
  import IconButton from "$lib/components/common/IconButton.svelte";
  import TargetAutocomplete from "$lib/components/editor/TargetAutocomplete.svelte";

  let {
    config,
    ctx,
    settingsComponent,
    onclose,
  }: {
    config: Record<string, unknown>;
    ctx: PaneContext;
    settingsComponent?: Component<PaneSettingsProps>;
    onclose: () => void;
  } = $props();

  const bindsTopic = $derived("connectionId" in config && "topic" in config);

  const settingsSource = useTopicSource(
    () => (config.connectionId as number | null) ?? null,
    () => (config.topic as string) ?? "",
  );

  const activeConnection = $derived(
    config.connectionId != null
      ? ctx.connections.find((connection) => connection.id === config.connectionId)
      : ctx.connections.find((connection) => connection.status === "connected"),
  );

  const connectionOptions = $derived([
    { value: "auto", label: "Auto (first connected)" },
    ...ctx.connections.map((connection) => ({
      value: String(connection.id),
      label: connection.name,
    })),
  ]);

  const topicSuggestions = $derived(
    activeConnection ? discoveryStore.suggestions(activeConnection.id, "topic") : [],
  );

  function refreshTopics() {
    if (activeConnection) void discoveryStore.refresh(activeConnection.id);
  }
  $effect(refreshTopics);
</script>

<div class="settings" role="dialog" aria-label="Pane settings">
  <div class="head">
    <span class="eyebrow">Settings</span>
    <IconButton size="sm" label="Close settings" onclick={onclose}><X size={12} /></IconButton>
  </div>

  {#if bindsTopic}
    <label class="field">
      <span class="lbl">Connection</span>
      <Select
        compact
        value={config.connectionId == null ? "auto" : String(config.connectionId)}
        options={connectionOptions}
        onchange={(value) => ctx.persist({ connectionId: value === "auto" ? null : Number(value) })}
      />
    </label>
    <div class="field">
      <span class="lbl">Topic</span>
      <TargetAutocomplete
        value={(config.topic as string) ?? ""}
        suggestions={topicSuggestions}
        placeholder="/topic, pick one or type a future topic"
        onrefresh={() =>
          activeConnection && discoveryStore.refresh(activeConnection.id, { force: true })}
        onchange={(value) => ctx.persist({ topic: value })}
      />
    </div>
  {/if}

  {#if settingsComponent}
    {@const Custom = settingsComponent}
    <div class="custom">
      <Custom {config} source={settingsSource} onchange={(patch) => ctx.persist(patch)} />
    </div>
  {/if}

  {#if bindsTopic}
    <p class="hint">
      {#if activeConnection}
        Streaming via <b>{activeConnection.name}</b>{config.connectionId == null ? " (auto)" : ""}
      {:else}
        No active connection. Connect one from the sidebar.
      {/if}
    </p>
  {/if}
</div>

<style>
  .settings {
    width: 300px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .eyebrow {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-text-dimmer);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .lbl {
    font-family: var(--font-mono);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-text-dimmer);
  }
  .custom {
    padding-top: 8px;
    border-top: 1px dashed var(--color-border);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .hint {
    margin: 0;
    font-size: 11px;
    color: var(--color-text-dimmer);
  }
  .hint b {
    color: var(--color-text-main);
  }
  .field :global(.combo input) {
    height: 28px;
    font-size: 12px;
  }
</style>
