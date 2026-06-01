import { describe, it, expect } from "vitest";
import type { Discovery, RequestKind, SchemaRef } from "../types";

interface MockDiscoveryEntry {
  name: string;
  schema: SchemaRef;
  connectionId: number;
}

function getSuggestionsByKind(
  discoveries: Map<number, Discovery>,
  kind: RequestKind,
  connectionId?: number | null,
): MockDiscoveryEntry[] {
  if (connectionId != null) {
    const discovery = discoveries.get(connectionId);
    if (!discovery) return [];
    const items =
      kind === "topic"
        ? discovery.topics
        : kind === "service"
          ? discovery.services
          : discovery.actions;
    return items.map((item) => ({
      name: item.name,
      schema: item.schema,
      connectionId,
    }));
  }
  const result: MockDiscoveryEntry[] = [];
  for (const [connId, discovery] of discoveries) {
    const items =
      kind === "topic"
        ? discovery.topics
        : kind === "service"
          ? discovery.services
          : discovery.actions;
    for (const item of items) {
      result.push({ name: item.name, schema: item.schema, connectionId: connId });
    }
  }
  return result;
}

function filterSuggestions(suggestions: MockDiscoveryEntry[], query: string): MockDiscoveryEntry[] {
  if (!query.trim()) return suggestions;
  const lower = query.toLowerCase();
  return suggestions.filter((s) => s.name.toLowerCase().includes(lower));
}

describe("discovery suggestion filtering", () => {
  const discovery1: Discovery = {
    topics: [
      { name: "/scan", schema: { name: "sensor_msgs/LaserScan", hash: "abc" } },
      { name: "/odom", schema: { name: "nav_msgs/Odometry", hash: "def" } },
      { name: "/cmd_vel", schema: { name: "geometry_msgs/Twist", hash: "ghi" } },
    ],
    services: [
      { name: "/add_two_ints", schema: { name: "example_interfaces/AddTwoInts", hash: "srv1" } },
    ],
    actions: [
      { name: "/fibonacci", schema: { name: "example_interfaces/Fibonacci", hash: "act1" } },
    ],
  };

  const discovery2: Discovery = {
    topics: [
      { name: "/scan", schema: { name: "sensor_msgs/LaserScan", hash: "abc" } },
      { name: "/camera/image_raw", schema: { name: "sensor_msgs/Image", hash: "img1" } },
    ],
    services: [],
    actions: [],
  };

  const discoveries = new Map<number, Discovery>([
    [1, discovery1],
    [2, discovery2],
  ]);

  it("returns topics for bound connection", () => {
    const results = getSuggestionsByKind(discoveries, "topic", 1);
    expect(results).toHaveLength(3);
    expect(results.map((r) => r.name)).toEqual(["/scan", "/odom", "/cmd_vel"]);
    expect(results.every((r) => r.connectionId === 1)).toBe(true);
  });

  it("returns services for bound connection", () => {
    const results = getSuggestionsByKind(discoveries, "service", 1);
    expect(results).toHaveLength(1);
    expect(results[0].name).toBe("/add_two_ints");
  });

  it("returns actions for bound connection", () => {
    const results = getSuggestionsByKind(discoveries, "action", 1);
    expect(results).toHaveLength(1);
    expect(results[0].name).toBe("/fibonacci");
  });

  it("returns empty for connection with no matching kind", () => {
    const results = getSuggestionsByKind(discoveries, "service", 2);
    expect(results).toHaveLength(0);
  });

  it("returns empty for unknown connection", () => {
    const results = getSuggestionsByKind(discoveries, "topic", 999);
    expect(results).toHaveLength(0);
  });

  it("merges topics across all connections when unbound", () => {
    const results = getSuggestionsByKind(discoveries, "topic", null);
    expect(results).toHaveLength(5);
    const names = results.map((r) => r.name);
    expect(names).toContain("/scan");
    expect(names).toContain("/odom");
    expect(names).toContain("/cmd_vel");
    expect(names).toContain("/camera/image_raw");
  });

  it("merged results include source connection id", () => {
    const results = getSuggestionsByKind(discoveries, "topic", null);
    const cameraResult = results.find((r) => r.name === "/camera/image_raw");
    expect(cameraResult?.connectionId).toBe(2);
  });

  it("includes duplicate topics from different connections", () => {
    const results = getSuggestionsByKind(discoveries, "topic", null);
    const scanResults = results.filter((r) => r.name === "/scan");
    expect(scanResults).toHaveLength(2);
    expect(scanResults.map((r) => r.connectionId).sort()).toEqual([1, 2]);
  });

  it("merges services across all connections when unbound", () => {
    const results = getSuggestionsByKind(discoveries, "service", null);
    expect(results).toHaveLength(1);
    expect(results[0].name).toBe("/add_two_ints");
    expect(results[0].connectionId).toBe(1);
  });

  it("merges actions across all connections when unbound", () => {
    const results = getSuggestionsByKind(discoveries, "action", null);
    expect(results).toHaveLength(1);
    expect(results[0].name).toBe("/fibonacci");
  });
});

describe("suggestion text filtering", () => {
  const suggestions: MockDiscoveryEntry[] = [
    { name: "/scan", schema: { name: "sensor_msgs/LaserScan", hash: "a" }, connectionId: 1 },
    { name: "/odom", schema: { name: "nav_msgs/Odometry", hash: "b" }, connectionId: 1 },
    {
      name: "/camera/image_raw",
      schema: { name: "sensor_msgs/Image", hash: "c" },
      connectionId: 2,
    },
    { name: "/cmd_vel", schema: { name: "geometry_msgs/Twist", hash: "d" }, connectionId: 1 },
  ];

  it("returns all when query is empty", () => {
    expect(filterSuggestions(suggestions, "")).toHaveLength(4);
    expect(filterSuggestions(suggestions, "   ")).toHaveLength(4);
  });

  it("filters by substring match case-insensitively", () => {
    const results = filterSuggestions(suggestions, "cam");
    expect(results).toHaveLength(1);
    expect(results[0].name).toBe("/camera/image_raw");
  });

  it("matches partial topic names", () => {
    const results = filterSuggestions(suggestions, "/c");
    expect(results).toHaveLength(2);
    expect(results.map((r) => r.name).sort()).toEqual(["/camera/image_raw", "/cmd_vel"]);
  });

  it("returns empty when nothing matches", () => {
    expect(filterSuggestions(suggestions, "nonexistent")).toHaveLength(0);
  });

  it("handles case-insensitive matching", () => {
    const results = filterSuggestions(suggestions, "SCAN");
    expect(results).toHaveLength(1);
    expect(results[0].name).toBe("/scan");
  });
});
