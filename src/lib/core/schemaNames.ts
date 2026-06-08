import { getSchema } from "./schema";

const resolved = new Map<string, string | null>();
const pending = new Map<string, Promise<string | null>>();

export function getCachedSchemaName(hash: string): string | null {
  return resolved.get(hash) ?? null;
}

export function isSchemaNameResolved(hash: string): boolean {
  return resolved.has(hash);
}

export function resolveSchemaName(hash: string): Promise<string | null> {
  if (resolved.has(hash)) return Promise.resolve(resolved.get(hash)!);
  const inflight = pending.get(hash);
  if (inflight) return inflight;

  const lookup = (async () => {
    try {
      const definition = await getSchema(hash);
      const name = definition?.name ?? null;
      resolved.set(hash, name);
      return name;
    } catch {
      resolved.set(hash, null);
      return null;
    } finally {
      pending.delete(hash);
    }
  })();
  pending.set(hash, lookup);
  return lookup;
}

export function clearSchemaNameCache(): void {
  resolved.clear();
  pending.clear();
}
