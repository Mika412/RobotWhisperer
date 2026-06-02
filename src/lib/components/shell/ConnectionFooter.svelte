<script lang="ts">
  import { ChevronUp, Settings, Plus, Power, PowerOff } from "@lucide/svelte";
  import { connectionStore } from "$lib/stores/connectionStore.svelte";
  import { modalStore } from "$lib/stores/modalStore.svelte";
  import ConnectionEditorModal from "$lib/components/connections/ConnectionEditorModal.svelte";
  import SettingsModal from "$lib/components/modals/SettingsModal.svelte";
  import IconButton from "$lib/components/common/IconButton.svelte";
  import type { Connection, TransportConfig, TransportStatus } from "$lib/core/types";

  let open = $state(false);

  const primary = $derived(
    connectionStore.connections.find((c) => connectionStore.status(c.id) === "connected") ??
      connectionStore.connections[0] ??
      null,
  );

  function chip(connection: Connection | null): { dot: string; text: string } {
    if (!connection) return { dot: "", text: "No connections" };
    const status = connectionStore.status(connection.id);
    if (status === "connected") return { dot: "running", text: connection.name };
    if (status === "connecting") return { dot: "dirty", text: `${connection.name} (connecting)` };
    if (status === "failed") return { dot: "danger", text: `${connection.name} (failed)` };
    return { dot: "", text: connection.name };
  }

  function dotFor(status: TransportStatus): string {
    if (status === "connected") return "running";
    if (status === "connecting" || status === "reconnecting") return "dirty";
    if (status === "failed") return "danger";
    return "";
  }

  function urlPreview(config: TransportConfig): string {
    switch (config.kind) {
      case "foxglove_ws":
        return config.url;
      case "rosbridge":
        return config.url;
      case "native_ros2":
        return `domain ${config.domain_id} · native`;
      case "dummy":
        return "in-process simulator";
    }
  }

  function toggle(connection: Connection) {
    const status = connectionStore.status(connection.id);
    const live = status === "connected" || status === "connecting";
    if (live) void connectionStore.deactivate(connection.id);
    else void connectionStore.activate(connection.id).catch(() => {});
  }

  function editConnection(connection?: Connection) {
    open = false;
    modalStore.open(ConnectionEditorModal, { connection });
  }
</script>

<svelte:window
  onclick={(event) => {
    if (!open) return;
    const target = event.target as HTMLElement | null;
    if (target?.closest(".conn-footer")) return;
    open = false;
  }}
/>

<div class="conn-footer">
  {#if open}
    <div class="panel">
      <div class="panel-header"><span class="section-label">CONNECTIONS</span></div>
      <div class="panel-body scrollbar-custom">
        {#each connectionStore.connections as connection (connection.id)}
          {@const status = connectionStore.status(connection.id)}
          {@const live = status === "connected" || status === "connecting"}
          <div
            class="row"
            role="button"
            tabindex="0"
            onclick={() => editConnection(connection)}
            onkeydown={(event) => event.key === "Enter" && editConnection(connection)}
          >
            <span class="status-dot {dotFor(status)}"></span>
            <div class="row-text">
              <div class="row-name">{connection.name}</div>
              {#if status === "failed" && connectionStore.error(connection.id)}
                <div class="row-meta error">{connectionStore.error(connection.id)}</div>
              {:else}
                <div class="row-meta">{urlPreview(connection.config)}</div>
              {/if}
            </div>
            <button
              class="row-action"
              title={live ? "Disconnect" : "Connect"}
              onclick={(event) => {
                event.stopPropagation();
                toggle(connection);
              }}
            >
              {#if live}<PowerOff size={14} />{:else}<Power size={14} />{/if}
            </button>
          </div>
        {/each}
        <button class="add-btn" onclick={() => editConnection()}>
          <Plus size={14} /> Add connection
        </button>
      </div>
    </div>
  {/if}

  <button class="chip" onclick={() => (open = !open)}>
    <span class="status-dot {chip(primary).dot}"></span>
    <span class="chip-name">{chip(primary).text}</span>
    <ChevronUp size={14} class="chip-chev" style="transform: rotate({open ? 0 : 180}deg);" />
  </button>
  <IconButton
    title="Settings"
    label="Settings"
    onclick={() => {
      open = false;
      modalStore.open(SettingsModal);
    }}
  >
    <Settings size={14} />
  </IconButton>
</div>

<style>
  .conn-footer {
    position: relative;
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    border-top: 1px solid var(--color-border);
    background: var(--color-bg-sidebar);
    flex-shrink: 0;
  }
  .chip {
    grid-column: 1;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--color-text-main);
    font-size: 13px;
    cursor: pointer;
    min-width: 0;
  }
  .chip:hover {
    background: var(--color-bg-hover);
    border-color: var(--color-border);
  }
  .chip-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
  }
  .chip :global(.chip-chev) {
    color: var(--color-text-disabled);
    transition: transform 0.12s;
    flex: none;
  }
  .panel {
    position: absolute;
    left: 8px;
    right: 8px;
    bottom: calc(100% + 4px);
    background: var(--color-bg-main);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    box-shadow: var(--shadow-soft, 0 10px 30px rgba(0, 0, 0, 0.4));
    z-index: 50;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .panel-header {
    padding: 8px 10px;
    border-bottom: 1px solid var(--color-border);
  }
  .section-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-text-disabled);
    font-family: var(--font-mono);
  }
  .panel-body {
    padding: 4px;
    overflow-y: auto;
    max-height: 280px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-radius: 6px;
    cursor: pointer;
  }
  .row:hover {
    background: var(--color-bg-hover);
  }
  .row-text {
    flex: 1;
    min-width: 0;
  }
  .row-name {
    font-size: 13px;
    color: var(--color-text-main);
  }
  .row-meta {
    font-size: 11px;
    color: var(--color-text-disabled);
    font-family: var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .row-meta.error {
    color: var(--color-danger);
    white-space: normal;
  }
  .row-action {
    background: transparent;
    border: 0;
    color: var(--color-text-disabled);
    padding: 4px;
    border-radius: 4px;
    cursor: pointer;
  }
  .row-action:hover {
    background: var(--color-bg-input);
    color: var(--color-text-main);
  }
  .add-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    margin-top: 4px;
    padding: 7px 8px;
    background: transparent;
    border: 1px dashed var(--color-border);
    border-radius: 6px;
    color: var(--color-text-dimmer);
    font-size: 12px;
    cursor: pointer;
  }
  .add-btn:hover {
    color: var(--color-accent);
    border-color: var(--color-accent);
    background: color-mix(in srgb, var(--color-accent) 6%, transparent);
  }
</style>
