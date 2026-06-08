<script lang="ts">
  import { Check, Copy, Settings2 } from "@lucide/svelte";
  import { FrameKind } from "$lib/workers/decoderCore";
  import type { FrameSource } from "$lib/visualizers/types";
  import type { RequestView, ResultTab } from "$lib/core/types";
  import { matchingVisualizers, getVisualizer } from "$lib/visualizers/registry";
  import IconButton from "$lib/components/common/IconButton.svelte";
  import Popover from "$lib/components/common/Popover.svelte";
  import PlotView from "$lib/visualizers/views/PlotView.svelte";
  import PlotVizSettings from "$lib/visualizers/settings/PlotVizSettings.svelte";

  interface PlotSeries {
    id: string;
    path: string;
    label: string;
    color: string;
  }
  interface PlotConfig {
    windowSeconds: number;
    series: PlotSeries[];
  }

  const PLOT_ID = "rw.viz.plot";
  const PLOT_DEFAULT: PlotConfig = { windowSeconds: 10, series: [] };

  let {
    source,
    rawText,
    view,
    onviewchange,
  }: {
    source: FrameSource;
    rawText: string;
    view: RequestView | null;
    onviewchange: (view: RequestView) => void;
  } = $props();

  const current = $derived<RequestView>(view ?? { tab: "raw", visualizerId: null, configs: {} });
  const frameKind = $derived(source.latest?.frameKind ?? FrameKind.Value);
  const matching = $derived(matchingVisualizers(source.schemaName, frameKind));
  const vizAvailable = $derived(matching.length > 0);

  const activeTab = $derived<ResultTab>(
    current.tab === "visualize" && !vizAvailable ? "raw" : current.tab,
  );

  const selectedVizId = $derived(
    current.visualizerId && matching.some((entry) => entry.id === current.visualizerId)
      ? current.visualizerId
      : (matching[0]?.id ?? null),
  );
  const selectedViz = $derived(selectedVizId ? getVisualizer(selectedVizId) : undefined);
  const vizConfig = $derived({
    ...(selectedViz?.defaultConfig ?? {}),
    ...(selectedVizId ? current.configs[selectedVizId] : undefined),
  });
  const plotConfig = $derived<PlotConfig>({
    ...PLOT_DEFAULT,
    ...(current.configs[PLOT_ID] as Partial<PlotConfig> | undefined),
  });

  const vizSettings = $derived(
    activeTab === "visualize" ? selectedViz?.settingsComponent : undefined,
  );
  const hasSettings = $derived(activeTab === "plot" || !!vizSettings);

  const visited = $state<Record<ResultTab, boolean>>({ raw: true, visualize: false, plot: false });
  $effect(() => {
    visited[activeTab] = true;
  });

  function setTab(tab: ResultTab) {
    onviewchange({ ...current, tab });
  }
  function selectViz(id: string) {
    onviewchange({ ...current, visualizerId: id });
  }
  function patchConfig(id: string, patch: Record<string, unknown>) {
    const base = current.configs[id] ?? {};
    onviewchange({ ...current, configs: { ...current.configs, [id]: { ...base, ...patch } } });
  }

  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;
  async function copy() {
    try {
      await navigator.clipboard.writeText(rawText);
      copied = true;
      if (copyTimer) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => (copied = false), 1200);
    } catch {}
  }
</script>

<div class="result section-card">
  <div class="result-head">
    <div class="tabs" role="tablist">
      <button
        class="tab"
        class:active={activeTab === "raw"}
        role="tab"
        onclick={() => setTab("raw")}
      >
        Raw
      </button>
      <button
        class="tab"
        class:active={activeTab === "visualize"}
        role="tab"
        disabled={!vizAvailable}
        title={vizAvailable ? "" : "No visualization for this message type"}
        onclick={() => setTab("visualize")}
      >
        Visualize
      </button>
      <button
        class="tab"
        class:active={activeTab === "plot"}
        role="tab"
        onclick={() => setTab("plot")}
      >
        Plot
      </button>
    </div>

    <div class="head-actions">
      {#if activeTab === "raw" && rawText}
        <IconButton size="sm" label="Copy to clipboard" title="Copy to clipboard" onclick={copy}>
          {#if copied}<Check size={13} />{:else}<Copy size={13} />{/if}
        </IconButton>
      {/if}
      {#if hasSettings}
        <Popover align="end" portalled>
          {#snippet trigger({ toggle, open })}
            <IconButton size="sm" label="Visualization settings" active={open} onclick={toggle}>
              <Settings2 size={14} />
            </IconButton>
          {/snippet}
          <div class="settings">
            {#if activeTab === "plot"}
              <PlotVizSettings
                config={plotConfig}
                {source}
                onchange={(patch) => patchConfig(PLOT_ID, patch)}
              />
            {:else if vizSettings && selectedVizId}
              {@const VizSettings = vizSettings}
              <VizSettings
                config={vizConfig}
                {source}
                onchange={(patch) => patchConfig(selectedVizId, patch)}
              />
            {/if}
          </div>
        </Popover>
      {/if}
    </div>
  </div>

  <div class="result-body">
    <pre class="raw scrollbar-custom selectable" hidden={activeTab !== "raw"}>{rawText ||
        "No data yet. Subscribe to a topic on a connected connection."}</pre>

    {#if visited.visualize && selectedViz}
      {@const View = selectedViz.component}
      <div class="panel" hidden={activeTab !== "visualize"}>
        {#if matching.length > 1}
          <div class="subswitch">
            {#each matching as entry (entry.id)}
              <button
                class="chip"
                class:active={entry.id === selectedVizId}
                onclick={() => selectViz(entry.id)}
              >
                {entry.displayName}
              </button>
            {/each}
          </div>
        {/if}
        <div class="view">
          <View {source} config={vizConfig} />
        </div>
      </div>
    {/if}

    {#if visited.plot}
      <div class="panel" hidden={activeTab !== "plot"}>
        <div class="view">
          <PlotView {source} config={plotConfig} />
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .result {
    flex: 1;
    min-height: 200px;
    display: flex;
    flex-direction: column;
  }
  .result-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--color-border);
  }
  .tabs {
    display: flex;
    gap: 2px;
  }
  .tab {
    padding: 5px 14px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--color-text-dimmer);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .tab:hover:not(:disabled) {
    color: var(--color-text-main);
    background: var(--color-bg-hover);
  }
  .tab.active {
    background: color-mix(in srgb, var(--color-accent) 16%, transparent);
    color: var(--color-accent);
  }
  .tab:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .settings {
    width: 300px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .result-body {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .raw {
    flex: 1;
    min-height: 0;
    margin: 0;
    padding: 12px;
    overflow: auto;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    color: var(--color-text-main);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .raw[hidden],
  .panel[hidden] {
    display: none;
  }
  .panel {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .subswitch {
    display: flex;
    gap: 4px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--color-border);
  }
  .chip {
    padding: 3px 10px;
    border: 1px solid var(--color-border);
    border-radius: 999px;
    background: var(--color-bg-input);
    color: var(--color-text-dimmer);
    font-size: 11px;
    cursor: pointer;
  }
  .chip:hover {
    color: var(--color-text-main);
  }
  .chip.active {
    background: color-mix(in srgb, var(--color-accent) 18%, transparent);
    border-color: var(--color-accent);
    color: var(--color-text-main);
  }
  .view {
    flex: 1;
    min-height: 0;
  }
</style>
