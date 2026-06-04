<script lang="ts">
  import type { PaneComponentProps } from "$lib/dashboard/registry/paneRegistry";
  import { useTopicSource } from "$lib/visualizers/frameSource.svelte";
  import PlotView from "$lib/visualizers/views/PlotView.svelte";

  interface PlotSeries {
    id: string;
    path: string;
    label: string;
    color: string;
  }
  interface PlotConfig {
    connectionId: number | null;
    topic: string;
    windowSeconds: number;
    series: PlotSeries[];
  }

  let { config }: PaneComponentProps<PlotConfig> = $props();

  const source = useTopicSource(
    () => config.connectionId ?? null,
    () => config.topic ?? "",
  );
</script>

{#if !config.topic}
  <div class="hint">Open the pane settings (gear) to choose a topic and add field series.</div>
{:else}
  <PlotView {source} config={{ windowSeconds: config.windowSeconds, series: config.series }} />
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
