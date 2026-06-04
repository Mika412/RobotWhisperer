import { describe, it, expect } from "vitest";
import { createRing, pushSample, snapshot, earliestTime, computeRange } from "../plotBuffers";

describe("ring buffer", () => {
  it("collects samples within the window", () => {
    const ring = createRing(8);
    for (let i = 0; i < 5; i += 1) pushSample(ring, i, i * 10);
    const slice = snapshot(ring, 10, 4);
    expect(slice.time).toEqual([0, 1, 2, 3, 4]);
    expect(slice.value).toEqual([0, 10, 20, 30, 40]);
  });

  it("drops samples older than the window", () => {
    const ring = createRing(8);
    for (let i = 0; i < 5; i += 1) pushSample(ring, i, i);
    const slice = snapshot(ring, 2, 4);
    expect(slice.time).toEqual([2, 3, 4]);
  });

  it("wraps and keeps only the most recent capacity samples", () => {
    const ring = createRing(4);
    for (let i = 0; i < 10; i += 1) pushSample(ring, i, i);
    const slice = snapshot(ring, 100, 9);
    expect(slice.time).toEqual([6, 7, 8, 9]);
  });

  it("reports the earliest retained time after wrapping", () => {
    const ring = createRing(4);
    for (let i = 0; i < 10; i += 1) pushSample(ring, i, i);
    expect(earliestTime(ring)).toBe(6);
  });

  it("returns null earliest time when empty", () => {
    expect(earliestTime(createRing(4))).toBeNull();
  });
});

describe("computeRange", () => {
  it("pads a normal range by 5%", () => {
    expect(computeRange([[0, 10]])).toEqual({ min: -0.5, max: 10.5 });
  });

  it("pads a degenerate range symmetrically", () => {
    expect(computeRange([[5, 5]])).toEqual({ min: 5 - 0.25, max: 5 + 0.25 });
  });

  it("ignores non-finite values", () => {
    expect(computeRange([[NaN, 2, 4]])).toEqual({ min: 1.9, max: 4.1 });
  });

  it("returns null for no finite data", () => {
    expect(computeRange([[], [NaN]])).toBeNull();
  });
});
