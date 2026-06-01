import { SvelteMap } from "svelte/reactivity";
import { getWasmInstance } from "./pipelineRpc";
import type {
  ArrayLength,
  FieldDef,
  FieldType,
  MessageDef,
  ParsedSchema,
  PrimitiveType,
  RequestKind,
  SchemaDefinition,
  SchemaKind,
  SchemaRef,
  SchemaSummary,
  Value,
} from "./types";

async function tauriInvoke<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(name, args);
}

export async function listSchemas(): Promise<SchemaSummary[]> {
  if (!import.meta.env.RW_WEB) {
    return tauriInvoke<SchemaSummary[]>("list_schemas_summary");
  }
  try {
    const wasm = await getWasmInstance();
    return JSON.parse(wasm.listSchemasSummary()) as SchemaSummary[];
  } catch (err) {
    console.warn("[schema] listSchemas failed", err);
    return [];
  }
}

export async function listSchemasByName(name: string): Promise<SchemaDefinition[]> {
  let defs: SchemaDefinition[] = [];
  if (!import.meta.env.RW_WEB) {
    defs = await tauriInvoke<SchemaDefinition[]>("list_schemas_by_name", { name });
  } else {
    try {
      const wasm = await getWasmInstance();
      defs = JSON.parse(wasm.listSchemasByName(name)) as SchemaDefinition[];
    } catch (err) {
      console.warn("[schema] listSchemasByName failed", err);
      defs = [];
    }
  }
  if (defs.length > 0) rememberDefinition(defs[0]);
  return defs;
}

const schemaByHashCache = new Map<string, SchemaDefinition>();
const schemaByNameCache = new Map<string, SchemaDefinition>();

export function getCachedSchemaByHash(hash: string): SchemaDefinition | null {
  return schemaByHashCache.get(hash) ?? null;
}

export function getCachedSchemaByName(name: string): SchemaDefinition | null {
  return schemaByNameCache.get(name) ?? null;
}

function rememberDefinition(def: SchemaDefinition | null): void {
  if (!def) return;
  if (def.hash) schemaByHashCache.set(def.hash, def);
  if (def.name) schemaByNameCache.set(def.name, def);
}

export async function getSchema(hash: string): Promise<SchemaDefinition | null> {
  const cached = schemaByHashCache.get(hash);
  if (cached) return cached;
  let def: SchemaDefinition | null = null;
  if (!import.meta.env.RW_WEB) {
    def = await tauriInvoke<SchemaDefinition | null>("get_schema_by_hash", { hash });
  } else {
    try {
      const wasm = await getWasmInstance();
      const json = wasm.getSchemaByHash(hash);
      def = json ? (JSON.parse(json) as SchemaDefinition) : null;
    } catch (err) {
      console.warn("[schema] getSchema failed", err);
    }
  }
  rememberDefinition(def);
  return def;
}

export async function registerSchema(
  name: string,
  kind: SchemaKind,
  definition: string,
): Promise<SchemaRef> {
  if (!import.meta.env.RW_WEB) {
    return tauriInvoke<SchemaRef>("register_schema", { name, kind, definition });
  }
  const wasm = await getWasmInstance();
  return JSON.parse(await wasm.registerSchema(name, kind, definition)) as SchemaRef;
}

export function filterByRequestKind(
  summaries: SchemaSummary[],
  requestKind: RequestKind,
): SchemaSummary[] {
  const target = matchingSchemaKind(requestKind);
  return summaries.filter((summary) => summary.kind === target);
}

function matchingSchemaKind(requestKind: RequestKind): SchemaKind {
  switch (requestKind) {
    case "topic":
      return "message";
    case "service":
      return "service";
    case "action":
      return "action";
    case "param":
      return "service";
  }
}

export function defaultValueFor(field: FieldType): Value {
  switch (field.kind) {
    case "primitive":
      return primitiveDefault(field.value);
    case "string":
    case "w_string":
      return { kind: "string", value: "" };
    case "array":
      if (field.value.length.kind === "fixed") {
        return {
          kind: "array",
          value: Array.from({ length: field.value.length.value }, () =>
            defaultValueFor(field.value.element),
          ),
        };
      }
      return { kind: "array", value: [] };
    case "complex":
      return { kind: "struct", value: {} };
    case "time":
      return { kind: "time", value: { sec: 0, nanosec: 0 } };
    case "duration":
      return { kind: "duration", value: { sec: 0, nanosec: 0 } };
  }
}

function primitiveDefault(primitive: PrimitiveType): Value {
  switch (primitive) {
    case "bool":
      return { kind: "bool", value: false };
    case "float32":
      return { kind: "f32", value: 0 };
    case "float64":
      return { kind: "f64", value: 0 };
    case "int8":
    case "int16":
    case "int32":
    case "int64":
      return { kind: "int", value: 0 };
    case "byte":
    case "char":
    case "uint8":
    case "uint16":
    case "uint32":
    case "uint64":
      return { kind: "uint", value: 0 };
  }
}

export function primaryMessage(parsed: ParsedSchema): MessageDef {
  switch (parsed.kind) {
    case "message":
      return { fields: parsed.fields, constants: parsed.constants };
    case "service":
      return parsed.request;
    case "action":
      return parsed.goal;
  }
}

const schemaCache = new SvelteMap<string, ParsedSchema | null>();
const pendingResolutions = new Map<string, Promise<ParsedSchema | null>>();

export function resolveSchemaByName(name: string): ParsedSchema | null {
  if (schemaCache.has(name)) {
    return schemaCache.get(name)!;
  }
  if (!pendingResolutions.has(name)) {
    const promise = fetchAndCacheSchema(name);
    pendingResolutions.set(name, promise);
  }
  return null;
}

async function fetchAndCacheSchema(name: string): Promise<ParsedSchema | null> {
  try {
    const definitions = await listSchemasByName(name);
    const messageDefinition = definitions.find((def) => def.parsed?.kind === "message");
    const parsed = (messageDefinition ?? definitions[0])?.parsed ?? null;
    schemaCache.set(name, parsed);
    return parsed;
  } catch {
    schemaCache.set(name, null);
    return null;
  } finally {
    pendingResolutions.delete(name);
  }
}

export async function resolveSchemaByNameAsync(name: string): Promise<ParsedSchema | null> {
  if (schemaCache.has(name)) {
    return schemaCache.get(name)!;
  }
  const pending = pendingResolutions.get(name);
  if (pending) return pending;
  const promise = fetchAndCacheSchema(name);
  pendingResolutions.set(name, promise);
  return promise;
}

export function clearSchemaCache(): void {
  schemaCache.clear();
  pendingResolutions.clear();
  schemaByHashCache.clear();
  schemaByNameCache.clear();
}

export function forgetUnresolvedSchemas(): void {
  for (const name of [...schemaCache.keys()]) {
    if (schemaCache.get(name) === null) schemaCache.delete(name);
  }
}

export function isSchemaResolutionPending(): boolean {
  return pendingResolutions.size > 0;
}

export type { ArrayLength, FieldDef, FieldType, MessageDef, ParsedSchema };
