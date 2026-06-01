import { describe, it, expect } from "vitest";
import { decodeCborPayload } from "../cborDecode";

function hex(s: string): Uint8Array {
  const clean = s.replace(/\s+/g, "");
  const out = new Uint8Array(clean.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

const SAMPLE_STATE_CBOR = hex(`
a2646b696e64667374727563746576616c7565a36666696e676572a2646b696e
646561727261796576616c756581a2646b696e64667374727563746576616c75
65a56a6a6f696e745f6e616d65a2646b696e646561727261796576616c756584
a2646b696e6466737472696e676576616c75656546305f4a30a2646b696e6466
737472696e676576616c75656546305f4a31a2646b696e6466737472696e6765
76616c75656546305f4a32a2646b696e6466737472696e676576616c75656546
305f4a336e6a6f696e745f706f736974696f6ea2646b696e6465617272617965
76616c756584a2646b696e64636636346576616c7565f90000a2646b696e6463
6636346576616c7565fb3fb999999999999aa2646b696e64636636346576616c
7565fb3fc999999999999aa2646b696e64636636346576616c7565fb3fd33333
333333346c74616374696c655f6e616d65a2646b696e646561727261796576616c
756583a2646b696e6466737472696e676576616c756563746970a2646b696e64
66737472696e676576616c756563746f70a2646b696e6466737472696e676576
616c756566626f74746f6d6d6d6f746f725f766f6c74616765a2646b696e6463
6636346576616c7565fb40381f7ced916873646e616d65a2646b696e64667374
72696e676576616c75656246306b66696e6765725f6e616d65a2646b696e6465
61727261796576616c756581a2646b696e6466737472696e676576616c756565
696e64657866686561646572a2646b696e64667374727563746576616c7565a2
686672616d655f6964a2646b696e6466737472696e676576616c75656772685f
70616c6d657374616d70a2646b696e646474696d656576616c7565a263736563
1a6553f100676e616e6f736563193039
`);

const EXPECTED = {
  kind: "struct",
  value: {
    finger: {
      kind: "array",
      value: [
        {
          kind: "struct",
          value: {
            joint_name: {
              kind: "array",
              value: [
                { kind: "string", value: "F0_J0" },
                { kind: "string", value: "F0_J1" },
                { kind: "string", value: "F0_J2" },
                { kind: "string", value: "F0_J3" },
              ],
            },
            joint_position: {
              kind: "array",
              value: [
                { kind: "f64", value: 0.0 },
                { kind: "f64", value: 0.1 },
                { kind: "f64", value: 0.2 },
                { kind: "f64", value: 0.30000000000000004 },
              ],
            },
            tactile_name: {
              kind: "array",
              value: [
                { kind: "string", value: "tip" },
                { kind: "string", value: "top" },
                { kind: "string", value: "bottom" },
              ],
            },
            motor_voltage: { kind: "f64", value: 24.123 },
            name: { kind: "string", value: "F0" },
          },
        },
      ],
    },
    finger_name: { kind: "array", value: [{ kind: "string", value: "index" }] },
    header: {
      kind: "struct",
      value: {
        frame_id: { kind: "string", value: "rh_palm" },
        stamp: { kind: "time", value: { nanosec: 12345, sec: 1700000000 } },
      },
    },
  },
};

describe("ciborium-emitted SampleState roundtrip", () => {
  it("matches ciborium's own JSON-shape output", () => {
    const decoded = decodeCborPayload(SAMPLE_STATE_CBOR);
    expect(decoded).toEqual(EXPECTED);
  });
});
