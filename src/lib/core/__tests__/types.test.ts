import { describe, it, expect } from "vitest";
import type { RequestKind, TransportKind, Value } from "../types";
import { WORKSPACE_FORMAT, WORKSPACE_VERSION, valuePreviewText } from "../types";

describe("domain wire-format constants", () => {
  it("RequestKind literals match the Rust serde representation", () => {
    const topic: RequestKind = "topic";
    const service: RequestKind = "service";
    const action: RequestKind = "action";

    expect(topic).toBe("topic");
    expect(service).toBe("service");
    expect(action).toBe("action");
  });

  it("TransportKind literals match the Rust serde representation", () => {
    const foxglove: TransportKind = "foxglove_ws";
    const rosbridge: TransportKind = "rosbridge";
    const native: TransportKind = "native_ros2";

    expect(foxglove).toBe("foxglove_ws");
    expect(rosbridge).toBe("rosbridge");
    expect(native).toBe("native_ros2");
  });

  it("workspace format constants match Rust", () => {
    expect(WORKSPACE_FORMAT).toBe("robot-whisperer/workspace");
    expect(WORKSPACE_VERSION).toBe(1);
  });
});

describe("valuePreviewText", () => {
  it("renders small values in full", () => {
    const value: Value = {
      kind: "struct",
      value: { a: { kind: "int", value: 1 }, b: { kind: "string", value: "hi" } },
    };
    expect(JSON.parse(valuePreviewText(value))).toEqual({ a: 1, b: "hi" });
  });

  it("caps a large array and notes how many were dropped", () => {
    const value: Value = {
      kind: "array",
      value: Array.from(
        { length: 50_000 },
        (_, index) => ({ kind: "uint", value: index }) as Value,
      ),
    };
    const parsed = JSON.parse(valuePreviewText(value)) as unknown[];
    expect(parsed.length).toBe(201);
    expect(parsed[200]).toBe("…(49800 more)");
  });

  it("caps a large byte field instead of materialising it whole", () => {
    const value: Value = {
      kind: "struct",
      value: { data: { kind: "bytes", value: Array.from({ length: 100_000 }, () => 7) } },
    };
    const parsed = JSON.parse(valuePreviewText(value)) as { data: unknown[] };
    expect(parsed.data.length).toBe(201);
    expect(parsed.data[200]).toBe("…(99800 more)");
  });
});
