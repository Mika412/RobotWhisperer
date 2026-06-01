import type {
  ActionEnvelope,
  ConnectionStatus,
  DiscoverySnapshot,
  FrameCallback,
  PipelineRpc,
  SubscribeOptions,
  SubscribeResponse,
} from "./pipelineRpc.shared";

export type {
  ActionEnvelope,
  ConnectionStatus,
  DiscoverySnapshot,
  FrameCallback,
  PipelineRpc,
  SubscribeOptions,
  SubscribeResponse,
};

let implPromise: Promise<PipelineRpc> | null = null;
function getImpl(): Promise<PipelineRpc> {
  if (!implPromise) {
    implPromise = (async () => {
      if (import.meta.env.RW_WEB) {
        const mod = await import("./pipelineRpc.wasm");
        return mod.create();
      }
      const mod = await import("./pipelineRpc.tauri");
      return mod.create();
    })();
  }
  return implPromise;
}

export const pipelineRpc: PipelineRpc = {
  openFoxglove: (url) => getImpl().then((i) => i.openFoxglove(url)),
  openRosbridge: (url) => getImpl().then((i) => i.openRosbridge(url)),
  openDummy: () => getImpl().then((i) => i.openDummy()),
  close: (id) => getImpl().then((i) => i.close(id)),
  subscribe: (streamKey, connectionId, topic, onFrame, options) =>
    getImpl().then((i) => i.subscribe(streamKey, connectionId, topic, onFrame, options)),
  unsubscribe: (streamKey, subId) => getImpl().then((i) => i.unsubscribe(streamKey, subId)),
  callService: (id, service, request) => getImpl().then((i) => i.callService(id, service, request)),
  sendActionGoal: (id, action, goal, onEnvelope) =>
    getImpl().then((i) => i.sendActionGoal(id, action, goal, onEnvelope)),
  cancelActionGoal: (goalId) => getImpl().then((i) => i.cancelActionGoal(goalId)),
  getDiscovery: (sessionId) => getImpl().then((i) => i.getDiscovery(sessionId)),
  onDiscovery: (sessionId, cb) => getImpl().then((i) => i.onDiscovery(sessionId, cb)),
  onStatus: (sessionId, cb) => getImpl().then((i) => i.onStatus(sessionId, cb)),
};

export type WasmInstance = import("./pipelineRpc.wasm").WasmInstance;
export function getWasmInstance(): Promise<WasmInstance> {
  if (!import.meta.env.RW_WEB) {
    return Promise.reject(
      new Error("getWasmInstance() is web-only; this build is the native shell"),
    );
  }
  return import("./pipelineRpc.wasm").then((m) => m.getWasmInstance());
}
