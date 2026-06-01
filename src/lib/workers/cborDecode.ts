const TEXT_DECODER = new TextDecoder("utf-8");

export function decodeCborPayload(bytes: Uint8Array): unknown {
  const r = new Reader(bytes);
  const v = readItem(r);
  if (r.offset !== r.end) {
    throw new Error(`cbor: ${r.end - r.offset} trailing bytes`);
  }
  return v;
}

class Reader {
  readonly view: DataView;
  readonly bytes: Uint8Array;
  readonly end: number;
  offset: number;
  constructor(bytes: Uint8Array) {
    this.bytes = bytes;
    this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    this.offset = 0;
    this.end = bytes.byteLength;
  }
  require(n: number): void {
    if (this.offset + n > this.end) {
      throw new Error(`cbor: short read (need ${n} at ${this.offset}/${this.end})`);
    }
  }
}

function readItem(r: Reader): unknown {
  r.require(1);
  const initial = r.bytes[r.offset++];
  const major = initial >> 5;
  const minor = initial & 0x1f;

  if (major === 0) return readArg(r, minor);
  if (major === 1) {
    const a = readArg(r, minor);
    return typeof a === "bigint" ? -(a as bigint) - 1n : -((a as number) + 1);
  }
  if (major === 2) {
    const len = sizeArg(r, minor);
    r.require(len);
    const out = new Array<number>(len);
    for (let i = 0; i < len; i++) out[i] = r.bytes[r.offset + i];
    r.offset += len;
    return out;
  }
  if (major === 3) {
    const len = sizeArg(r, minor);
    r.require(len);
    const s = TEXT_DECODER.decode(r.bytes.subarray(r.offset, r.offset + len));
    r.offset += len;
    return s;
  }
  if (major === 4) {
    const len = sizeArg(r, minor);
    const out = new Array<unknown>(len);
    for (let i = 0; i < len; i++) out[i] = readItem(r);
    return out;
  }
  if (major === 5) {
    const len = sizeArg(r, minor);
    const out: Record<string, unknown> = {};
    for (let i = 0; i < len; i++) {
      const k = readItem(r);
      if (typeof k !== "string") {
        throw new Error(`cbor: non-string map key at offset ${r.offset}`);
      }
      out[k] = readItem(r);
    }
    return out;
  }
  if (major === 7) {
    if (minor === 20) return false;
    if (minor === 21) return true;
    if (minor === 22) return null;
    if (minor === 23) return undefined;
    if (minor === 25) {
      r.require(2);
      const u = r.view.getUint16(r.offset, false);
      r.offset += 2;
      return decodeFloat16(u);
    }
    if (minor === 26) {
      r.require(4);
      const v = r.view.getFloat32(r.offset, false);
      r.offset += 4;
      return v;
    }
    if (minor === 27) {
      r.require(8);
      const v = r.view.getFloat64(r.offset, false);
      r.offset += 8;
      return v;
    }
    throw new Error(`cbor: unsupported special minor=${minor}`);
  }
  throw new Error(`cbor: unsupported major=${major} minor=${minor} at offset ${r.offset - 1}`);
}

function readArg(r: Reader, minor: number): number | bigint {
  if (minor < 24) return minor;
  if (minor === 24) {
    r.require(1);
    return r.bytes[r.offset++];
  }
  if (minor === 25) {
    r.require(2);
    const v = r.view.getUint16(r.offset, false);
    r.offset += 2;
    return v;
  }
  if (minor === 26) {
    r.require(4);
    const v = r.view.getUint32(r.offset, false);
    r.offset += 4;
    return v;
  }
  if (minor === 27) {
    r.require(8);
    const hi = r.view.getUint32(r.offset, false);
    const lo = r.view.getUint32(r.offset + 4, false);
    r.offset += 8;
    if (hi === 0 || (hi < 0x200000 && hi * 0x1_0000_0000 + lo <= Number.MAX_SAFE_INTEGER)) {
      return hi * 0x1_0000_0000 + lo;
    }
    return (BigInt(hi) << 32n) | BigInt(lo);
  }
  throw new Error(`cbor: indefinite-length / reserved minor=${minor}`);
}

function sizeArg(r: Reader, minor: number): number {
  const v = readArg(r, minor);
  if (typeof v === "bigint") {
    throw new Error(`cbor: collection length exceeds Number.MAX_SAFE_INTEGER`);
  }
  return v;
}

function decodeFloat16(u: number): number {
  const sign = (u & 0x8000) !== 0 ? -1 : 1;
  const exp = (u >> 10) & 0x1f;
  const frac = u & 0x3ff;
  if (exp === 0) return sign * Math.pow(2, -14) * (frac / 1024);
  if (exp === 0x1f) return frac === 0 ? sign * Infinity : NaN;
  return sign * Math.pow(2, exp - 15) * (1 + frac / 1024);
}
