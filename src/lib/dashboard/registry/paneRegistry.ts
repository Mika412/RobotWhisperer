import type { Component } from "svelte";
import type { PipelineStream } from "$lib/core/pipelineHub";
import type { ActionEnvelope } from "$lib/core/pipelineRpc.shared";
import type { TransportStatus, Value } from "$lib/core/types";
import type { VisualizerSettingsProps } from "$lib/visualizers/types";

export type PaneCategory = "visualization" | "data" | "control" | "misc";

export interface ConnectionSummary {
  id: number;
  name: string;
  status: TransportStatus;
}

export interface PaneContext {
  readonly nodeId: string;
  readonly title: string | null;
  readonly connections: ConnectionSummary[];
  persist(patch: Record<string, unknown>): void;
  setTitle(title: string | null): void;
  subscribe(connectionId: number, topic: string): Promise<PipelineStream>;
  callService(connectionId: number, service: string, request: Value): Promise<Value>;
  sendActionGoal(
    connectionId: number,
    action: string,
    goal: Value,
    onEnvelope: (envelope: ActionEnvelope) => void,
  ): Promise<string>;
  cancelActionGoal(goalId: string): Promise<void>;
}

export interface PaneComponentProps<Config = Record<string, unknown>> {
  config: Config;
  ctx: PaneContext;
}

export type PaneSettingsProps<Config = Record<string, unknown>> = VisualizerSettingsProps<Config>;

export interface PaneDescriptor<Config = Record<string, unknown>> {
  type: string;
  displayName: string;
  description?: string;
  category: PaneCategory;
  group?: string;
  defaultConfig: Config;
  component: Component<PaneComponentProps<Config>>;
  settingsComponent?: Component<PaneSettingsProps<Config>>;
}

export const DEFAULT_PANE_GROUP = "Panes";

const registry = new Map<string, PaneDescriptor>();

export function registerPane<Config>(descriptor: PaneDescriptor<Config>): void {
  registry.set(descriptor.type, descriptor as unknown as PaneDescriptor);
}

export function getPane(type: string): PaneDescriptor | undefined {
  return registry.get(type);
}

export function listPanes(): PaneDescriptor[] {
  return [...registry.values()];
}

export function panesByGroup(): { group: string; panes: PaneDescriptor[] }[] {
  const groups: { group: string; panes: PaneDescriptor[] }[] = [];
  for (const descriptor of registry.values()) {
    if (descriptor.type === "rw.placeholder") continue;
    const name = descriptor.group ?? DEFAULT_PANE_GROUP;
    let bucket = groups.find((entry) => entry.group === name);
    if (!bucket) {
      bucket = { group: name, panes: [] };
      groups.push(bucket);
    }
    bucket.panes.push(descriptor);
  }
  return groups;
}

export function panesByCategory(): Record<PaneCategory, PaneDescriptor[]> {
  const buckets: Record<PaneCategory, PaneDescriptor[]> = {
    visualization: [],
    data: [],
    control: [],
    misc: [],
  };
  for (const descriptor of registry.values()) buckets[descriptor.category].push(descriptor);
  return buckets;
}
