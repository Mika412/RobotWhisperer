// ROS 2 CDR (XCDR v1-style) minimal decoder for common builtins + arrays/sequences.
// - Detects endianness from the 4‑byte CDR encapsulation header (0x0000 = BE, 0x0001 = LE; next 2 bytes are options).
// - Aligns primitives per CDR (1,2,4,8).
// - Supports: bool, char, octet/uint8, int8, int16, uint16, int32, uint32, int64, uint64,
//             float32, float64, string (bounded/unbounded), and arrays: T[N], T[], string[].
// - Parses a ROS 2 `.msg`-style schema string (one message), ignoring constants and comments.
// Limitations (kept intentionally simple for web use):
//   * No nested messages (e.g. std_msgs/Header) unless they are expanded into this schema.
//   * No WStrings, unions, or XCDR2 DHeader formats.
//   * No complex containers like maps.
// If you need nested types, expand them in your schemaText or extend `decodeField` to look them up.

type Field =
  | { kind: 'primitive'; name: string; type: Primitive }
  | { kind: 'array'; name: string; elemType: Primitive; length: number }       // fixed length
  | { kind: 'sequence'; name: string; elemType: Primitive }                    // variable length (uint32 length prefix)
  ;

type Primitive =
  | 'bool' | 'char' | 'byte' | 'octet' | 'uint8' | 'int8'
  | 'int16' | 'uint16' | 'int32' | 'uint32' | 'int64' | 'uint64'
  | 'float32' | 'float64' | 'string';

const BUILTINS: Record<string, Primitive> = {
  bool: 'bool',
  char: 'char',
  byte: 'byte',
  octet: 'octet',
  uint8: 'uint8',
  int8: 'int8',
  int16: 'int16',
  uint16: 'uint16',
  int32: 'int32',
  uint32: 'uint32',
  int64: 'int64',
  uint64: 'uint64',
  float32: 'float32',
  float64: 'float64',
  string: 'string',
};

function isBuiltin(t: string): t is Primitive {
  return t in BUILTINS;
}

function align(offset: number, align: number): number {
  const mask = align - 1;
  return (offset + mask) & ~mask;
}

class CdrReader {
  private view: DataView;
  private off = 0;
  private little = false;

  constructor(private buf: ArrayBuffer) {
    this.view = new DataView(buf);
    // Detect encapsulation (first 4 bytes): identifier (2 bytes) + options (2 bytes)
    if (this.view.byteLength < 4) throw new Error('Buffer too small for CDR encapsulation');
    const ident = this.view.getUint16(0, false); // identifier is big-endian by spec
    // 0x0000 => BE, 0x0001 => LE. Some writers may use 0x0003 (PL_CDR2_LE); treat LSB as endianness.
    this.little = (ident & 0x0001) === 0x0001;
    // Skip 4‑byte header
    this.off = 4;
  }

  private ensure(n: number) {
    if (this.off + n > this.view.byteLength) {
      throw new Error(`CDR: out of data (need ${n} bytes at ${this.off}, size ${this.view.byteLength})`);
    }
  }

  private alignTo(n: number) {
    const a = align(this.off, n);
    this.off = a;
  }

  readUint8(): number {
    // 1‑byte alignment
    this.alignTo(1);
    this.ensure(1);
    const v = this.view.getUint8(this.off);
    this.off += 1;
    return v;
  }
  readInt8(): number {
    this.alignTo(1);
    this.ensure(1);
    const v = this.view.getInt8(this.off);
    this.off += 1;
    return v;
  }
  readUint16(): number {
    this.alignTo(2);
    this.ensure(2);
    const v = this.view.getUint16(this.off, this.little);
    this.off += 2;
    return v;
  }
  readInt16(): number {
    this.alignTo(2);
    this.ensure(2);
    const v = this.view.getInt16(this.off, this.little);
    this.off += 2;
    return v;
  }
  readUint32(): number {
    this.alignTo(4);
    this.ensure(4);
    const v = this.view.getUint32(this.off, this.little);
    this.off += 4;
    return v;
  }
  readInt32(): number {
    this.alignTo(4);
    this.ensure(4);
    const v = this.view.getInt32(this.off, this.little);
    this.off += 4;
    return v;
  }
  readBigUint64(): bigint {
    this.alignTo(8);
    this.ensure(8);
    const v = this.view.getBigUint64(this.off, this.little);
    this.off += 8;
    return v;
  }
  readBigInt64(): bigint {
    this.alignTo(8);
    this.ensure(8);
    const v = this.view.getBigInt64(this.off, this.little);
    this.off += 8;
    return v;
  }
  readFloat32(): number {
    this.alignTo(4);
    this.ensure(4);
    const v = this.view.getFloat32(this.off, this.little);
    this.off += 4;
    return v;
  }
  readFloat64(): number {
    this.alignTo(8);
    this.ensure(8);
    const v = this.view.getFloat64(this.off, this.little);
    this.off += 8;
    return v;
  }
  readBytes(n: number): Uint8Array {
    this.alignTo(1);
    this.ensure(n);
    const v = new Uint8Array(this.buf, this.off, n);
    this.off += n;
    return new Uint8Array(v); // copy
  }

  readString(): string {
    // CDR string: 4‑byte uint length (including the terminating NUL), then bytes
    this.alignTo(4);
    const n = this.readUint32();
    // Length includes the terminating '\0'. Enforce sanity.
    if (n === 0) return '';
    const bytes = this.readBytes(n);
    // Drop terminating 0 if present
    const end = bytes[bytes.length - 1] === 0 ? bytes.length - 1 : bytes.length;
    // UTF‑8 decode
    return new TextDecoder('utf-8', { fatal: false }).decode(bytes.slice(0, end));
  }

  readPrimitive(t: Primitive): any {
    switch (t) {
      case 'bool': return this.readUint8() !== 0;
      case 'char': return this.readUint8();        // treat as unsigned char
      case 'byte':
      case 'octet':
      case 'uint8': return this.readUint8();
      case 'int8': return this.readInt8();
      case 'int16': return this.readInt16();
      case 'uint16': return this.readUint16();
      case 'int32': return this.readInt32();
      case 'uint32': return this.readUint32();
      case 'int64': return this.readBigInt64();
      case 'uint64': return this.readBigUint64();
      case 'float32': return this.readFloat32();
      case 'float64': return this.readFloat64();
      case 'string': return this.readString();
    }
  }

  readArray(elem: Primitive, n: number): any[] {
    const out: any[] = new Array(n);
    // Align to element alignment before the array (CDR rule: each element aligned as if separate).
    const alignments: Record<Primitive, number> = {
      bool: 1, char: 1, byte: 1, octet: 1, uint8: 1, int8: 1,
      int16: 2, uint16: 2, int32: 4, uint32: 4, float32: 4,
      int64: 8, uint64: 8, float64: 8, string: 4
    };
    this.alignTo(alignments[elem]);
    for (let i = 0; i < n; i++) {
      out[i] = this.readPrimitive(elem);
    }
    return out;
  }

  readSequence(elem: Primitive): any[] {
    // sequence length: uint32, 4‑byte aligned
    const n = this.readUint32();
    return this.readArray(elem, n);
  }
}

function parseMsgSchema(schemaText: string): Field[] {
  const fields: Field[] = [];
  const lines = schemaText.split(/\r?\n/);
  for (const raw of lines) {
    // Strip comments (#...) and trim
    const line = raw.replace(/#.*$/, '').trim();
    if (!line) continue;
    // constants like "uint8 FOO=1" -> ignore
    if (line.includes('=')) continue;

    // tokens: "<type>[optional array] <name>"
    const m = line.match(/^([A-Za-z0-9_\/]+)(\s*\[[0-9]*\])?\s+([A-Za-z0-9_]+)$/);
    if (!m) continue;

    const fullType = m[1];
    const arrSpec = m[2]?.trim();
    const name = m[3];

    // Only allow builtins for now
    const tnorm = fullType in BUILTINS ? BUILTINS[fullType] : undefined;
    if (!tnorm) {
      throw new Error(`Unsupported (non-builtin) field type "${fullType}" for "${name}". Expand nested types into this schema or extend the decoder.`);
    }

    if (!arrSpec) {
      fields.push({ kind: 'primitive', name, type: tnorm });
    } else if (arrSpec === '[]' || arrSpec === '[-1]') {
      fields.push({ kind: 'sequence', name, elemType: tnorm });
    } else {
      const len = parseInt(arrSpec.slice(1, -1), 10);
      if (!Number.isFinite(len) || len < 0) throw new Error(`Bad array length for ${name}: ${arrSpec}`);
      fields.push({ kind: 'array', name, elemType: tnorm, length: len });
    }
  }
  return fields;
}

function decodeField(r: CdrReader, f: Field): any {
  switch (f.kind) {
    case 'primitive':
      return r.readPrimitive(f.type);
    case 'array':
      return r.readArray(f.elemType, f.length);
    case 'sequence':
      return r.readSequence(f.elemType);
  }
}

export function decodeRos2Cdr(typeName: string, schemaText: string, buf: ArrayBuffer): any {
  const fields = parseMsgSchema(schemaText);
  const r = new CdrReader(buf);
  const out: Record<string, any> = { _type: typeName };
  for (const f of fields) {
    out[f.name] = decodeField(r, f);
  }
  return out;
}

// Helper for unit tests / debug: decode to [name, value] entries in order
export function decodeRos2CdrEntries(schemaText: string, buf: ArrayBuffer): Array<[string, any]> {
  const fields = parseMsgSchema(schemaText);
  const r = new CdrReader(buf);
  const out: Array<[string, any]> = [];
  for (const f of fields) {
    out.push([f.name, decodeField(r, f)]);
  }
  return out;
}

// Small self‑test when run under Node (ignored in browser bundlers)
if (typeof process !== 'undefined' && process.env && process.env.NODE_ENV === 'test_cdr') {
  // Construct a tiny CDR buffer: LE encapsulation, then fields: uint32 a; float64 b; string s; uint8[3] arr; int16[] seq;
  const enc = new Uint8Array([0x00, 0x01, 0x00, 0x00]); // LE, options=0
  const parts: number[] = Array.from(enc);

  // Helper to push alignment
  let offset = 4;
  const pushAlign = (n: number) => {
    const pad = (-(offset) & (n - 1));
    for (let i = 0; i < pad; i++) parts.push(0), offset++;
  };
  const pushU32 = (v: number) => { pushAlign(4); const dv = new DataView(new ArrayBuffer(4)); dv.setUint32(0, v, true); parts.push(...new Uint8Array(dv.buffer)); offset += 4; };
  const pushF64 = (v: number) => { pushAlign(8); const dv = new DataView(new ArrayBuffer(8)); dv.setFloat64(0, v, true); parts.push(...new Uint8Array(dv.buffer)); offset += 8; };
  const pushU8  = (v: number) => { pushAlign(1); parts.push(v & 0xFF); offset += 1; };
  const pushI16 = (v: number) => { pushAlign(2); const dv = new DataView(new ArrayBuffer(2)); dv.setInt16(0, v, true); parts.push(...new Uint8Array(dv.buffer)); offset += 2; };
  const pushStr = (s: string) => {
    pushAlign(4);
    const enc = new TextEncoder();
    const bytes = enc.encode(s);
    pushU32(bytes.length + 1);
    for (const b of bytes) pushU8(b);
    pushU8(0);
  };

  // Schema
  const schema = [
    'uint32 a',
    'float64 b',
    'string s',
    'uint8[3] arr',
    'int16[] seq',
  ].join('\n');

  // a
  pushU32(42);
  // b
  pushF64(Math.PI);
  // s
  pushStr('hello');
  // arr (uint8[3])
  pushU8(1); pushU8(2); pushU8(3);
  // seq length (uint32) + elems
  pushU32(4);
  pushI16(-1); pushI16(2); pushI16(-3); pushI16(4);

  const buf = new Uint8Array(parts).buffer;
  const decoded = decodeRos2Cdr('example/Msg', schema, buf);
  // eslint-disable-next-line no-console
  console.log(decoded);
}
