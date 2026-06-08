import type { DropZone } from "$lib/dashboard/layout/layout";

const EDGE_FRACTION = 0.25;

export function pickZone(rect: DOMRect, x: number, y: number): DropZone {
  if (rect.width <= 0 || rect.height <= 0) return "center";
  const margins: Record<Exclude<DropZone, "center">, number> = {
    left: (x - rect.left) / rect.width,
    right: (rect.right - x) / rect.width,
    top: (y - rect.top) / rect.height,
    bottom: (rect.bottom - y) / rect.height,
  };
  let zone: Exclude<DropZone, "center"> = "left";
  for (const key of Object.keys(margins) as (keyof typeof margins)[]) {
    if (margins[key] < margins[zone]) zone = key;
  }
  return margins[zone] <= EDGE_FRACTION ? zone : "center";
}
