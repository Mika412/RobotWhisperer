import type { DecodedFrame } from "$lib/workers/decoderManager";
import type { Value } from "./types";

export interface SubscribeResponse {
  subscription_id: string;
  schema_id: string;
  schema_name: string;
  viz_role: string;
}

export interface SubscribeOptions {
  targetHz?: number;
  queueLength?: number;
  fields?: string[];
}

export type FrameCallback = (frame: DecodedFrame) => void;

export type ActionEnvelope =
  | { kind: "feedback"; value: Value }
  | { kind: "result"; value: Value }
  | { kind: "error"; message: string }
  | { kind: "closed" };

export interface PipelineRpc {
  openFoxglove(url: string): Promise<string>;
  openRosbridge(url: string): Promise<string>;
  openDummy(): Promise<string>;
  close(connectionId: string): Promise<void>;
  subscribe(
    streamKey: string,
    connectionId: string,
    topic: string,
    onFrame: FrameCallback,
    options?: SubscribeOptions,
  ): Promise<SubscribeResponse>;
  unsubscribe(streamKey: string, subscriptionId: string): Promise<void>;
  callService(connectionId: string, service: string, request: Value): Promise<Value>;
  sendActionGoal(
    connectionId: string,
    action: string,
    goal: Value,
    onEnvelope: (envelope: ActionEnvelope) => void,
  ): Promise<string>;
  cancelActionGoal(goalId: string): Promise<void>;
  getDiscovery(sessionId: string): Promise<DiscoverySnapshot | null>;
  onDiscovery(sessionId: string, cb: (snapshot: DiscoverySnapshot) => void): Promise<void>;
  onStatus(sessionId: string, cb: (status: ConnectionStatus) => void): Promise<void>;
}

export type ConnectionStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "reconnecting"
  | { failed: string };

export interface DiscoverySnapshot {
  topics: { name: string; schema: { name: string; hash: string } }[];
  services: { name: string; schema: { name: string; hash: string } }[];
  actions: { name: string; schema: { name: string; hash: string } }[];
}
