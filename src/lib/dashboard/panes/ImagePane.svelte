<script lang="ts">
  import type { PaneComponentProps } from "$lib/dashboard/registry/paneRegistry";
  import { useTopicSource } from "$lib/visualizers/frameSource.svelte";
  import ImageView from "$lib/visualizers/views/ImageView.svelte";

  type ImageConfig = { connectionId: number | null; topic: string };
  let { config }: PaneComponentProps<ImageConfig> = $props();

  const source = useTopicSource(
    () => config.connectionId ?? null,
    () => config.topic ?? "",
  );
</script>

{#if !config.topic}
  <div class="hint">Open the pane settings (gear) to choose a connection and image topic.</div>
{:else}
  <ImageView {source} config={{}} />
{/if}

<style>
  .hint {
    padding: 16px;
    text-align: center;
    font-size: 12px;
    color: var(--color-text-disabled);
  }
</style>
