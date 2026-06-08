<script lang="ts">
  import uPlot from "uplot";
  import "uplot/dist/uPlot.min.css";
  import type { VisualizerProps } from "../types";
  import { PlotState } from "../data/plotState";
  import { snapshot, earliestTime, computeRange } from "../data/plotBuffers";

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

  let { source, config, overlay }: VisualizerProps<PlotConfig> = $props();

  const plotState = new PlotState();

  $effect(() => source.onFrame((frame) => plotState.ingest(config.series, frame.json)));

  let host = $state<HTMLDivElement>();
  let plot: uPlot | null = null;
  let raf = 0;
  let lastRenderMs = 0;
  let renderedSeriesKey = "";

  function cssVar(name: string, fallback: string): string {
    const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    return value || fallback;
  }

  function options(width: number, height: number): uPlot.Options {
    const axis = cssVar("--color-text-disabled", "#475569");
    const grid = cssVar("--color-border", "rgba(148,163,184,0.1)");
    return {
      width: Math.max(40, width),
      height: Math.max(40, height),
      padding: [10, 12, 6, 4],
      scales: { x: { time: false } },
      cursor: { show: false },
      legend: { show: false },
      select: { show: false } as unknown as uPlot.Options["select"],
      axes: [
        {
          stroke: axis,
          grid: { stroke: grid },
          ticks: { show: false },
          size: 24,
          values: (_u, values) => values.map((v) => `${v.toFixed(1)}s`),
          font: "10px var(--font-mono, monospace)",
        },
        {
          stroke: axis,
          grid: { stroke: grid },
          ticks: { show: false },
          size: 38,
          font: "10px var(--font-mono, monospace)",
        },
      ],
      series: [
        { label: "t" },
        ...config.series.map((series) => ({
          label: series.label || series.path,
          stroke: series.color,
          width: 1.5,
          points: { show: false },
        })),
      ],
    };
  }

  function rebuild() {
    if (!host) return;
    if (plot) {
      plot.destroy();
      plot = null;
    }
    const rect = host.getBoundingClientRect();
    const empty: number[][] = [[], ...config.series.map(() => [])];
    plot = new uPlot(options(rect.width, rect.height), empty as uPlot.AlignedData, host);
    renderedSeriesKey = seriesKey();
  }

  function seriesKey(): string {
    return config.series.map((series) => `${series.id}:${series.color}:${series.label}`).join("|");
  }

  function render() {
    raf = requestAnimationFrame(render);
    if (!plot || !host || host.offsetParent === null) return;
    if (seriesKey() !== renderedSeriesKey) {
      rebuild();
      return;
    }
    const nowMs = performance.now();
    if (nowMs - lastRenderMs < 33) return;
    lastRenderMs = nowMs;

    const nowSec = plotState.nowSeconds();
    const window = config.windowSeconds || 10;
    const columns: number[][] = [];
    let xs: number[] = [];
    let earliest = Infinity;
    for (const series of config.series) {
      const ring = plotState.ring(series.id);
      if (!ring) {
        columns.push([]);
        continue;
      }
      const slice = snapshot(ring, window, nowSec);
      columns.push(slice.value);
      if (slice.time.length > xs.length) xs = slice.time;
      const first = earliestTime(ring);
      if (first !== null && first < earliest) earliest = first;
    }

    let xMin = earliest === Infinity ? nowSec - window : Math.max(nowSec - window, earliest);
    if (nowSec - xMin < 0.5) xMin = nowSec - 0.5;

    const aligned: number[][] = [xs];
    for (const column of columns) {
      if (column.length === xs.length) {
        aligned.push(column);
      } else {
        const padded = new Array<number>(xs.length).fill(NaN);
        const offset = xs.length - column.length;
        for (let i = 0; i < column.length; i += 1) padded[offset + i] = column[i];
        aligned.push(padded);
      }
    }

    plot.setData(aligned as unknown as uPlot.AlignedData, false);
    plot.setScale("x", { min: xMin, max: nowSec });
    const range = computeRange(columns);
    if (range) plot.setScale("y", range);
  }

  $effect(() => {
    rebuild();
    raf = requestAnimationFrame(render);
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        if (plot && width > 10 && height > 10) plot.setSize({ width, height });
      }
    });
    if (host) observer.observe(host);
    return () => {
      observer.disconnect();
      cancelAnimationFrame(raf);
      if (plot) {
        plot.destroy();
        plot = null;
      }
    };
  });
</script>

<div class="plot">
  {#if config.series.length === 0}
    <div class="hint">No series yet. Add numeric fields in settings.</div>
  {/if}
  <div class="canvas" bind:this={host}></div>
  {#if config.series.length > 0}
    <div class="legend">
      {#each config.series as series (series.id)}
        <span class="entry" title="{series.label || series.path} · {series.path}">
          <span class="swatch" style="background: {series.color}"></span>
          <span class="name">{series.label || series.path}</span>
        </span>
      {/each}
    </div>
  {/if}
  {#if source.status === "error"}
    <div class="err">{source.error}</div>
  {/if}
  {@render overlay?.()}
</div>

<style>
  .plot {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--color-bg-deep, var(--color-bg-main));
  }
  .hint {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    text-align: center;
    font-size: 12px;
    color: var(--color-text-disabled);
    pointer-events: none;
    z-index: 1;
  }
  .canvas {
    flex: 1;
    min-height: 0;
    min-width: 0;
    padding: 2px 4px 4px;
  }
  .legend {
    position: absolute;
    top: 6px;
    right: 8px;
    z-index: 2;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
    max-width: 50%;
    opacity: 0.8;
    transition: opacity 0.15s ease;
  }
  .legend:hover {
    opacity: 1;
    flex-direction: column;
    flex-wrap: nowrap;
    align-items: flex-end;
    max-width: 80%;
    max-height: calc(100% - 12px);
    overflow-y: auto;
    gap: 3px;
  }
  .entry {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    max-width: 100%;
  }
  .legend:hover .entry {
    padding: 1px 7px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--color-bg-deep) 70%, transparent);
    backdrop-filter: blur(3px);
  }
  .swatch {
    width: 11px;
    height: 3px;
    flex: none;
    border-radius: 1px;
  }
  .name {
    display: none;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--color-text-dimmer);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .legend:hover .name {
    display: inline;
  }
  .err {
    position: absolute;
    bottom: 6px;
    left: 8px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-danger);
  }
  :global(.canvas .u-wrap) {
    background: transparent;
  }
</style>
