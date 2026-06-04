import type { Component, Snippet } from "svelte";
import type { DecodedFrame } from "$lib/workers/decoderManager";

export type VizStatus = "idle" | "waiting" | "active" | "error";

export interface FrameSource {
  onFrame(callback: (frame: DecodedFrame) => void): () => void;
  readonly latest: DecodedFrame | null;
  readonly value: unknown;
  readonly status: VizStatus;
  readonly schemaName: string;
  readonly error: string | null;
}

export interface VisualizerProps<Config = Record<string, unknown>> {
  source: FrameSource;
  config: Config;
  overlay?: Snippet;
}

export interface VisualizerSettingsProps<Config = Record<string, unknown>> {
  config: Config;
  source: FrameSource;
  onchange: (patch: Partial<Config>) => void;
}

export interface VisualizerDescriptor<Config = Record<string, unknown>> {
  id: string;
  displayName: string;
  defaultConfig: Config;
  component: Component<VisualizerProps<Config>>;
  settingsComponent?: Component<VisualizerSettingsProps<Config>>;
  accepts(schemaName: string, frameKind: number): number;
}
