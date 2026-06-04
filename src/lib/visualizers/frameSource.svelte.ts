import { pipelineHub, type PipelineStream } from "$lib/core/pipelineHub";
import { connectionStore } from "$lib/stores/connectionStore.svelte";
import type { DecodedFrame } from "$lib/workers/decoderManager";
import type { FrameSource, VizStatus } from "./types";

const RETRY_MS = 1500;
const DISPLAY_THROTTLE_MS = 120;

export function useTopicSource(
  getConnectionId: () => number | null | undefined,
  getTopic: () => string,
): FrameSource {
  let status = $state<VizStatus>("idle");
  let schemaName = $state("");
  let error = $state<string | null>(null);
  let value = $state<unknown>(null);

  let latest: DecodedFrame | null = null;
  const listeners: ((frame: DecodedFrame) => void)[] = [];

  function deliver(frame: DecodedFrame) {
    latest = frame;
    for (const listener of listeners) listener(frame);
  }

  $effect(() => {
    const explicitId = getConnectionId() ?? null;
    const topic = getTopic().trim();
    const connection =
      explicitId != null
        ? connectionStore.connections.find((entry) => entry.id === explicitId)
        : connectionStore.connections.find(
            (entry) => connectionStore.status(entry.id) === "connected",
          );

    if (!topic) {
      status = "idle";
      error = null;
      return;
    }
    const sessionId = connection ? connectionStore.sessionId(connection.id) : null;
    if (!connection || !sessionId) {
      status = "waiting";
      return;
    }

    let cancelled = false;
    let stream: PipelineStream | null = null;
    let off: (() => void) | null = null;
    let retry: ReturnType<typeof setTimeout> | null = null;
    status = "waiting";
    error = null;
    const display = setInterval(() => {
      value = latest ? ((latest as { json?: unknown }).json ?? null) : null;
    }, DISPLAY_THROTTLE_MS);

    async function attempt() {
      try {
        const handle = await pipelineHub.subscribe(sessionId!, topic);
        if (cancelled) {
          await handle.dispose();
          return;
        }
        stream = handle;
        schemaName = handle.schemaName;
        error = null;
        status = "active";
        off = handle.onFrame(deliver);
      } catch (err) {
        if (cancelled) return;
        error = err instanceof Error ? err.message : String(err);
        status = "waiting";
        retry = setTimeout(attempt, RETRY_MS);
      }
    }
    void attempt();

    return () => {
      cancelled = true;
      clearInterval(display);
      if (retry) clearTimeout(retry);
      off?.();
      if (stream) void stream.dispose();
      latest = null;
    };
  });

  return {
    onFrame(callback) {
      listeners.push(callback);
      if (latest) callback(latest);
      return () => {
        const index = listeners.indexOf(callback);
        if (index >= 0) listeners.splice(index, 1);
      };
    },
    get latest() {
      return latest;
    },
    get value() {
      return value;
    },
    get status() {
      return status;
    },
    get schemaName() {
      return schemaName;
    },
    get error() {
      return error;
    },
  };
}

export function staticFrameSource(frame: DecodedFrame | null): FrameSource {
  return {
    onFrame(callback) {
      if (frame) callback(frame);
      return () => {};
    },
    get latest() {
      return frame;
    },
    get value() {
      return frame ? ((frame as { json?: unknown }).json ?? null) : null;
    },
    get status() {
      return frame ? "active" : "idle";
    },
    get schemaName() {
      return frame?.schemaName ?? "";
    },
    get error() {
      return null;
    },
  };
}
