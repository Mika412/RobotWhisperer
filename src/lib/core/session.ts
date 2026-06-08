import type {
  ConnectionId,
  Discovery,
  SessionDiscoveryEvent,
  SessionInfo,
  SessionStatusEvent,
  SubscriptionLifecycleEvent,
  Value,
} from "./types";
import { isTauri } from "./platform";

type UnlistenFn = () => void;

async function ensureTauri(call: string): Promise<void> {
  if (!isTauri()) {
    throw new Error(
      `${call} is only available on the Tauri desktop shell; ` +
        `use pipelineHub on the web shell instead`,
    );
  }
}

async function tauriInvoke<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(name, args);
}

async function tauriListen<T>(name: string, cb: (event: T) => void): Promise<UnlistenFn> {
  const { listen } = await import("@tauri-apps/api/event");
  return listen<T>(name, (event) => cb(event.payload));
}

export async function activateConnection(connectionId: ConnectionId): Promise<string> {
  await ensureTauri("activateConnection");
  return tauriInvoke<string>("activate_connection", { connectionId });
}

export async function deactivateConnection(connectionId: ConnectionId): Promise<void> {
  await ensureTauri("deactivateConnection");
  return tauriInvoke<void>("deactivate_connection", { connectionId });
}

export async function listSessionsStatus(): Promise<SessionInfo[]> {
  if (!isTauri()) return [];
  return tauriInvoke<SessionInfo[]>("list_sessions_status");
}

export async function getDiscovery(connectionId: ConnectionId): Promise<Discovery | null> {
  if (!isTauri()) return null;
  return tauriInvoke<Discovery | null>("get_discovery", { connectionId });
}

export async function callServiceCanonical(
  connectionId: ConnectionId,
  service: string,
  request: Value,
): Promise<Value> {
  await ensureTauri("callServiceCanonical");
  const responseJson = await tauriInvoke<string>("call_service_canonical", {
    connectionId,
    service,
    requestJson: JSON.stringify(request),
  });
  return JSON.parse(responseJson) as Value;
}

export type ActionEnvelope =
  | { kind: "feedback"; value: Value }
  | { kind: "result"; value: Value }
  | { kind: "error"; message: string }
  | { kind: "closed" };

export async function sendActionGoalCanonical(
  connectionId: ConnectionId,
  action: string,
  goal: Value,
  onEnvelope: (envelope: ActionEnvelope) => void,
): Promise<string> {
  await ensureTauri("sendActionGoalCanonical");
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
      const text = new TextDecoder().decode(bytes);
      onEnvelope(JSON.parse(text) as ActionEnvelope);
    } catch (err) {
      console.warn("action envelope parse failed", err);
    }
  };
  return invoke<string>("send_action_goal_canonical", {
    connectionId,
    action,
    goalJson: JSON.stringify(goal),
    channel,
  });
}

export async function cancelActionGoalCanonical(goalId: string): Promise<void> {
  await ensureTauri("cancelActionGoalCanonical");
  return tauriInvoke<void>("cancel_action_goal_canonical", { goalId });
}

export async function onSessionStatus(
  callback: (event: SessionStatusEvent) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return tauriListen<SessionStatusEvent>("session://status", callback);
}

export async function onDiscoveryUpdate(
  callback: (event: SessionDiscoveryEvent) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return tauriListen<SessionDiscoveryEvent>("session://discovery", callback);
}

export async function onSubscriptionStale(
  callback: (event: SubscriptionLifecycleEvent) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return tauriListen<SubscriptionLifecycleEvent>("subscription://stale", callback);
}

export async function onSubscriptionFresh(
  callback: (event: SubscriptionLifecycleEvent) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return tauriListen<SubscriptionLifecycleEvent>("subscription://fresh", callback);
}
