import { describe, it, expect } from "vitest";
import {
  decodeFrameBuffer,
  FrameKind,
  FrameFlags,
  PayloadKind,
  WIRE_VERSION,
  PERF_TRACE_SIZE,
} from "$lib/workers/decoderCore";

function packV4(args: {
  handle: string;
  timestampNs: bigint;
  frameKind: FrameKind;
  flags: number;
  payload: Uint8Array;
  perf?: bigint[];
}): ArrayBuffer {
  const handleBytes = new TextEncoder().encode(args.handle);
  const tail = args.perf ? PERF_TRACE_SIZE : 0;
  const total = 1 + 1 + 2 + 8 + 4 + handleBytes.length + 4 + args.payload.length + tail;
  const buf = new ArrayBuffer(total);
  const view = new DataView(buf);
  const u8 = new Uint8Array(buf);
  let o = 0;
  view.setUint8(o++, WIRE_VERSION);
  view.setUint8(o++, args.frameKind);
  view.setUint16(o, args.flags, true);
  o += 2;
  view.setBigUint64(o, args.timestampNs, true);
  o += 8;
  view.setUint32(o, handleBytes.length, true);
  o += 4;
  u8.set(handleBytes, o);
  o += handleBytes.length;
  view.setUint32(o, args.payload.length, true);
  o += 4;
  u8.set(args.payload, o);
  o += args.payload.length;
  if (args.perf) {
    for (const stamp of args.perf) {
      view.setBigUint64(o, stamp, true);
      o += 8;
    }
  }
  return buf;
}

const META = { schemaId: "sid-abc", schemaName: "std_msgs/Float64", vizRole: "plot:data" };

const CBOR_F64 = new Uint8Array([
  0xa2, 0x64, 0x6b, 0x69, 0x6e, 0x64, 0x63, 0x66, 0x36, 0x34, 0x65, 0x76, 0x61, 0x6c, 0x75, 0x65,
  0xfb, 0x3f, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
]);

describe("frame decode (decodeFrameBuffer, v4)", () => {
  it("decodes a CBOR value frame and attaches the supplied schema meta", () => {
    const buf = packV4({
      handle: "h",
      timestampNs: 7n,
      frameKind: FrameKind.Value,
      flags: FrameFlags.PayloadCbor,
      payload: CBOR_F64,
    });
    const decoded = decodeFrameBuffer(buf, 0, META);
    expect(decoded).not.toBeNull();
    const frame = decoded!.frame;
    expect(frame.wireVersion).toBe(WIRE_VERSION);
    expect(frame.frameKind).toBe(FrameKind.Value);
    expect(frame.flags & FrameFlags.PayloadCbor).not.toBe(0);
    expect(frame.json).toEqual({ kind: "f64", value: 1.5 });
    expect(frame.schemaId).toBe("sid-abc");
    expect(frame.schemaHash).toBe("sid-abc");
    expect(frame.schemaName).toBe("std_msgs/Float64");
    expect(frame.vizRole).toBe("plot:data");
  });

  it("decodes a JSON value frame and leaves schema fields empty without meta", () => {
    const json = JSON.stringify({ kind: "struct", fields: [] });
    const buf = packV4({
      handle: "h",
      timestampNs: 0n,
      frameKind: FrameKind.Value,
      flags: 0,
      payload: new TextEncoder().encode(json),
    });
    const decoded = decodeFrameBuffer(buf, 0);
    expect(decoded).not.toBeNull();
    const frame = decoded!.frame;
    expect(frame.json).toEqual({ kind: "struct", fields: [] });
    expect(frame.kind).toBe(PayloadKind.DecodedJson);
    expect(frame.schemaName).toBe("");
  });

  it("decodes an image frame into rgba + dimensions", () => {
    const payload = new Uint8Array(16 + 4);
    const view = new DataView(payload.buffer);
    view.setUint32(0, 1, true);
    view.setUint32(4, 1, true);
    payload.set([10, 20, 30, 40], 16);
    const buf = packV4({
      handle: "h",
      timestampNs: 0n,
      frameKind: FrameKind.Image,
      flags: 0,
      payload,
    });
    const frame = decodeFrameBuffer(buf, 0)!.frame;
    expect(frame.frameKind).toBe(FrameKind.Image);
    expect(frame.width).toBe(1);
    expect(frame.height).toBe(1);
    expect(frame.rgba).toBeInstanceOf(Uint8ClampedArray);
    expect(Array.from(frame.rgba as Uint8ClampedArray)).toEqual([10, 20, 30, 40]);
  });

  it("parses the perf-trace tail when the flag is set", () => {
    const buf = packV4({
      handle: "h",
      timestampNs: 0n,
      frameKind: FrameKind.Value,
      flags: FrameFlags.PayloadCbor | FrameFlags.PerfTrace,
      payload: CBOR_F64,
      perf: [100n, 200n, 300n, 400n, 500n],
    });
    const frame = decodeFrameBuffer(buf, 1234, META)!.frame;
    expect(frame.perf).toBeDefined();
    expect(frame.perf!.wsRecvNs).toBe(100);
    expect(frame.perf!.channelSendNs).toBe(500);
    expect(frame.perf!.workerRecvNs).toBe(1234);
  });

  it("rejects an unsupported wire version", () => {
    const buf = packV4({
      handle: "h",
      timestampNs: 0n,
      frameKind: FrameKind.Value,
      flags: 0,
      payload: new Uint8Array([1]),
    });
    new DataView(buf).setUint8(0, 3);
    expect(decodeFrameBuffer(buf, 0)).toBeNull();
  });

  it("rejects a too-short buffer", () => {
    expect(decodeFrameBuffer(new Uint8Array([WIRE_VERSION, 1, 0]).buffer, 0)).toBeNull();
  });
});
