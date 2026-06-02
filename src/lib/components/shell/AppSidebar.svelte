<script lang="ts">
  import { Plus, Inbox, Trash2, LayoutGrid, Upload, Download } from "@lucide/svelte";
  import { requestsStore } from "$lib/stores/requestsStore.svelte";
  import { dashboardStore } from "$lib/stores/dashboardStore.svelte";
  import { tabsStore } from "$lib/stores/tabsStore.svelte";
  import { subscriptionStore } from "$lib/stores/subscriptionStore.svelte";
  import { downloadWorkspace, importWorkspaceFile } from "$lib/stores/workspaceIo";
  import TypeBadge from "$lib/components/TypeBadge.svelte";
  import IconButton from "$lib/components/common/IconButton.svelte";
  import ConnectionFooter from "./ConnectionFooter.svelte";

  let importInput = $state<HTMLInputElement>();
  let ioStatus = $state<string | null>(null);

  async function exportWorkspace() {
    try {
      await downloadWorkspace();
      ioStatus = "Exported workspace";
    } catch (error) {
      ioStatus = `Export failed: ${error instanceof Error ? error.message : error}`;
    }
  }

  async function onImportFile(event: Event) {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    if (!file) return;
    try {
      const report = await importWorkspaceFile(file, "merge");
      ioStatus = `Imported ${report.requests_added} requests, ${report.connections_added} connections`;
    } catch (error) {
      ioStatus = `Import failed: ${error instanceof Error ? error.message : error}`;
    } finally {
      if (importInput) importInput.value = "";
    }
  }

  async function addRequest() {
    const request = await requestsStore.create();
    tabsStore.openRequest(request.id);
  }

  function deleteRequest(id: number, event: Event) {
    event.stopPropagation();
    tabsStore.closeRequest(id);
    void requestsStore.remove(id);
  }

  function addDashboard() {
    const dashboard = dashboardStore.create();
    tabsStore.openDashboard(dashboard.id);
  }

  function deleteDashboard(id: string, event: Event) {
    event.stopPropagation();
    tabsStore.closeDashboard(id);
    dashboardStore.remove(id);
  }
</script>

<aside class="app-sidebar">
  <div class="workspace-bar">
    <span class="ws-label">WORKSPACE</span>
    <div class="ws-actions">
      <IconButton
        size="sm"
        label="Import workspace"
        title="Import workspace"
        onclick={() => importInput?.click()}
      >
        <Upload size={14} />
      </IconButton>
      <IconButton
        size="sm"
        label="Export workspace"
        title="Export workspace"
        onclick={exportWorkspace}
      >
        <Download size={14} />
      </IconButton>
    </div>
    <input
      bind:this={importInput}
      class="hidden-input"
      type="file"
      accept="application/json,.json"
      onchange={onImportFile}
    />
  </div>
  {#if ioStatus}
    <div class="ws-status" role="status">{ioStatus}</div>
  {/if}

  <div class="section">
    <div class="section-header">
      <span class="section-label">REQUESTS</span>
      <IconButton size="sm" label="New request" title="New request" onclick={addRequest}>
        <Plus size={14} />
      </IconButton>
    </div>
    <div class="section-body scrollbar-custom">
      {#each requestsStore.requests as request (request.id)}
        <div
          class="leaf-row"
          class:is-active={tabsStore.isActive(`request:${request.id}`)}
          role="button"
          tabindex="0"
          onclick={() => tabsStore.openRequest(request.id)}
          onkeydown={(event) => event.key === "Enter" && tabsStore.openRequest(request.id)}
        >
          <TypeBadge kind={request.kind} />
          <span class="leaf-name">{request.name}</span>
          {#if subscriptionStore.isActive(request.id)}<span class="status-dot running"></span>{/if}
          <button
            class="leaf-trash"
            title="Delete request"
            aria-label="Delete request"
            onclick={(event) => deleteRequest(request.id, event)}
          >
            <Trash2 size={12} />
          </button>
        </div>
      {:else}
        <div class="empty">
          <Inbox size={28} />
          <p>No requests yet. Click + to create one.</p>
        </div>
      {/each}
    </div>
  </div>

  <div class="section section-dashboards">
    <div class="section-header">
      <span class="section-label">DASHBOARDS</span>
      <IconButton size="sm" label="New dashboard" title="New dashboard" onclick={addDashboard}>
        <Plus size={14} />
      </IconButton>
    </div>
    <div class="section-body scrollbar-custom">
      {#each dashboardStore.layouts as dashboard (dashboard.id)}
        <div
          class="leaf-row"
          class:is-active={tabsStore.isActive(`dashboard:${dashboard.id}`)}
          role="button"
          tabindex="0"
          onclick={() => tabsStore.openDashboard(dashboard.id)}
          onkeydown={(event) => event.key === "Enter" && tabsStore.openDashboard(dashboard.id)}
        >
          <LayoutGrid size={15} class="dashboard-icon" />
          <span class="leaf-name">{dashboard.title}</span>
          <button
            class="leaf-trash"
            title="Delete dashboard"
            aria-label="Delete dashboard"
            onclick={(event) => deleteDashboard(dashboard.id, event)}
          >
            <Trash2 size={12} />
          </button>
        </div>
      {/each}
    </div>
  </div>

  <ConnectionFooter />
</aside>

<style>
  .app-sidebar {
    width: 240px;
    height: 100%;
    background: var(--color-bg-sidebar);
    border-right: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    min-height: 0;
  }
  .workspace-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px 6px;
    border-bottom: 1px solid var(--color-border);
  }
  .ws-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--color-text-disabled);
    font-family: var(--font-mono);
  }
  .ws-actions {
    display: flex;
    gap: 2px;
  }
  .hidden-input {
    display: none;
  }
  .ws-status {
    padding: 4px 10px;
    font-size: 11px;
    color: var(--color-text-dimmer);
    border-bottom: 1px solid var(--color-border);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .section {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    padding: 6px 8px;
  }
  .section-dashboards {
    flex: 0 1 auto;
    max-height: 45%;
    border-top: 1px solid var(--color-border);
  }
  :global(.dashboard-icon) {
    color: var(--color-accent);
    flex: none;
  }
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 6px 8px;
  }
  .section-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-text-disabled);
    font-family: var(--font-mono);
  }
  .section-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .leaf-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 6px;
    width: 100%;
    color: var(--color-text-dimmer);
    font-size: 13px;
    cursor: pointer;
    border-radius: 6px;
    text-align: left;
  }
  .leaf-row:hover {
    background: var(--color-bg-hover);
    color: var(--color-text-main);
  }
  .leaf-row.is-active {
    background: color-mix(in srgb, var(--color-accent) 16%, transparent);
    color: var(--color-text-main);
  }
  .leaf-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .leaf-trash {
    background: transparent;
    border: 0;
    color: var(--color-text-disabled);
    padding: 2px;
    border-radius: 4px;
    cursor: pointer;
    visibility: hidden;
  }
  .leaf-row:hover .leaf-trash {
    visibility: visible;
  }
  .leaf-trash:hover {
    color: var(--color-danger);
    background: color-mix(in srgb, var(--color-danger) 14%, transparent);
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 24px 12px;
    text-align: center;
    font-size: 12px;
    color: var(--color-text-disabled);
  }
</style>
