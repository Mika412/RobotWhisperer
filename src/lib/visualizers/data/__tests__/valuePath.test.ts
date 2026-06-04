import { describe, it, expect } from "vitest";
import { parsePath, readValue, readNumber, readNumberArray } from "../valuePath";
import type { Value } from "$lib/core/types";

const sample: Value = {
  kind: "struct",
  value: {
    data: { kind: "int", value: 42 },
    pose: {
      kind: "struct",
      value: {
        position: {
          kind: "struct",
          value: { x: { kind: "f64", value: 1.5 }, y: { kind: "f64", value: -2 } },
        },
      },
    },
    ranges: {
      kind: "array",
      value: [
        { kind: "f32", value: 0.1 },
        { kind: "f32", value: 0.2 },
        { kind: "f32", value: 0.3 },
      ],
    },
    flag: { kind: "bool", value: true },
    name: { kind: "string", value: "hi" },
  },
};

describe("parsePath", () => {
  it("tokenises dotted keys and indices", () => {
    expect(parsePath("pose.position.x")).toEqual([
      { kind: "key", name: "pose" },
      { kind: "key", name: "position" },
      { kind: "key", name: "x" },
    ]);
    expect(parsePath("ranges[2]")).toEqual([
      { kind: "key", name: "ranges" },
      { kind: "index", index: 2 },
    ]);
  });

  it("returns an empty list for an empty path", () => {
    expect(parsePath("")).toEqual([]);
  });
});

describe("readNumber", () => {
  it("reads a top-level int", () => {
    expect(readNumber(sample, parsePath("data"))).toBe(42);
  });

  it("reads a nested float", () => {
    expect(readNumber(sample, parsePath("pose.position.x"))).toBe(1.5);
  });

  it("reads an array element", () => {
    expect(readNumber(sample, parsePath("ranges[1]"))).toBeCloseTo(0.2);
  });

  it("coerces a bool to 0/1", () => {
    expect(readNumber(sample, parsePath("flag"))).toBe(1);
  });

  it("returns undefined for a missing key", () => {
    expect(readNumber(sample, parsePath("pose.position.z"))).toBeUndefined();
  });

  it("returns undefined for a string field", () => {
    expect(readNumber(sample, parsePath("name"))).toBeUndefined();
  });

  it("returns undefined for an out-of-range index", () => {
    expect(readNumber(sample, parsePath("ranges[9]"))).toBeUndefined();
  });
});

describe("readNumberArray", () => {
  it("reads an array of numbers", () => {
    const result = readNumberArray(sample, parsePath("ranges"));
    expect(result).toHaveLength(3);
    expect(result?.[0]).toBeCloseTo(0.1);
  });

  it("returns undefined for a non-array", () => {
    expect(readNumberArray(sample, parsePath("data"))).toBeUndefined();
  });
});

describe("readValue", () => {
  it("returns the struct node for a partial path", () => {
    expect(readValue(sample, parsePath("pose.position"))?.kind).toBe("struct");
  });
});
