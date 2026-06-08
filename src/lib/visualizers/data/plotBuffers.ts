export interface RingBuffer {
  time: Float64Array;
  value: Float64Array;
  writeIndex: number;
  total: number;
}

export const DEFAULT_CAPACITY = 4096;

export function createRing(capacity = DEFAULT_CAPACITY): RingBuffer {
  return {
    time: new Float64Array(capacity),
    value: new Float64Array(capacity),
    writeIndex: 0,
    total: 0,
  };
}

export function pushSample(ring: RingBuffer, time: number, value: number): void {
  const capacity = ring.time.length;
  ring.time[ring.writeIndex] = time;
  ring.value[ring.writeIndex] = value;
  ring.writeIndex = (ring.writeIndex + 1) % capacity;
  ring.total += 1;
}

export function snapshot(
  ring: RingBuffer,
  windowSeconds: number,
  nowSeconds: number,
): { time: number[]; value: number[] } {
  const capacity = ring.time.length;
  const length = Math.min(ring.total, capacity);
  const time: number[] = [];
  const value: number[] = [];
  if (length === 0) return { time, value };
  const start = ring.total <= capacity ? 0 : ring.writeIndex;
  const cutoff = nowSeconds - windowSeconds;
  for (let i = 0; i < length; i += 1) {
    const index = (start + i) % capacity;
    const t = ring.time[index];
    if (t < cutoff) continue;
    time.push(t);
    value.push(ring.value[index]);
  }
  return { time, value };
}

export function earliestTime(ring: RingBuffer): number | null {
  if (ring.total === 0) return null;
  const capacity = ring.time.length;
  const index = ring.total <= capacity ? 0 : ring.writeIndex;
  return ring.time[index];
}

export function computeRange(columns: number[][]): { min: number; max: number } | null {
  let min = Infinity;
  let max = -Infinity;
  for (const column of columns) {
    for (const v of column) {
      if (!Number.isFinite(v)) continue;
      if (v < min) min = v;
      if (v > max) max = v;
    }
  }
  if (min === Infinity || max === -Infinity) return null;
  if (min === max) {
    const pad = Math.abs(min) * 0.05 || 1;
    return { min: min - pad, max: max + pad };
  }
  const pad = (max - min) * 0.05;
  return { min: min - pad, max: max + pad };
}
