import { createRing, pushSample, type RingBuffer } from "./plotBuffers";
import { parsePath, readNumber, type PathStep } from "./valuePath";

export interface PlotSeriesSpec {
  id: string;
  path: string;
}

export class PlotState {
  readonly startMs = performance.now();
  private rings = new Map<string, RingBuffer>();
  private parsed = new Map<string, PathStep[]>();

  nowSeconds(): number {
    return (performance.now() - this.startMs) / 1000;
  }

  ring(id: string): RingBuffer | undefined {
    return this.rings.get(id);
  }

  ingest(series: readonly PlotSeriesSpec[], value: unknown): void {
    if (!value) return;
    const time = this.nowSeconds();
    for (const spec of series) {
      let steps = this.parsed.get(spec.path);
      if (!steps) {
        steps = parsePath(spec.path);
        this.parsed.set(spec.path, steps);
      }
      const sample = readNumber(value, steps);
      if (sample === undefined || !Number.isFinite(sample)) continue;
      let ring = this.rings.get(spec.id);
      if (!ring) {
        ring = createRing();
        this.rings.set(spec.id, ring);
      }
      pushSample(ring, time, sample);
    }
  }
}
