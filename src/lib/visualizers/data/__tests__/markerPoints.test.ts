import { describe, it, expect } from "vitest";
import { extractCloud } from "../markerPoints";
import type { Value } from "$lib/core/types";

function point(x: number, y: number, z: number): Value {
  return {
    kind: "struct",
    value: {
      x: { kind: "f64", value: x },
      y: { kind: "f64", value: y },
      z: { kind: "f64", value: z },
    },
  };
}

function markerArray(points: Value[], color?: Value): Value {
  const markerFields: Record<string, Value> = { points: { kind: "array", value: points } };
  if (color) markerFields.color = color;
  return {
    kind: "struct",
    value: {
      markers: { kind: "array", value: [{ kind: "struct", value: markerFields }] },
    },
  };
}

describe("extractCloud", () => {
  it("flattens marker points into an xyz buffer", () => {
    const cloud = extractCloud(markerArray([point(1, 2, 3), point(4, 5, 6)]));
    expect(cloud?.positions).toEqual(new Float32Array([1, 2, 3, 4, 5, 6]));
  });

  it("reads the marker colour", () => {
    const color: Value = {
      kind: "struct",
      value: {
        r: { kind: "f64", value: 0.5 },
        g: { kind: "f64", value: 0.25 },
        b: { kind: "f64", value: 1 },
      },
    };
    const cloud = extractCloud(markerArray([point(0, 0, 0)], color));
    expect(cloud?.color).toEqual([0.5, 0.25, 1]);
  });

  it("returns null for a non-markerarray value", () => {
    expect(extractCloud({ kind: "struct", value: { data: { kind: "int", value: 1 } } })).toBeNull();
  });

  it("skips malformed points", () => {
    const bad: Value = { kind: "struct", value: { x: { kind: "f64", value: 1 } } };
    const cloud = extractCloud(markerArray([bad, point(7, 8, 9)]));
    expect(cloud?.positions).toEqual(new Float32Array([7, 8, 9]));
  });
});
