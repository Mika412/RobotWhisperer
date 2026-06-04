<script lang="ts">
  import { X } from "@lucide/svelte";
  import type { VisualizerSettingsProps } from "../types";
  import NumberInput from "$lib/components/common/NumberInput.svelte";
  import LiveFieldTree from "../LiveFieldTree.svelte";

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

  const PALETTE = ["#ec4899", "#38bdf8", "#a3e635", "#fbbf24", "#c084fc", "#34d399"];

  let { config, source, onchange }: VisualizerSettingsProps<PlotConfig> = $props();

  const selectedPaths = $derived(new Set(config.series.map((series) => series.path)));

  function seriesId(): string {
    return `series_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
  }

  function toggleSeries(path: string) {
    const existing = config.series.find((series) => series.path === path);
    if (existing) {
      onchange({ series: config.series.filter((series) => series.id !== existing.id) });
      return;
    }
    const series: PlotSeries = {
      id: seriesId(),
      path,
      label: path,
      color: PALETTE[config.series.length % PALETTE.length],
    };
    onchange({ series: [...config.series, series] });
  }

  function patchSeries(id: string, patch: Partial<PlotSeries>) {
    onchange({
      series: config.series.map((series) => (series.id === id ? { ...series, ...patch } : series)),
    });
  }
  function cycleColor(series: PlotSeries) {
    const index = PALETTE.indexOf(series.color);
    patchSeries(series.id, { color: PALETTE[(index + 1) % PALETTE.length] });
  }
</script>

<label class="field">
  <span class="lbl">Window (seconds)</span>
  <NumberInput
    size="sm"
    min="1"
    value={config.windowSeconds}
    oninput={(event) => onchange({ windowSeconds: Number(event.currentTarget.value) || 10 })}
  />
</label>

<div class="field">
  <span class="lbl">Series: pick numeric fields from the live message</span>
  <LiveFieldTree {source} selected={selectedPaths} onpick={toggleSeries} />
</div>

{#if config.series.length > 0}
  <div class="field">
    <span class="lbl">Selected ({config.series.length})</span>
    <div class="selected-list scrollbar-custom">
      {#each config.series as series (series.id)}
        <div class="series">
          <button
            class="swatch"
            style="background: {series.color}"
            aria-label="Change colour"
            onclick={() => cycleColor(series)}
          ></button>
          <input
            class="series-label"
            value={series.label}
            oninput={(event) => patchSeries(series.id, { label: event.currentTarget.value })}
          />
          <button
            class="remove"
            aria-label="Remove series"
            onclick={() => onchange({ series: config.series.filter((s) => s.id !== series.id) })}
          >
            <X size={12} />
          </button>
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
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
  .selected-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: min(180px, 22vh);
    overflow: auto;
  }
  .series {
    display: grid;
    grid-template-columns: 22px 1fr 24px;
    align-items: center;
    gap: 6px;
    padding: 4px;
    background: var(--color-bg-input);
    border: 1px solid var(--color-border);
    border-radius: 6px;
  }
  .swatch {
    width: 22px;
    height: 18px;
    border: 1px solid var(--color-border);
    border-radius: 4px;
    cursor: pointer;
  }
  .series-label {
    background: var(--color-bg-main);
    border: 1px solid var(--color-border);
    border-radius: 4px;
    color: var(--color-text-main);
    padding: 3px 6px;
    font-size: 12px;
    font-family: var(--font-mono);
    min-width: 0;
  }
  .series-label:focus {
    outline: none;
    border-color: var(--color-accent);
  }
  .remove {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    background: transparent;
    border: 1px solid var(--color-border);
    border-radius: 4px;
    color: var(--color-text-dimmer);
    cursor: pointer;
  }
  .remove:hover {
    color: var(--color-danger);
    border-color: var(--color-danger);
  }
</style>
