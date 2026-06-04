<script lang="ts">
  import type { PaneComponentProps } from "$lib/dashboard/registry/paneRegistry";
  import { useTopicSource } from "$lib/visualizers/frameSource.svelte";
  import PointCloudView from "$lib/visualizers/views/PointCloudView.svelte";

  type CloudConfig = { connectionId: number | null; topic: string; pointSize: number };
  let { config }: PaneComponentProps<CloudConfig> = $props();

  const source = useTopicSource(
    () => config.connectionId ?? null,
    () => config.topic ?? "",
  );
</script>

{#if !config.topic}
  <div class="hint">
    Open the pane settings (gear) to choose a connection and MarkerArray topic.
  </div>
{:else}
  <PointCloudView {source} config={{ pointSize: config.pointSize }} />
{/if}

<style>
  .hint {
    display: grid;
    place-items: center;
    height: 100%;
    padding: 16px;
    text-align: center;
    font-size: 12px;
    color: var(--color-text-disabled);
  }
</style>
