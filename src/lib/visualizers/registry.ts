import type { VisualizerDescriptor } from "./types";

const registry = new Map<string, VisualizerDescriptor>();

export function registerVisualizer<Config>(descriptor: VisualizerDescriptor<Config>): void {
  registry.set(descriptor.id, descriptor as unknown as VisualizerDescriptor);
}

export function getVisualizer(id: string): VisualizerDescriptor | undefined {
  return registry.get(id);
}

export function listVisualizers(): VisualizerDescriptor[] {
  return [...registry.values()];
}

export function matchingVisualizers(schemaName: string, frameKind: number): VisualizerDescriptor[] {
  return [...registry.values()]
    .map((descriptor) => ({ descriptor, score: descriptor.accepts(schemaName, frameKind) }))
    .filter((entry) => entry.score > 0)
    .sort((a, b) => b.score - a.score)
    .map((entry) => entry.descriptor);
}
