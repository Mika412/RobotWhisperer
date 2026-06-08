import type { Value } from "$lib/core/types";

export interface CloudData {
  positions: Float32Array;
  color: [number, number, number] | null;
}

function asNumber(value: Value | undefined): number | undefined {
  if (!value) return undefined;
  switch (value.kind) {
    case "int":
    case "uint":
    case "f32":
    case "f64":
      return value.value;
    default:
      return undefined;
  }
}

function readColor(value: Value | undefined): [number, number, number] | null {
  if (!value || value.kind !== "struct") return null;
  const r = asNumber(value.value.r);
  const g = asNumber(value.value.g);
  const b = asNumber(value.value.b);
  if (r === undefined || g === undefined || b === undefined) return null;
  return [r, g, b];
}

export function extractCloud(value: unknown): CloudData | null {
  const root = value as Value | undefined;
  if (!root || root.kind !== "struct") return null;
  const markers = root.value.markers;
  if (!markers || markers.kind !== "array") return null;

  const coordinates: number[] = [];
  let color: [number, number, number] | null = null;
  for (const marker of markers.value) {
    if (marker.kind !== "struct") continue;
    if (!color) color = readColor(marker.value.color);
    const points = marker.value.points;
    if (!points || points.kind !== "array") continue;
    for (const point of points.value) {
      if (point.kind !== "struct") continue;
      const x = asNumber(point.value.x);
      const y = asNumber(point.value.y);
      const z = asNumber(point.value.z);
      if (x === undefined || y === undefined || z === undefined) continue;
      coordinates.push(x, y, z);
    }
  }
  return { positions: new Float32Array(coordinates), color };
}
