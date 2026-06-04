import { describe, it, expect } from "vitest";
import { buildValueTree, shapeSignature } from "../valueFieldTree";
import type { Value } from "$lib/core/types";

function f64(value: number): Value {
  return { kind: "f64", value };
}

const sample: Value = {
  kind: "struct",
  value: {
    header: { kind: "struct", value: { seq: { kind: "uint", value: 3 } } },
    name: { kind: "string", value: "left" },
    forces: { kind: "array", value: [f64(1), f64(2), f64(3)] },
    ok: { kind: "bool", value: true },
  },
};

describe("buildValueTree", () => {
  const tree = buildValueTree(sample);

  it("lists top-level struct fields with bare paths", () => {
    expect(tree.map((node) => node.path)).toEqual(["header", "name", "forces", "ok"]);
  });

  it("marks numeric and bool leaves plottable, strings not", () => {
    const byPath = Object.fromEntries(tree.map((node) => [node.path, node]));
    expect(byPath.name.plottable).toBe(false);
    expect(byPath.ok.plottable).toBe(true);
  });

  it("expands a struct child with a dotted path", () => {
    const header = tree.find((node) => node.path === "header")!;
    expect(header.children?.[0].path).toBe("header.seq");
    expect(header.children?.[0].plottable).toBe(true);
  });

  it("expands an array to its real length with indexed paths", () => {
    const forces = tree.find((node) => node.path === "forces")!;
    expect(forces.children?.map((child) => child.path)).toEqual([
      "forces[0]",
      "forces[1]",
      "forces[2]",
    ]);
    expect(forces.typeLabel).toBe("f64[3]");
    expect(forces.children?.every((child) => child.plottable)).toBe(true);
  });

  it("returns an empty tree for non-object input", () => {
    expect(buildValueTree(null)).toEqual([]);
    expect(buildValueTree(42)).toEqual([]);
  });
});

describe("shapeSignature", () => {
  it("is stable when only numeric values change", () => {
    const a = shapeSignature(sample);
    const changed: Value = {
      kind: "struct",
      value: { ...sample.value, forces: { kind: "array", value: [f64(9), f64(9), f64(9)] } },
    };
    expect(shapeSignature(changed)).toBe(a);
  });

  it("changes when an array length changes", () => {
    const longer: Value = {
      kind: "struct",
      value: { ...sample.value, forces: { kind: "array", value: [f64(1), f64(2)] } },
    };
    expect(shapeSignature(longer)).not.toBe(shapeSignature(sample));
  });

  it("changes when a key is added", () => {
    const extra: Value = {
      kind: "struct",
      value: { ...sample.value, extra: f64(0) },
    };
    expect(shapeSignature(extra)).not.toBe(shapeSignature(sample));
  });
});
