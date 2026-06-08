import { pipelineRpc, type ActionEnvelope, type SubscribeOptions } from "$lib/core/pipelineRpc";
import type { DecodedFrame } from "$lib/workers/decoderManager";
import type { SessionId, Value } from "$lib/core/types";

export type { ActionEnvelope, SubscribeOptions };

export const HUB_DEFAULT_TARGET_HZ = 60;

type FrameCallback = (frame: DecodedFrame) => void;

interface SharedStream {
  key: string;
  connectionId: SessionId;
  topic: string;
  subscriptionId: string;
  schemaId: string;
  schemaName: string;
  vizRole: string;
  listeners: Set<FrameCallback>;
  latest: { value: DecodedFrame | null };
  refCount: number;
  readyPromise: Promise<void>;
}

export interface PipelineStream {
  readonly key: string;
  readonly connectionId: SessionId;
  readonly topic: string;
  readonly latest: { readonly value: DecodedFrame | null };
  onFrame(cb: FrameCallback): () => void;
  readonly schemaId: string;
  readonly schemaName: string;
  readonly vizRole: string;
  dispose(): Promise<void>;
}

class PipelineHub {
  private streams = new Map<string, SharedStream>();

  async openFoxglove(url: string): Promise<string> {
    return pipelineRpc.openFoxglove(url);
  }

  async openRosbridge(url: string): Promise<string> {
    return pipelineRpc.openRosbridge(url);
  }

  async openDummy(): Promise<string> {
    return pipelineRpc.openDummy();
  }

  async close(connectionId: string): Promise<void> {
    await pipelineRpc.close(connectionId);
  }

  async callService(connectionId: SessionId, service: string, request: Value): Promise<Value> {
    return pipelineRpc.callService(connectionId, service, request);
  }

  async sendActionGoal(
    connectionId: SessionId,
    action: string,
    goal: Value,
    onEnvelope: (envelope: ActionEnvelope) => void,
  ): Promise<string> {
    return pipelineRpc.sendActionGoal(connectionId, action, goal, onEnvelope);
  }

  async cancelActionGoal(goalId: string): Promise<void> {
    return pipelineRpc.cancelActionGoal(goalId);
  }

  async subscribe(
    connectionId: SessionId,
    topic: string,
    options?: SubscribeOptions,
  ): Promise<PipelineStream> {
    const key = `${connectionId}:${topic}`;
    let shared = this.streams.get(key);
    if (!shared) {
      const effective: SubscribeOptions = { ...(options ?? {}) };
      if (effective.targetHz === undefined) {
        effective.targetHz = HUB_DEFAULT_TARGET_HZ;
      }
      shared = this.createShared(connectionId, topic, key, effective);
      this.streams.set(key, shared);
    }
    shared.refCount += 1;
    await shared.readyPromise;
    return this.makeHandle(shared);
  }

  private createShared(
    connectionId: SessionId,
    topic: string,
    key: string,
    options?: SubscribeOptions,
  ): SharedStream {
    const shared: SharedStream = {
      key,
      connectionId,
      topic,
      subscriptionId: "",
      schemaId: "",
      schemaName: "",
      vizRole: "",
      listeners: new Set(),
      latest: { value: null },
      refCount: 0,
      readyPromise: Promise.resolve(),
    };

    const onFrame = (frame: DecodedFrame) => {
      if (frame.perf) {
        frame.perf.hubDispatchNs = Math.round((performance.timeOrigin + performance.now()) * 1e6);
      }
      shared.latest.value = frame;
      for (const cb of shared.listeners) cb(frame);
    };

    shared.readyPromise = pipelineRpc
      .subscribe(key, connectionId, topic, onFrame, options)
      .then((resp) => {
        shared.subscriptionId = resp.subscription_id;
        shared.schemaId = resp.schema_id;
        shared.schemaName = resp.schema_name;
        shared.vizRole = resp.viz_role;
      });

    return shared;
  }

  private makeHandle(shared: SharedStream): PipelineStream {
    let disposed = false;
    return {
      get key(): string {
        return shared.key;
      },
      get connectionId(): SessionId {
        return shared.connectionId;
      },
      get topic(): string {
        return shared.topic;
      },
      get latest(): { readonly value: DecodedFrame | null } {
        return shared.latest;
      },
      get schemaId(): string {
        return shared.schemaId;
      },
      get schemaName(): string {
        return shared.schemaName;
      },
      get vizRole(): string {
        return shared.vizRole;
      },
      onFrame: (cb: FrameCallback) => {
        shared.listeners.add(cb);
        if (shared.latest.value) {
          cb(shared.latest.value);
        }
        return () => {
          shared.listeners.delete(cb);
        };
      },
      dispose: async () => {
        if (disposed) return;
        disposed = true;
        shared.refCount -= 1;
        if (shared.refCount === 0) {
          this.streams.delete(shared.key);
          if (shared.subscriptionId) {
            await pipelineRpc.unsubscribe(shared.key, shared.subscriptionId);
          }
        }
      },
    };
  }
}

export const pipelineHub = new PipelineHub();
