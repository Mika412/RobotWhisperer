import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const getSchemaMock = vi.fn();

vi.mock("$lib/core/schema", () => ({
  getSchema: (hash: string) => getSchemaMock(hash),
}));

import {
  clearSchemaNameCache,
  getCachedSchemaName,
  isSchemaNameResolved,
  resolveSchemaName,
} from "$lib/core/schemaNames";

describe("schemaNames cache", () => {
  beforeEach(() => {
    clearSchemaNameCache();
    getSchemaMock.mockReset();
  });

  afterEach(() => {
    clearSchemaNameCache();
  });

  it("caches a successful name resolution", async () => {
    getSchemaMock.mockResolvedValueOnce({
      name: "visualization_msgs/MarkerArray",
      hash: "abc",
    });
    const name = await resolveSchemaName("abc");
    expect(name).toBe("visualization_msgs/MarkerArray");
    expect(isSchemaNameResolved("abc")).toBe(true);
    expect(getCachedSchemaName("abc")).toBe("visualization_msgs/MarkerArray");
    const second = await resolveSchemaName("abc");
    expect(second).toBe("visualization_msgs/MarkerArray");
    expect(getSchemaMock).toHaveBeenCalledTimes(1);
  });

  it("negative-caches a missing schema so we don't keep re-issuing RPCs", async () => {
    getSchemaMock.mockResolvedValueOnce(null);
    const name = await resolveSchemaName("missing");
    expect(name).toBeNull();
    expect(isSchemaNameResolved("missing")).toBe(true);
    expect(getCachedSchemaName("missing")).toBeNull();
    await resolveSchemaName("missing");
    expect(getSchemaMock).toHaveBeenCalledTimes(1);
  });

  it("dedupes concurrent in-flight resolutions for the same hash", async () => {
    let resolveOnce: (value: { name: string; hash: string } | null) => void = () => {};
    getSchemaMock.mockImplementationOnce(() => new Promise((res) => (resolveOnce = res)));
    const a = resolveSchemaName("dedupe");
    const b = resolveSchemaName("dedupe");
    resolveOnce({ name: "std_msgs/Header", hash: "dedupe" });
    expect(await a).toBe("std_msgs/Header");
    expect(await b).toBe("std_msgs/Header");
    expect(getSchemaMock).toHaveBeenCalledTimes(1);
  });

  it("a different hash still resolves (per-hash, not global lockout)", async () => {
    getSchemaMock.mockResolvedValueOnce(null);
    await resolveSchemaName("hash_a");
    getSchemaMock.mockResolvedValueOnce({
      name: "visualization_msgs/MarkerArray",
      hash: "hash_b",
    });
    const second = await resolveSchemaName("hash_b");
    expect(second).toBe("visualization_msgs/MarkerArray");
    expect(getSchemaMock).toHaveBeenCalledTimes(2);
  });
});
