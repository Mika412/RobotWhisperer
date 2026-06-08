import {
  decodeFrameBuffer,
  peekHandle,
  perfNowNs,
  setDiagnosticSink,
  PayloadKind,
  FrameKind,
  FrameFlags,
  PERF_TRACE_SIZE,
} from "./decoderCore";
import type { DecodedFramePayload, PerfTrace } from "./decoderCore";

export { PayloadKind, FrameKind, FrameFlags, PERF_TRACE_SIZE };
export type { DecodedFramePayload, PerfTrace };

setDiagnosticSink((message) => {
  self.postMessage({ type: "diagnostic", message });
});

interface StreamBinding {
  streamKey: string;
  schemaId: string;
  schemaName: string;
  vizRole: string;
}

let ingestWs: WebSocket | null = null;
let ingestUrl: string | null = null;
const handleToStream = new Map<string, StreamBinding>();
const latestByHandle = new Map<string, ArrayBuffer>();
let drainTimer: ReturnType<typeof setTimeout> | null = null;

const DRAIN_INTERVAL_MS = 12;

function drainIngest(): void {
  drainTimer = null;
  if (latestByHandle.size === 0) return;
  const entries = [...latestByHandle];
  latestByHandle.clear();
  const recvNs = perfNowNs();
  for (const [handle, buffer] of entries) {
    const binding = handleToStream.get(handle);
    if (!binding) continue;
    const decoded = decodeFrameBuffer(buffer, recvNs, binding);
    if (!decoded) continue;
    self.postMessage(
      { type: "decoded", streamKey: binding.streamKey, frame: decoded.frame },
      { transfer: decoded.transfer },
    );
  }
}

function scheduleDrain(): void {
  if (drainTimer !== null) return;
  drainTimer = setTimeout(drainIngest, DRAIN_INTERVAL_MS);
}

function connectIngest(url: string): void {
  if (
    ingestWs &&
    ingestUrl === url &&
    (ingestWs.readyState === WebSocket.CONNECTING || ingestWs.readyState === WebSocket.OPEN)
  ) {
    return;
  }
  if (ingestWs) {
    try {
      ingestWs.close();
    } catch {}
    ingestWs = null;
  }
  ingestUrl = url;
  try {
    const ws = new WebSocket(url);
    ws.binaryType = "arraybuffer";
    ws.onmessage = (e: MessageEvent) => {
      const buf = e.data;
      if (!(buf instanceof ArrayBuffer)) return;
      const handle = peekHandle(buf);
      if (!handle) return;
      latestByHandle.set(handle, buf);
      scheduleDrain();
    };
    ws.onerror = () => {
      self.postMessage({ type: "diagnostic", message: "ingest ws error" });
    };
    ws.onclose = () => {
      if (ingestWs === ws) ingestWs = null;
    };
    ingestWs = ws;
  } catch (err) {
    self.postMessage({ type: "diagnostic", message: `ingest ws connect failed: ${String(err)}` });
  }
}

self.onmessage = (event: MessageEvent) => {
  const data = event.data as {
    type: string;
    streamKey?: string;
    url?: string;
    handle?: string;
    schemaId?: string;
    schemaName?: string;
    vizRole?: string;
  };
  switch (data.type) {
    case "connectIngest":
      if (data.url) connectIngest(data.url);
      break;
    case "mapStream":
      if (data.handle && data.streamKey) {
        handleToStream.set(data.handle, {
          streamKey: data.streamKey,
          schemaId: data.schemaId ?? "",
          schemaName: data.schemaName ?? "",
          vizRole: data.vizRole ?? "",
        });
      }
      break;
    case "unmapStream":
      if (data.handle) {
        handleToStream.delete(data.handle);
        latestByHandle.delete(data.handle);
      }
      break;
  }
};
