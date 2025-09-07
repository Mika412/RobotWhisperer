/// <reference lib="webworker" />
import type {
  ServerMsg,
  ClientMsg,
  ChannelIndex,
} from "../ros/protocol/foxglove";

const topicWorkerUrl = new URL("./topic.worker.ts", import.meta.url);

let ws: WebSocket | null = null;
let channels: ChannelIndex = { byId: new Map(), byTopic: new Map() };
let topicWorkers = new Map<number, Worker>(); // channelId -> worker
let rpcResolvers = new Map<string, (msg: any) => void>();
let lastTopics: { name: string; type: string }[] = [];
let lastServices: { name: string; type: string }[] = [];

let nextSubId = 0;
const subscriptions = new Map<number, number>(); // subId -> channelId

function post(type: string, payload?: any) {
  (self as any).postMessage({ type, payload });
}
function rpcRespond(rid: string, ok: boolean, result?: any, error?: any) {
  (self as any).postMessage({ type: "rpc", rid, ok, result, error });
}

self.onmessage = (e) => {
  const { type, id, cmd, payload } = e.data;
  if (type === "connect") start(payload.url || "ws://localhost:8765");
  if (type === "rpc") handleRpc(id, cmd, payload);
};

function start(url: string) {
  teardown();
  ws = new WebSocket(url, "foxglove.websocket.v1");
  ws.binaryType = "arraybuffer";
  ws.onopen = () => post("status", { connected: true, info: "connected" });
  ws.onclose = () => post("status", { connected: false, info: "closed" });
  ws.onerror = (ev) => post("status", { connected: false, info: "error" });
  ws.onmessage = (ev) => handleServerMessage(ev.data);
}

function teardown() {
  ws?.close();
  ws = null;
  for (const [, w] of topicWorkers) w.terminate();
  topicWorkers.clear();
  channels.byId.clear();
  channels.byTopic.clear();
}

function handleServerMessage(data: any) {
  // JSON control frames (serverInfo, advertise, serviceResponse, etc.)
  if (typeof data === "string") {
    const msg = JSON.parse(data) as ServerMsg;
    if ((msg as any).op === "serverInfo") {
      post("ready");
      return;
    }

    if ((msg as any).op === "advertise") {
      const adv = (msg as any).channels as Array<{
        id: number;
        topic: string;
        schemaName: string;
        schema: string;
        encoding: string;
      }>;

      for (const c of adv) {
        const schemaEncoding = guessSchemaEncoding(c.schema);
        console.log("Schema encoding is", schemaEncoding);
        console.log(c.schema);
        channels.byId.set(c.id, {
          topic: c.topic,
          type: c.schemaName,
          encoding: c.encoding,
          schema: c.schema,
          schemaEncoding,
        });
        channels.byTopic.set(c.topic, c.id);
      }
      console.log("Publishing topics");
      // Publish live topic list
      lastTopics = adv.map((c) => ({ name: c.topic, type: c.schemaName }));
      post("topics", lastTopics);

      console.log("Publishing schema");
      // NEW: surface schemas for caching in the main thread
      post(
        "schemas",
        adv.map((c) => ({
          messageType: c.schemaName,
          definition: c.schema,
          encoding: guessSchemaEncoding(c.schema),
        })),
      );
      console.log("End");
      return;
    }

    if ((msg as any).op === "message") {
      // Some servers can send JSON `message` envelopes with base64 payloads.
      const m = msg as any;
      const ch = channels.byId.get(m.channelId);
      if (!ch) return;

      // If `m.data` is base64, turn into ArrayBuffer
      let abuf: ArrayBuffer;
      if (typeof m.data === "string") {
        const b = atob(m.data);
        const u8 = new Uint8Array(b.length);
        for (let i = 0; i < b.length; i++) u8[i] = b.charCodeAt(i);
        abuf = u8.buffer;
      } else {
        // If server serialized it differently, ignore here.
        return;
      }
      forwardToTopicWorker(m.channelId, ch, abuf);
      return;
    }

    if ((msg as any).op === "serviceResponse") {
      const r = msg as any;
      const resolver = rpcResolvers.get(String(r.id));
      if (resolver) resolver(r);
      return;
    }

    return;
  }

  console.log("Got message", data);
  // BINARY FRAMES (fast path: Foxglove Bridge typically uses binary for data)
  // Expected layout (common Foxglove framing):
  // [0..3]   u32   channelId (little-endian)
  // [4..11]  u64   timestamp (nanoseconds since epoch)  (optional on some servers)
  // [..]     rest  payload bytes
  if (data instanceof ArrayBuffer || ArrayBuffer.isView(data)) {
    const buf =
      data instanceof ArrayBuffer ? data : (data as ArrayBufferView).buffer;
    const dv = new DataView(buf);
    if (dv.byteLength < 8) return; // too short to contain header

    // Try to decode channelId first (LE)
    let channelId = dv.getUint32(0, /*littleEndian*/ true);
    channelId = 3;
    let payloadOffset = 4;

    // If we have at least 12 bytes, assume there is a u64 timestamp and skip it.
    if (dv.byteLength >= 12) payloadOffset = 12;

    const ch = channels.byId.get(channelId);
    if (!ch) return;
    console.log("FUUUCKCKC");
    const payload = buf.slice(payloadOffset); // transferable
    forwardToTopicWorker(channelId, ch, payload as ArrayBuffer);
    return;
  }

  // Unknown frame type – ignore
}

function forwardToTopicWorker(
  channelId: number,
  ch: any,
  payload: ArrayBuffer,
) {
  let worker = topicWorkers.get(channelId);
  console.log("IM HERE WORKER FICK", channelId, worker);
  if (!worker) {
    const url = new URL("./topic.worker.ts", import.meta.url);
    worker = new Worker(url, { type: "module" });
    worker.onmessage = (e) => {
      post("message", e.data);
    };
    console.log("SURVIVDED");
    topicWorkers.set(channelId, worker);
  }
  // Transfer the payload (zero-copy)
  worker.postMessage({ type: "data", channel: ch, data: payload }, [payload]);
}

function handleRpc(rid: string, cmd: string, payload: any) {
  try {
    switch (cmd) {
      case "listTopics":
        rpcRespond(rid, true, lastTopics);
        break;
      case "listServices":
        rpcRespond(rid, true, lastServices);
        break;
      case "listActions":
        rpcRespond(rid, true, []);
        break;
      case "subscribe": {
        const topicName = payload.name;
        const channelId = channels.byTopic.get(topicName);
        console.log("CHanneles", channels);
        console.log(channelId);

        if (channelId === undefined) {
          return rpcRespond(
            rid,
            false,
            undefined,
            `Unknown topic: ${topicName}`,
          );
        }

        const subId = nextSubId++;
        subscriptions.set(subId, channelId);

        const msg: ClientMsg = {
          op: "subscribe",
          // This object now correctly includes the 'id'
          subscriptions: [{ id: subId, channelId: channelId }],
        };
        ws?.send(JSON.stringify(msg));
        // Respond with the new ID so it can be used to unsubscribe later
        rpcRespond(rid, true, String(subId));
        break;
      }

      case "unsubscribe": {
        const subId = parseInt(payload.subId, 10);
        if (subscriptions.has(subId)) {
          const msg: ClientMsg = {
            op: "unsubscribe",
            subscriptions: [{ id: subId }],
          };
          ws?.send(JSON.stringify(msg));
          subscriptions.delete(subId);
          rpcRespond(rid, true);
        } else {
          rpcRespond(rid, false, undefined, "subscription not found");
        }
        break;
      }
      case "callService": {
        const { name, request } = payload;
        // Minimal example: Foxglove callService — assign id and await response
        const callId = Math.floor(Math.random() * 1e9);
        rpcResolvers.set(String(callId), (resp) => {
          rpcResolvers.delete(String(callId));
          if (resp.ok) rpcRespond(rid, true, resp.data);
          else rpcRespond(rid, false, null, resp.error);
        });
        const msg: ClientMsg = {
          op: "callService",
          id: callId,
          service: name,
          encoding: "json",
          data: JSON.stringify(request),
        };
        ws?.send(JSON.stringify(msg));
        break;
      }
      default:
        rpcRespond(rid, false, null, `unknown cmd ${cmd}`);
    }
  } catch (e: any) {
    rpcRespond(rid, false, null, e?.message || String(e));
  }
}

function guessSchemaEncoding(txt: string): string {
  if (txt?.startsWith("MSG:") || txt?.includes("#")) return "ros2msg";
  if (txt?.trim()?.startsWith("{")) return "jsonschema";
  return "unknown";
}
