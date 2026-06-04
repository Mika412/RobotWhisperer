<script lang="ts">
  import { Settings, Maximize2, Minimize2, X } from "@lucide/svelte";
  import { getPane, type PaneContext } from "$lib/dashboard/registry/paneRegistry";
  import PaneSettingsPopover from "$lib/dashboard/chrome/PaneSettingsPopover.svelte";
  import IconButton from "$lib/components/common/IconButton.svelte";
  import Popover from "$lib/components/common/Popover.svelte";
  import type { PaneNode } from "$lib/dashboard/layout/layout";

  let {
    pane,
    ctx,
    maximized,
    onmaximize,
    onclose,
  }: {
    pane: PaneNode;
    ctx: PaneContext;
    maximized: boolean;
    onmaximize: () => void;
    onclose: () => void;
  } = $props();

  const descriptor = $derived(getPane(pane.paneType));

  let settingsOpen = $state(false);
</script>

<div class="actions">
  <Popover bind:open={settingsOpen} align="end" portalled>
    {#snippet trigger({ toggle })}
      <IconButton
        active={settingsOpen}
        title="Pane settings"
        label="Pane settings"
        draggable="false"
        onclick={toggle}
      >
        <Settings size={14} />
      </IconButton>
    {/snippet}
    <PaneSettingsPopover
      {ctx}
      config={pane.config}
      settingsComponent={descriptor?.settingsComponent}
      onclose={() => (settingsOpen = false)}
    />
  </Popover>
  <IconButton
    title={maximized ? "Restore" : "Maximize"}
    label={maximized ? "Restore" : "Maximize"}
    draggable="false"
    onclick={onmaximize}
  >
    {#if maximized}<Minimize2 size={14} />{:else}<Maximize2 size={14} />{/if}
  </IconButton>
  <IconButton
    tone="danger"
    title="Close pane"
    label="Close pane"
    draggable="false"
    onclick={onclose}
  >
    <X size={14} />
  </IconButton>
</div>

<style>
  .actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: none;
  }
</style>
