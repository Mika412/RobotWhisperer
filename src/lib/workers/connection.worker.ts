/// <reference lib="webworker" />
import type {
  ServerMsg,
  ClientMsg,
  ChannelIndex,
  Channel,
} from "../ros/protocol/foxglove";

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
        channels.byId.set(c.id, {
          topic: c.topic,
          type: c.schemaName,
          encoding: c.encoding,
          schema: c.schema,
          schemaEncoding,
        });
        channels.byTopic.set(c.topic, c.id);
      }

      lastTopics = adv.map((c) => ({ name: c.topic, type: c.schemaName }));
      post("topics", lastTopics);

      post(
        "schemas",
        adv.map((c) => ({
          messageType: c.schemaName,
          definition: c.schema,
          encoding: guessSchemaEncoding(c.schema),
        })),
      );
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

  if (data instanceof ArrayBuffer || ArrayBuffer.isView(data)) {
    const buf =
      data instanceof ArrayBuffer ? data : (data as ArrayBufferView).buffer;
    const dv = new DataView(buf);
    if (dv.byteLength < 8) return;

    const channelId = dv.getUint32(0, /*littleEndian*/ true);
    let payloadOffset = 4;
    if (dv.byteLength >= 12) payloadOffset = 12; // Skip timestamp

    const ch = channels.byId.get(channelId);
    if (!ch) return;
    const payload = buf.slice(payloadOffset);
    forwardToTopicWorker(channelId, ch, payload as ArrayBuffer);
    return;
  }
}

function forwardToTopicWorker(
  channelId: number,
  ch: Channel,
  payload: ArrayBuffer,
) {
  let worker = topicWorkers.get(channelId);
  if (!worker) {
    const url = new URL("./topic.worker.ts", import.meta.url);
    worker = new Worker(url, { type: "module" });
    worker.onmessage = (e) => {
      post("message", e.data);
    };
    topicWorkers.set(channelId, worker);
  }
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
        break; // Not implemented
      case "subscribe": {
        const topicName = payload.name;
        const channelId = channels.byTopic.get(topicName);

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
          subscriptions: [{ id: subId, channelId: channelId }],
        };
        ws?.send(JSON.stringify(msg));
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
      default:
        rpcRespond(rid, false, null, `unknown cmd ${cmd}`);
    }
  } catch (e: any) {
    rpcRespond(rid, false, null, e?.message || String(e));
  }
}

function guessSchemaEncoding(
  txt: string,
): "ros2msg" | "jsonschema" | "unknown" {
  if (txt?.startsWith("MSG:") || txt?.includes("#")) return "ros2msg";
  if (txt?.trim()?.startsWith("{")) return "jsonschema";
  return "unknown";
}
