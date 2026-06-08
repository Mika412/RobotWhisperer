import { decodeCborPayload } from "./cborDecode";

export const WIRE_VERSION = 4;

export const enum PayloadKind {
  RawCdr = 0,
  DecodedJson = 1,
  ImageRgba = 2,
  MarkerPositions = 3,
}

export const enum FrameKind {
  Value = 1,
  Image = 2,
  PointCloud = 3,
  Packed = 4,
  Error = 5,
}

export const FrameFlags = Object.freeze({
  StaleReplay: 1 << 0,
  LossyDropped: 1 << 1,
  ImageCompressed: 1 << 2,
  PerfTrace: 1 << 3,
  PayloadCbor: 1 << 4,
} as const);

export interface PerfTrace {
  wsRecvNs: number;
  decodeStartNs: number;
  decodeEndNs: number;
  packStartNs: number;
  channelSendNs: number;
  workerRecvNs: number;
  workerDecodedNs: number;
  hubDispatchNs?: number;
  paneRecvNs?: number;
  paneAppliedNs?: number;
}

export const PERF_TRACE_SIZE = 5 * 8;

export interface FrameMeta {
  schemaId: string;
  schemaName: string;
  vizRole: string;
}

export interface DecodedFramePayload {
  wireVersion: number;
  timestampNs: bigint;
  schemaId: string;
  schemaHash: string;
  schemaName: string;
  vizRole: string;
  flags: number;
  kind: PayloadKind;
  frameKind: FrameKind;
  json: unknown | null;
  rgba: Uint8ClampedArray | null;
  width: number;
  height: number;
  positions: Float32Array | null;
  raw: Uint8Array | null;
  perf?: PerfTrace;
}

interface Envelope {
  timestampNs: bigint;
  flags: number;
  frameKind: FrameKind;
  buffer: ArrayBuffer;
  payloadOffset: number;
  payloadLength: number;
  backendPerf?: {
    wsRecvNs: number;
    decodeStartNs: number;
    decodeEndNs: number;
    packStartNs: number;
    channelSendNs: number;
  };
}

const textDecoder = new TextDecoder();

let diagnosticSink: (message: string) => void = (message) => {
  console.warn("[decoder]", message);
};
export function setDiagnosticSink(sink: (message: string) => void): void {
  diagnosticSink = sink;
}

let badVersionWarned = false;
function warnOnce(message: string): void {
  if (badVersionWarned) return;
  badVersionWarned = true;
  diagnosticSink(message);
}

export function perfNowNs(): number {
  return Math.round((performance.timeOrigin + performance.now()) * 1e6);
}

export function peekHandle(buffer: ArrayBuffer): string | null {
  const bytes = new Uint8Array(buffer);
  if (bytes.length < 16 || bytes[0] !== WIRE_VERSION) return null;
  const view = new DataView(buffer);
  const handleLen = view.getUint32(12, true);
  if (bytes.length < 16 + handleLen) return null;
  return textDecoder.decode(bytes.subarray(16, 16 + handleLen));
}

function parseEnvelope(buffer: ArrayBuffer): Envelope | null {
  const bytes = new Uint8Array(buffer);
  if (bytes.length < 12) {
    warnOnce(`decoder: dropped frame, only ${bytes.length} bytes`);
    return null;
  }
  const version = bytes[0];
  if (version !== WIRE_VERSION) {
    warnOnce(`decoder: unsupported wire version ${version}, first 16 bytes: ${dumpHex(bytes, 16)}`);
    return null;
  }
  const view = new DataView(buffer);
  const frameKind = frameKindFromByte(bytes[1]);
  if (frameKind === null) {
    warnOnce(`decoder: unknown frame kind ${bytes[1]}`);
    return null;
  }
  const flags = view.getUint16(2, true);
  const timestampNs = view.getBigUint64(4, true);

  let cursor = 12;
  if (bytes.length < cursor + 4) return null;
  const handleLen = view.getUint32(cursor, true);
  cursor += 4 + handleLen;

  if (bytes.length < cursor + 4) return null;
  const payloadLen = view.getUint32(cursor, true);
  cursor += 4;
  if (bytes.length < cursor + payloadLen) return null;

  let backendPerf: Envelope["backendPerf"] = undefined;
  const tailOffset = cursor + payloadLen;
  if ((flags & FrameFlags.PerfTrace) !== 0 && bytes.length >= tailOffset + PERF_TRACE_SIZE) {
    const readU64 = (offset: number) =>
      view.getUint32(offset, true) + view.getUint32(offset + 4, true) * 0x1_0000_0000;
    backendPerf = {
      wsRecvNs: readU64(tailOffset),
      decodeStartNs: readU64(tailOffset + 8),
      decodeEndNs: readU64(tailOffset + 16),
      packStartNs: readU64(tailOffset + 24),
      channelSendNs: readU64(tailOffset + 32),
    };
  }

  return {
    timestampNs,
    flags,
    frameKind,
    buffer,
    payloadOffset: cursor,
    payloadLength: payloadLen,
    backendPerf,
  };
}

function frameKindFromByte(byte: number): FrameKind | null {
  switch (byte) {
    case 1:
      return FrameKind.Value;
    case 2:
      return FrameKind.Image;
    case 3:
      return FrameKind.PointCloud;
    case 4:
      return FrameKind.Packed;
    case 5:
      return FrameKind.Error;
    default:
      return null;
  }
}

function payloadKindFromFrameKind(kind: FrameKind): PayloadKind {
  switch (kind) {
    case FrameKind.Image:
      return PayloadKind.ImageRgba;
    case FrameKind.Packed:
      return PayloadKind.MarkerPositions;
    case FrameKind.Value:
    case FrameKind.PointCloud:
    case FrameKind.Error:
    default:
      return PayloadKind.DecodedJson;
  }
}

function decodeImageRgba(
  env: Envelope,
): { rgba: Uint8ClampedArray; width: number; height: number } | null {
  if (env.payloadLength < 16) return null;
  const view = new DataView(env.buffer, env.payloadOffset, env.payloadLength);
  const width = view.getUint32(0, true);
  const height = view.getUint32(4, true);
  const pixelStart = env.payloadOffset + 16;
  const pixelLen = width * height * 4;
  if (env.payloadOffset + env.payloadLength < pixelStart + pixelLen) return null;
  const rgba = new Uint8ClampedArray(env.buffer, pixelStart, pixelLen);
  return { rgba, width, height };
}

function decodePositions(env: Envelope): Float32Array | null {
  if (env.payloadLength < 4) return null;
  if ((env.payloadOffset & 3) !== 0) {
    const copy = new Uint8Array(env.payloadLength);
    copy.set(new Uint8Array(env.buffer, env.payloadOffset, env.payloadLength));
    return new Float32Array(copy.buffer, 0, copy.byteLength / 4);
  }
  return new Float32Array(env.buffer, env.payloadOffset, env.payloadLength / 4);
}

function decodeJson(env: Envelope): unknown | null {
  try {
    const slice = new Uint8Array(env.buffer, env.payloadOffset, env.payloadLength);
    return JSON.parse(textDecoder.decode(slice));
  } catch {
    return null;
  }
}

function decodeValuePayload(env: Envelope): unknown | null {
  if ((env.flags & FrameFlags.PayloadCbor) !== 0) {
    try {
      const slice = new Uint8Array(env.buffer, env.payloadOffset, env.payloadLength);
      return decodeCborPayload(slice);
    } catch (err) {
      warnOnce(`decoder: CBOR decode failed: ${(err as Error).message}`);
      return null;
    }
  }
  return decodeJson(env);
}

function dumpHex(bytes: Uint8Array, count: number): string {
  return Array.from(bytes.subarray(0, Math.min(count, bytes.length)))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join(" ");
}

export function decodeFrameBuffer(
  buffer: ArrayBuffer,
  workerRecvNs: number,
  meta?: FrameMeta,
): { frame: DecodedFramePayload; transfer: Transferable[] } | null {
  const env = parseEnvelope(buffer);
  if (!env) return null;

  const frame: DecodedFramePayload = {
    wireVersion: WIRE_VERSION,
    timestampNs: env.timestampNs,
    schemaId: meta?.schemaId ?? "",
    schemaHash: meta?.schemaId ?? "",
    schemaName: meta?.schemaName ?? "",
    vizRole: meta?.vizRole ?? "",
    flags: env.flags,
    kind: payloadKindFromFrameKind(env.frameKind),
    frameKind: env.frameKind,
    json: null,
    rgba: null,
    width: 0,
    height: 0,
    positions: null,
    raw: null,
  };

  switch (env.frameKind) {
    case FrameKind.Image: {
      const image = decodeImageRgba(env);
      if (image) {
        frame.rgba = image.rgba;
        frame.width = image.width;
        frame.height = image.height;
      }
      break;
    }
    case FrameKind.Packed: {
      frame.positions = decodePositions(env);
      break;
    }
    case FrameKind.Value:
    case FrameKind.PointCloud:
    case FrameKind.Error: {
      frame.json = decodeValuePayload(env);
      break;
    }
  }

  if (env.backendPerf) {
    const workerDecodedNs = perfNowNs();
    frame.perf = {
      wsRecvNs: env.backendPerf.wsRecvNs,
      decodeStartNs: env.backendPerf.decodeStartNs,
      decodeEndNs: env.backendPerf.decodeEndNs,
      packStartNs: env.backendPerf.packStartNs,
      channelSendNs: env.backendPerf.channelSendNs,
      workerRecvNs,
      workerDecodedNs,
    };
  }

  return { frame, transfer: [env.buffer] };
}
