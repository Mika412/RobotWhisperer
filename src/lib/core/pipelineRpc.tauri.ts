import { decoderWorker } from "$lib/workers/decoderManager";
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

function optionsToBackend(options: SubscribeOptions | undefined): unknown {
  if (!options) return null;
  const payload: Record<string, unknown> = {};
  if (typeof options.targetHz === "number") payload.target_hz = options.targetHz;
  if (typeof options.queueLength === "number") payload.queue_length = options.queueLength;
  if (Array.isArray(options.fields) && options.fields.length > 0) payload.fields = options.fields;
  return Object.keys(payload).length === 0 ? null : payload;
}

class TauriPipelineRpc implements PipelineRpc {
  private ingestReady: Promise<void> | null = null;

  private ensureIngestConnected(): Promise<void> {
    if (!this.ingestReady) {
      this.ingestReady = (async () => {
        const { invoke } = await import("@tauri-apps/api/core");
        const port = await invoke<number>("ingest_ws_port");
        decoderWorker.connectIngest(`ws://127.0.0.1:${port}`);
      })().catch((err) => {
        this.ingestReady = null;
        throw err;
      });
    }
    return this.ingestReady;
  }

  async openFoxglove(url: string): Promise<string> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("pipeline_open_foxglove", { url });
  }

  async openRosbridge(url: string): Promise<string> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("pipeline_open_rosbridge", { url });
  }

  async openDummy(): Promise<string> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("pipeline_open_dummy");
  }

  async close(connectionId: string): Promise<void> {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("pipeline_close", { connectionId });
  }

  async subscribe(
    streamKey: string,
    connectionId: string,
    topic: string,
    onFrame: FrameCallback,
    options?: SubscribeOptions,
  ): Promise<SubscribeResponse> {
    const { invoke } = await import("@tauri-apps/api/core");
    await this.ensureIngestConnected();
    decoderWorker.registerStream(streamKey, onFrame);
    const resp = await invoke<SubscribeResponse>("pipeline_subscribe_topic", {
      connectionId,
      topic,
      options: optionsToBackend(options),
    });
    decoderWorker.mapStream(streamKey, {
      handle: resp.subscription_id,
      schemaId: resp.schema_id,
      schemaName: resp.schema_name,
      vizRole: resp.viz_role,
    });
    return resp;
  }

  async unsubscribe(streamKey: string, subscriptionId: string): Promise<void> {
    decoderWorker.unregisterStream(streamKey);
    decoderWorker.unmapStream(streamKey, subscriptionId);
    const { invoke } = await import("@tauri-apps/api/core");
    try {
      await invoke("pipeline_unsubscribe", { subscriptionId });
    } catch {}
  }

  async callService(connectionId: string, service: string, request: Value): Promise<Value> {
    const { invoke } = await import("@tauri-apps/api/core");
    const responseJson = await invoke<string>("pipeline_call_service", {
      connectionId,
      service,
      requestJson: JSON.stringify(request),
    });
    return JSON.parse(responseJson) as Value;
  }

  async sendActionGoal(
    connectionId: string,
    action: string,
    goal: Value,
    onEnvelope: (envelope: ActionEnvelope) => void,
  ): Promise<string> {
    const { invoke, Channel } = await import("@tauri-apps/api/core");
    const channel = new Channel<ArrayBuffer | Uint8Array | number[]>();
    channel.onmessage = (raw) => {
      try {
        const bytes =
          raw instanceof ArrayBuffer
            ? new Uint8Array(raw)
            : raw instanceof Uint8Array
              ? raw
              : Uint8Array.from(raw);
        onEnvelope(JSON.parse(new TextDecoder().decode(bytes)) as ActionEnvelope);
      } catch (err) {
        console.warn("[pipelineRpc.tauri] action envelope parse failed", err);
      }
    };
    return invoke<string>("pipeline_send_action_goal", {
      connectionId,
      action,
      goalJson: JSON.stringify(goal),
      channel,
    });
  }

  async cancelActionGoal(goalId: string): Promise<void> {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("pipeline_cancel_action_goal", { goalId });
  }

  async getDiscovery(sessionId: string): Promise<DiscoverySnapshot | null> {
    const { invoke } = await import("@tauri-apps/api/core");
    const json = await invoke<string>("pipeline_discovery", { connectionId: sessionId });
    if (!json || json === "null") return null;
    return JSON.parse(json) as DiscoverySnapshot;
  }

  async onDiscovery(
    _sessionId: string,
    _cb: (snapshot: DiscoverySnapshot) => void,
  ): Promise<void> {}

  async onStatus(_sessionId: string, _cb: (status: ConnectionStatus) => void): Promise<void> {}
}

export function create(): PipelineRpc {
  return new TauriPipelineRpc();
}

export function getWasmInstance(): Promise<never> {
  return Promise.reject(
    new Error("getWasmInstance() invoked on the native Tauri shell; this is a bug."),
  );
}
