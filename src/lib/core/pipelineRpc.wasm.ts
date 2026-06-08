import {
  FrameFlags,
  FrameKind,
  PayloadKind,
  type DecodedFramePayload,
  type PerfTrace,
} from "$lib/workers/decoder.worker";
import type { Value } from "./types";
import type {
  ActionEnvelope,
  ConnectionStatus,
  DiscoverySnapshot,
  FrameCallback,
  PipelineRpc,
  SubscribeOptions,
  SubscribeResponse,
} from "./pipelineRpc.shared";

interface WasmFrame {
  subscriptionId: string;
  schemaId: string;
  schemaName: string;
  vizRole: string;
  timestampNs: number | bigint;
  isReplay: boolean;
  value?: unknown;
  rgba?: Uint8ClampedArray;
  width?: number;
  height?: number;
  perf?: PerfTrace;
}

function perfNowNs(): number {
  return Math.round((performance.timeOrigin + performance.now()) * 1e6);
}

function wasmFrameToDecoded(frame: WasmFrame): DecodedFramePayload {
  const timestampNs =
    typeof frame.timestampNs === "bigint" ? frame.timestampNs : BigInt(frame.timestampNs);
  const isImage = frame.rgba != null && frame.width != null && frame.height != null;
  let perf: PerfTrace | undefined;
  if (frame.perf) {
    const recv = perfNowNs();
    perf = { ...frame.perf, workerRecvNs: recv, workerDecodedNs: recv };
  }
  return {
    wireVersion: 3,
    timestampNs,
    schemaId: frame.schemaId,
    schemaHash: frame.schemaId,
    schemaName: frame.schemaName,
    vizRole: frame.vizRole,
    flags: frame.isReplay ? FrameFlags.StaleReplay : 0,
    kind: isImage ? PayloadKind.ImageRgba : PayloadKind.DecodedJson,
    frameKind: isImage ? FrameKind.Image : FrameKind.Value,
    json: isImage ? null : (frame.value ?? null),
    rgba: isImage ? (frame.rgba as Uint8ClampedArray) : null,
    width: isImage ? (frame.width as number) : 0,
    height: isImage ? (frame.height as number) : 0,
    positions: null,
    raw: null,
    perf,
  };
}

interface WasmModule {
  default: (input?: unknown) => Promise<unknown>;
  WasmRobotWhisperer: {
    create(): Promise<WasmInstance>;
  };
}

export interface WasmInstance {
  pipelineOpenFoxglove(url: string): Promise<string>;
  pipelineOpenRosbridge(url: string): Promise<string>;
  pipelineOpenDummy(): Promise<string>;
  pipelineGetDiscovery(connectionId: string): Promise<string>;
  pipelineOnDiscovery(connectionId: string, onDiscovery: (json: string) => void): Promise<void>;
  pipelineOnStatus(connectionId: string, onStatus: (json: string) => void): Promise<void>;
  pipelineClose(connectionId: string): Promise<void>;
  pipelineSubscribeTopic(
    connectionId: string,
    topic: string,
    onFrame: (frame: WasmFrame) => void,
    optionsJson?: string,
  ): Promise<SubscribeResponse>;
  pipelineUnsubscribe(subscriptionId: string): Promise<void>;
  pipelineCallService(connectionId: string, service: string, requestJson: string): Promise<string>;
  pipelineSendActionGoal(
    connectionId: string,
    action: string,
    goalJson: string,
    onEnvelope: (json: string) => void,
  ): Promise<string>;
  pipelineCancelActionGoal(goalId: string): Promise<void>;
  listRequests(): Promise<string>;
  getRequest(id: number): Promise<string | null>;
  createRequest(draftJson: string): Promise<string>;
  updateRequest(requestJson: string): Promise<void>;
  deleteRequest(id: number): Promise<void>;
  listCollections(): Promise<string>;
  createCollection(draftJson: string): Promise<string>;
  updateCollection(collectionJson: string): Promise<void>;
  deleteCollection(id: number): Promise<void>;
  listConnections(): Promise<string>;
  getConnection(id: number): Promise<string | null>;
  createConnection(draftJson: string): Promise<string>;
  updateConnection(connectionJson: string): Promise<void>;
  deleteConnection(id: number): Promise<void>;
  clearWorkspaceStorage(): Promise<void>;
  exportWorkspace(): Promise<string>;
  importWorkspace(fileJson: string, mode: string): Promise<string>;
  listSchemasSummary(): string;
  listSchemasByName(name: string): string;
  getSchemaByHash(hash: string): string | null;
  registerSchema(name: string, kind: string, definition: string): Promise<string>;
  setPerfTraceEnabled(enabled: boolean): void;
}

let wasmInstance: Promise<WasmInstance> | null = null;
export function getWasmInstance(): Promise<WasmInstance> {
  if (!wasmInstance) {
    wasmInstance = (async () => {
      const mod = (await import("$lib/wasm/generated/rw_wasm")) as unknown as WasmModule;
      await mod.default();
      return mod.WasmRobotWhisperer.create();
    })();
  }
  return wasmInstance;
}

function optionsToJson(options: SubscribeOptions | undefined): string | undefined {
  if (!options) return undefined;
  const payload: Record<string, number> = {};
  if (typeof options.targetHz === "number") payload.target_hz = options.targetHz;
  if (typeof options.queueLength === "number") payload.queue_length = options.queueLength;
  return Object.keys(payload).length === 0 ? undefined : JSON.stringify(payload);
}

class WasmPipelineRpc implements PipelineRpc {
  private async getInstance(): Promise<WasmInstance> {
    return getWasmInstance();
  }

  async openFoxglove(url: string): Promise<string> {
    const w = await this.getInstance();
    return w.pipelineOpenFoxglove(url);
  }

  async openRosbridge(url: string): Promise<string> {
    const w = await this.getInstance();
    return w.pipelineOpenRosbridge(url);
  }

  async openDummy(): Promise<string> {
    const w = await this.getInstance();
    return w.pipelineOpenDummy();
  }

  async close(connectionId: string): Promise<void> {
    const w = await this.getInstance();
    await w.pipelineClose(connectionId);
  }

  async subscribe(
    _streamKey: string,
    connectionId: string,
    topic: string,
    onFrame: FrameCallback,
    options?: SubscribeOptions,
  ): Promise<SubscribeResponse> {
    const w = await this.getInstance();
    return w.pipelineSubscribeTopic(
      connectionId,
      topic,
      (frame) => {
        onFrame(wasmFrameToDecoded(frame));
      },
      optionsToJson(options),
    );
  }

  async unsubscribe(_streamKey: string, subscriptionId: string): Promise<void> {
    const w = await this.getInstance();
    try {
      await w.pipelineUnsubscribe(subscriptionId);
    } catch {}
  }

  async callService(connectionId: string, service: string, request: Value): Promise<Value> {
    const w = await this.getInstance();
    const responseJson = await w.pipelineCallService(
      connectionId,
      service,
      JSON.stringify(request),
    );
    return JSON.parse(responseJson) as Value;
  }

  async sendActionGoal(
    connectionId: string,
    action: string,
    goal: Value,
    onEnvelope: (envelope: ActionEnvelope) => void,
  ): Promise<string> {
    const w = await this.getInstance();
    return w.pipelineSendActionGoal(connectionId, action, JSON.stringify(goal), (json) => {
      try {
        onEnvelope(JSON.parse(json) as ActionEnvelope);
      } catch (err) {
        console.warn("[pipelineRpc.wasm] action envelope parse failed", err);
      }
    });
  }

  async cancelActionGoal(goalId: string): Promise<void> {
    const w = await this.getInstance();
    await w.pipelineCancelActionGoal(goalId);
  }

  async getDiscovery(sessionId: string): Promise<DiscoverySnapshot | null> {
    const w = await this.getInstance();
    try {
      const json = await w.pipelineGetDiscovery(sessionId);
      if (!json || json === "null") return null;
      return JSON.parse(json) as DiscoverySnapshot;
    } catch (err) {
      console.warn("[pipelineRpc.wasm] getDiscovery failed", err);
      return null;
    }
  }

  async onDiscovery(sessionId: string, cb: (snapshot: DiscoverySnapshot) => void): Promise<void> {
    const w = await this.getInstance();
    await w.pipelineOnDiscovery(sessionId, (json) => {
      try {
        cb(JSON.parse(json) as DiscoverySnapshot);
      } catch (err) {
        console.warn("[pipelineRpc.wasm] discovery parse failed", err);
      }
    });
  }

  async onStatus(sessionId: string, cb: (status: ConnectionStatus) => void): Promise<void> {
    const w = await this.getInstance();
    await w.pipelineOnStatus(sessionId, (json) => {
      try {
        cb(JSON.parse(json) as ConnectionStatus);
      } catch (err) {
        console.warn("[pipelineRpc.wasm] status parse failed", err);
      }
    });
  }
}

export function create(): PipelineRpc {
  return new WasmPipelineRpc();
}
