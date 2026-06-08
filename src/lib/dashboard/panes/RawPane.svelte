<script lang="ts">
  import type { PaneComponentProps } from "$lib/dashboard/registry/paneRegistry";
  import { useTopicSource } from "$lib/visualizers/frameSource.svelte";
  import RawView from "$lib/visualizers/views/RawView.svelte";

  type RawConfig = { connectionId: number | null; topic: string };
  let { config }: PaneComponentProps<RawConfig> = $props();

  const source = useTopicSource(
    () => config.connectionId ?? null,
    () => config.topic ?? "",
  );
</script>

{#if !config.topic}
  <div class="hint">Open the pane settings (gear) to choose a connection and topic.</div>
{:else}
  <RawView {source} config={{}} />
{/if}

<style>
  .hint {
    padding: 16px;
    font-size: 12px;
    color: var(--color-text-disabled);
  }
</style>
