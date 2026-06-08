import { describe, it, expect } from "vitest";
import { decodeCborPayload } from "../cborDecode";

function enc(value: unknown): Uint8Array {
  const out: number[] = [];
  write(value, out);
  return new Uint8Array(out);
}

function writeHeader(major: number, arg: number, out: number[]): void {
  if (arg < 24) {
    out.push((major << 5) | arg);
  } else if (arg < 256) {
    out.push((major << 5) | 24, arg);
  } else if (arg < 65536) {
    out.push((major << 5) | 25, (arg >> 8) & 0xff, arg & 0xff);
  } else {
    out.push((major << 5) | 26);
    out.push((arg >>> 24) & 0xff, (arg >>> 16) & 0xff, (arg >>> 8) & 0xff, arg & 0xff);
  }
}

function write(value: unknown, out: number[]): void {
  if (value === null) {
    out.push(0xf6);
  } else if (value === true) {
    out.push(0xf5);
  } else if (value === false) {
    out.push(0xf4);
  } else if (typeof value === "number") {
    if (Number.isInteger(value)) {
      if (value >= 0) writeHeader(0, value, out);
      else writeHeader(1, -value - 1, out);
    } else {
      out.push(0xfb);
      const v = new DataView(new ArrayBuffer(8));
      v.setFloat64(0, value, false);
      for (let i = 0; i < 8; i++) out.push(v.getUint8(i));
    }
  } else if (typeof value === "string") {
    const bytes = new TextEncoder().encode(value);
    writeHeader(3, bytes.length, out);
    for (const b of bytes) out.push(b);
  } else if (Array.isArray(value)) {
    writeHeader(4, value.length, out);
    for (const item of value) write(item, out);
  } else if (typeof value === "object") {
    const keys = Object.keys(value as Record<string, unknown>);
    writeHeader(5, keys.length, out);
    for (const k of keys) {
      write(k, out);
      write((value as Record<string, unknown>)[k], out);
    }
  } else {
    throw new Error(`enc: unsupported ${typeof value}`);
  }
}

describe("decodeCborPayload", () => {
  it("decodes a primitive {kind, value} envelope (f64)", () => {
    const bytes = enc({ kind: "f64", value: 1.5 });
    expect(decodeCborPayload(bytes)).toEqual({ kind: "f64", value: 1.5 });
  });

  it("decodes nested structs the way the JSON path does", () => {
    const v = {
      kind: "struct",
      value: {
        joint_position: {
          kind: "array",
          value: [
            { kind: "f64", value: 0.1 },
            { kind: "f64", value: 0.2 },
          ],
        },
        name: { kind: "string", value: "rh_FFJ3" },
      },
    };
    expect(decodeCborPayload(enc(v))).toEqual(v);
  });

  it("decodes booleans and null", () => {
    expect(decodeCborPayload(enc({ kind: "bool", value: true }))).toEqual({
      kind: "bool",
      value: true,
    });
    expect(decodeCborPayload(enc({ kind: "null" }))).toEqual({ kind: "null" });
  });

  it("decodes negative integers via major 1", () => {
    expect(decodeCborPayload(enc({ kind: "int", value: -42 }))).toEqual({
      kind: "int",
      value: -42,
    });
  });

  it("throws on trailing garbage rather than silently truncating", () => {
    const ok = enc({ kind: "bool", value: true });
    const dirty = new Uint8Array(ok.length + 2);
    dirty.set(ok, 0);
    dirty[ok.length] = 0xf5;
    dirty[ok.length + 1] = 0xf5;
    expect(() => decodeCborPayload(dirty)).toThrow(/trailing/);
  });
});
