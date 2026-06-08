import { describe, it, expect } from "vitest";
import { PlotState } from "../plotState";
import { snapshot } from "../plotBuffers";
import type { Value } from "$lib/core/types";

function frame(value: number): Value {
  return { kind: "struct", value: { data: { kind: "f64", value } } };
}

describe("PlotState", () => {
  it("ingests samples for a configured series", () => {
    const state = new PlotState();
    const series = [{ id: "a", path: "data" }];
    state.ingest(series, frame(1));
    state.ingest(series, frame(2));
    const ring = state.ring("a");
    expect(ring).toBeDefined();
    expect(snapshot(ring!, 100, state.nowSeconds()).value).toEqual([1, 2]);
  });

  it("ignores series whose path does not resolve to a number", () => {
    const state = new PlotState();
    state.ingest([{ id: "missing", path: "nope" }], frame(1));
    expect(state.ring("missing")).toBeUndefined();
  });

  it("does nothing for a null value", () => {
    const state = new PlotState();
    state.ingest([{ id: "a", path: "data" }], null);
    expect(state.ring("a")).toBeUndefined();
  });
});
