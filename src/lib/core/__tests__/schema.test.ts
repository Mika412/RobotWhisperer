import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  defaultValueFor,
  filterByRequestKind,
  primaryMessage,
  resolveSchemaByName,
  resolveSchemaByNameAsync,
  clearSchemaCache,
} from "../schema";
import type { FieldType, ParsedSchema, SchemaDefinition, SchemaSummary } from "../types";

describe("defaultValueFor", () => {
  it("seeds primitives at zero / false", () => {
    expect(defaultValueFor({ kind: "primitive", value: "bool" })).toEqual({
      kind: "bool",
      value: false,
    });
    expect(defaultValueFor({ kind: "primitive", value: "uint32" })).toEqual({
      kind: "uint",
      value: 0,
    });
    expect(defaultValueFor({ kind: "primitive", value: "int32" })).toEqual({
      kind: "int",
      value: 0,
    });
    expect(defaultValueFor({ kind: "primitive", value: "float32" })).toEqual({
      kind: "f32",
      value: 0,
    });
  });

  it("seeds strings empty", () => {
    expect(defaultValueFor({ kind: "string", value: { bound: null } })).toEqual({
      kind: "string",
      value: "",
    });
  });

  it("seeds unbounded arrays empty", () => {
    const fieldType: FieldType = {
      kind: "array",
      value: {
        element: { kind: "primitive", value: "uint8" },
        length: { kind: "unbounded" },
      },
    };
    expect(defaultValueFor(fieldType)).toEqual({ kind: "array", value: [] });
  });

  it("seeds fixed arrays with element-default repeated", () => {
    const fieldType: FieldType = {
      kind: "array",
      value: {
        element: { kind: "primitive", value: "float64" },
        length: { kind: "fixed", value: 3 },
      },
    };
    const seeded = defaultValueFor(fieldType);
    expect(seeded.kind).toBe("array");
    if (seeded.kind === "array") {
      expect(seeded.value).toHaveLength(3);
      expect(seeded.value[0]).toEqual({ kind: "f64", value: 0 });
    }
  });

  it("seeds time and duration to zero", () => {
    expect(defaultValueFor({ kind: "time" })).toEqual({
      kind: "time",
      value: { sec: 0, nanosec: 0 },
    });
    expect(defaultValueFor({ kind: "duration" })).toEqual({
      kind: "duration",
      value: { sec: 0, nanosec: 0 },
    });
  });
});

describe("filterByRequestKind", () => {
  const summaries: SchemaSummary[] = [
    { name: "std_msgs/Header", hash: "h1", kind: "message", dependency_count: 1 },
    { name: "example_interfaces/AddTwoInts", hash: "h2", kind: "service", dependency_count: 0 },
    { name: "example_interfaces/Fibonacci", hash: "h3", kind: "action", dependency_count: 0 },
  ];

  it("keeps only message schemas for topic requests", () => {
    expect(filterByRequestKind(summaries, "topic").map((s) => s.name)).toEqual(["std_msgs/Header"]);
  });

  it("keeps only service schemas for service requests", () => {
    expect(filterByRequestKind(summaries, "service").map((s) => s.name)).toEqual([
      "example_interfaces/AddTwoInts",
    ]);
  });

  it("keeps only action schemas for action requests", () => {
    expect(filterByRequestKind(summaries, "action").map((s) => s.name)).toEqual([
      "example_interfaces/Fibonacci",
    ]);
  });
});

describe("primaryMessage", () => {
  it("returns the message body for a topic schema", () => {
    const parsed: ParsedSchema = { kind: "message", fields: [], constants: [] };
    expect(primaryMessage(parsed)).toEqual({ fields: [], constants: [] });
  });

  it("returns the request body for a service schema", () => {
    const parsed: ParsedSchema = {
      kind: "service",
      request: {
        fields: [{ name: "a", field_type: { kind: "primitive", value: "int64" } }],
        constants: [],
      },
      response: { fields: [], constants: [] },
    };
    expect(primaryMessage(parsed).fields[0].name).toBe("a");
  });

  it("returns the goal body for an action schema", () => {
    const parsed: ParsedSchema = {
      kind: "action",
      goal: {
        fields: [{ name: "order", field_type: { kind: "primitive", value: "int32" } }],
        constants: [],
      },
      result: { fields: [], constants: [] },
      feedback: { fields: [], constants: [] },
    };
    expect(primaryMessage(parsed).fields[0].name).toBe("order");
  });
});

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe("resolveSchemaByName", () => {
  beforeEach(() => {
    clearSchemaCache();
    vi.clearAllMocks();
    (window as any).__TAURI_INTERNALS__ = {};
  });

  afterEach(() => {
    delete (window as any).__TAURI_INTERNALS__;
  });

  const mockDefinition: SchemaDefinition = {
    name: "geometry_msgs/Point",
    kind: "message",
    hash: "point-hash",
    definition: "float64 x\nfloat64 y\nfloat64 z",
    parsed: {
      kind: "message",
      fields: [
        { name: "x", field_type: { kind: "primitive", value: "float64" } },
        { name: "y", field_type: { kind: "primitive", value: "float64" } },
        { name: "z", field_type: { kind: "primitive", value: "float64" } },
      ],
      constants: [],
    },
    dependencies: [],
  };

  it("returns null on first call (async fetch pending)", () => {
    mockInvoke.mockResolvedValueOnce([mockDefinition]);
    const result = resolveSchemaByName("geometry_msgs/Point");
    expect(result).toBeNull();
  });

  it("returns cached schema after async resolution", async () => {
    mockInvoke.mockResolvedValueOnce([mockDefinition]);
    const result = await resolveSchemaByNameAsync("geometry_msgs/Point");
    expect(result).not.toBeNull();
    expect(result?.kind).toBe("message");
    if (result?.kind === "message") {
      expect(result.fields).toHaveLength(3);
      expect(result.fields[0].name).toBe("x");
    }
  });

  it("returns from cache synchronously after async resolution", async () => {
    mockInvoke.mockResolvedValueOnce([mockDefinition]);
    await resolveSchemaByNameAsync("geometry_msgs/Point");
    const cached = resolveSchemaByName("geometry_msgs/Point");
    expect(cached).not.toBeNull();
    expect(cached?.kind).toBe("message");
  });

  it("returns null for unknown schema names", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    const result = await resolveSchemaByNameAsync("nonexistent/Type");
    expect(result).toBeNull();
  });

  it("caches null results for unknown schemas", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await resolveSchemaByNameAsync("nonexistent/Type");
    const cached = resolveSchemaByName("nonexistent/Type");
    expect(cached).toBeNull();
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });

  it("does not make duplicate requests for the same name", async () => {
    mockInvoke.mockResolvedValue([mockDefinition]);
    const promise1 = resolveSchemaByNameAsync("geometry_msgs/Point");
    const promise2 = resolveSchemaByNameAsync("geometry_msgs/Point");
    await Promise.all([promise1, promise2]);
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });

  it("clearSchemaCache allows re-fetch", async () => {
    mockInvoke.mockResolvedValue([mockDefinition]);
    await resolveSchemaByNameAsync("geometry_msgs/Point");
    expect(mockInvoke).toHaveBeenCalledTimes(1);
    clearSchemaCache();
    await resolveSchemaByNameAsync("geometry_msgs/Point");
    expect(mockInvoke).toHaveBeenCalledTimes(2);
  });
});
