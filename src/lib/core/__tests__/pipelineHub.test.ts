import { describe, it, expect, vi, beforeEach } from "vitest";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
  Channel: class {
    onmessage?: (buf: ArrayBuffer | unknown) => void;
  },
}));

vi.mock("$lib/core/platform", () => ({
  isTauri: () => true,
  isBrowser: () => false,
}));

const decoderRegister = vi.fn();
const decoderUnregister = vi.fn();
let lastWorkerCallback: ((frame: unknown) => void) | null = null;

vi.mock("$lib/workers/decoderManager", () => ({
  decoderWorker: {
    registerStream: (key: string, cb: (frame: unknown) => void) => {
      decoderRegister(key, cb);
      lastWorkerCallback = cb;
    },
    unregisterStream: (key: string) => decoderUnregister(key),
    connectIngest: () => {},
    mapStream: () => {},
    unmapStream: () => {},
  },
}));

beforeEach(() => {
  invokeMock.mockReset();
  decoderRegister.mockReset();
  decoderUnregister.mockReset();
  lastWorkerCallback = null;
  invokeMock.mockImplementation((command: string) => {
    if (command === "ingest_ws_port") return Promise.resolve(54321);
    if (command === "pipeline_subscribe_topic") {
      return Promise.resolve({
        subscription_id: "sub-1",
        schema_id: "sid-abc",
        schema_name: "std_msgs/String",
        viz_role: "text",
      });
    }
    if (command === "pipeline_unsubscribe") return Promise.resolve();
    if (command === "pipeline_open_foxglove") return Promise.resolve("conn-1");
    if (command === "pipeline_open_rosbridge") return Promise.resolve("conn-2");
    if (command === "pipeline_close") return Promise.resolve();
    return Promise.reject(new Error(`unexpected command ${command}`));
  });
});

async function loadHub() {
  vi.resetModules();
  const module = await import("$lib/core/pipelineHub");
  return module.pipelineHub;
}

describe("pipelineHub", () => {
  it("opens one backend subscribe per (connection, topic)", async () => {
    const hub = await loadHub();
    const a = await hub.subscribe("conn-1", "/scan");
    const b = await hub.subscribe("conn-1", "/scan");
    const calls = invokeMock.mock.calls.filter((c) => c[0] === "pipeline_subscribe_topic");
    expect(calls).toHaveLength(1);
    expect(a.schemaName).toBe("std_msgs/String");
    expect(b.vizRole).toBe("text");
  });

  it("opens a fresh backend subscribe for a different topic", async () => {
    const hub = await loadHub();
    await hub.subscribe("conn-1", "/scan");
    await hub.subscribe("conn-1", "/odom");
    const calls = invokeMock.mock.calls.filter((c) => c[0] === "pipeline_subscribe_topic");
    expect(calls).toHaveLength(2);
  });

  it("ref-counts disposal: last dispose triggers backend unsubscribe", async () => {
    const hub = await loadHub();
    const a = await hub.subscribe("conn-1", "/scan");
    const b = await hub.subscribe("conn-1", "/scan");

    await a.dispose();
    const unsubAfterFirst = invokeMock.mock.calls.filter((c) => c[0] === "pipeline_unsubscribe");
    expect(unsubAfterFirst).toHaveLength(0);

    await b.dispose();
    const unsubAfterSecond = invokeMock.mock.calls.filter((c) => c[0] === "pipeline_unsubscribe");
    expect(unsubAfterSecond).toHaveLength(1);
    expect(unsubAfterSecond[0][1]).toEqual({ subscriptionId: "sub-1" });
  });

  it("registers a decoder-worker stream keyed by (connection, topic)", async () => {
    const hub = await loadHub();
    await hub.subscribe("conn-1", "/scan");
    expect(decoderRegister).toHaveBeenCalledWith("conn-1:/scan", expect.any(Function));
  });

  it("delivers decoded frames to onFrame callbacks", async () => {
    const hub = await loadHub();
    const stream = await hub.subscribe("conn-1", "/scan");
    const received: unknown[] = [];
    stream.onFrame((frame) => received.push(frame));
    expect(lastWorkerCallback).not.toBeNull();
    const fakeFrame = { schemaName: "std_msgs/String", json: { data: "hi" } } as unknown;
    lastWorkerCallback!(fakeFrame);
    expect(received).toEqual([fakeFrame]);
  });

  it("late onFrame attaches see the latest cached frame on subscribe", async () => {
    const hub = await loadHub();
    const stream = await hub.subscribe("conn-1", "/scan");
    const fakeFrame = { schemaName: "x", json: 1 } as unknown;
    lastWorkerCallback!(fakeFrame);

    const received: unknown[] = [];
    stream.onFrame((frame) => received.push(frame));
    expect(received).toEqual([fakeFrame]);
  });

  it("exposes openFoxglove and openRosbridge invokers that pass only the url", async () => {
    const hub = await loadHub();
    const fox = await hub.openFoxglove("ws://localhost:9091");
    const ros = await hub.openRosbridge("ws://localhost:9089");
    expect(fox).toBe("conn-1");
    expect(ros).toBe("conn-2");
    const foxCall = invokeMock.mock.calls.find((c) => c[0] === "pipeline_open_foxglove");
    expect(foxCall?.[1]).toEqual({ url: "ws://localhost:9091" });
    const rosCall = invokeMock.mock.calls.find((c) => c[0] === "pipeline_open_rosbridge");
    expect(rosCall?.[1]).toEqual({ url: "ws://localhost:9089" });
  });

  it("exposes a Stream-compatible shape (key, latest.value, dispose)", async () => {
    const hub = await loadHub();
    const stream = await hub.subscribe("conn-1", "/scan");
    expect(stream.key).toBe("conn-1:/scan");
    expect(stream.connectionId).toBe("conn-1");
    expect(stream.topic).toBe("/scan");
    expect(stream.latest.value).toBeNull();

    const fakeFrame = { schemaName: "x" } as unknown;
    lastWorkerCallback!(fakeFrame);
    expect(stream.latest.value).toBe(fakeFrame);
  });

  it("dispose is idempotent", async () => {
    const hub = await loadHub();
    const stream = await hub.subscribe("conn-1", "/scan");
    await stream.dispose();
    await stream.dispose();
    const unsubCalls = invokeMock.mock.calls.filter((c) => c[0] === "pipeline_unsubscribe");
    expect(unsubCalls).toHaveLength(1);
  });

  it("injects the hub default targetHz when the caller omits options", async () => {
    const hub = await loadHub();
    await hub.subscribe("conn-1", "/scan");
    const call = invokeMock.mock.calls.find((c) => c[0] === "pipeline_subscribe_topic");
    expect(call?.[1]).toMatchObject({
      connectionId: "conn-1",
      topic: "/scan",
      options: { target_hz: 60 },
    });
  });

  it("respects an explicit targetHz: 0 as an uncapped opt-out", async () => {
    const hub = await loadHub();
    await hub.subscribe("conn-1", "/scan", { targetHz: 0 });
    const call = invokeMock.mock.calls.find((c) => c[0] === "pipeline_subscribe_topic");
    expect(call?.[1]).toMatchObject({
      options: { target_hz: 0 },
    });
  });

  it("respects an explicit higher targetHz override", async () => {
    const hub = await loadHub();
    await hub.subscribe("conn-1", "/scan", { targetHz: 240 });
    const call = invokeMock.mock.calls.find((c) => c[0] === "pipeline_subscribe_topic");
    expect(call?.[1]).toMatchObject({
      options: { target_hz: 240 },
    });
  });
});
