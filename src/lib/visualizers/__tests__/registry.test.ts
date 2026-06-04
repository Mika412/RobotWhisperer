import { describe, it, expect, beforeAll } from "vitest";
import {
  registerVisualizer,
  getVisualizer,
  listVisualizers,
  matchingVisualizers,
} from "../registry";
import type { Component } from "svelte";
import type { VisualizerProps } from "../types";

const FRAME_VALUE = 1;
const FRAME_IMAGE = 2;

const stub = null as unknown as Component<VisualizerProps>;

beforeAll(() => {
  registerVisualizer({
    id: "image",
    displayName: "Image",
    defaultConfig: {},
    component: stub,
    accepts: (schema, kind) =>
      kind === FRAME_IMAGE
        ? 100
        : /^(Compressed|Raw)?Image$/.test(schema.split(/[/.]/).pop() ?? "")
          ? 90
          : 0,
  });
  registerVisualizer({
    id: "cloud",
    displayName: "Cloud",
    defaultConfig: {},
    component: stub,
    accepts: (schema) => (/MarkerArray$/.test(schema) ? 100 : 0),
  });
  registerVisualizer({
    id: "hand",
    displayName: "Hand",
    defaultConfig: {},
    component: stub,
    accepts: (schema) => (/SampleState$/.test(schema) ? 100 : 0),
  });
  registerVisualizer({
    id: "fingertip",
    displayName: "Fingertip",
    defaultConfig: {},
    component: stub,
    accepts: (schema) => (/SampleState$/.test(schema) ? 90 : 0),
  });
});

describe("visualizer registry", () => {
  it("registers and looks up by id", () => {
    expect(getVisualizer("image")?.displayName).toBe("Image");
    expect(getVisualizer("missing")).toBeUndefined();
    expect(listVisualizers().length).toBe(4);
  });

  it("matches the right visualizer for a schema", () => {
    expect(matchingVisualizers("sensor_msgs/msg/Image", FRAME_IMAGE).map((v) => v.id)).toEqual([
      "image",
    ]);
    expect(
      matchingVisualizers("visualization_msgs/msg/MarkerArray", FRAME_VALUE).map((v) => v.id),
    ).toEqual(["cloud"]);
  });

  it("returns nothing for an unsupported schema (Visualize tab stays disabled)", () => {
    expect(matchingVisualizers("std_msgs/msg/Int64", FRAME_VALUE)).toEqual([]);
  });

  it("returns multiple matches highest-score-first (the viz sub-switcher)", () => {
    expect(
      matchingVisualizers("example_msgs/msg/SampleState", FRAME_VALUE).map((v) => v.id),
    ).toEqual(["hand", "fingertip"]);
  });
});
