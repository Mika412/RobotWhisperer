export type RequestId = number;
export type CollectionId = number;
export type ConnectionId = number;
export type SessionId = string;

export type RequestKind = "topic" | "service" | "action" | "param";
export type TransportKind = "foxglove_ws" | "rosbridge" | "native_ros2" | "dummy";

export interface SchemaRef {
  name: string;
  hash: string;
}

export type Value =
  | { kind: "null" }
  | { kind: "bool"; value: boolean }
  | { kind: "int"; value: number }
  | { kind: "uint"; value: number }
  | { kind: "f32"; value: number }
  | { kind: "f64"; value: number }
  | { kind: "string"; value: string }
  | { kind: "bytes"; value: number[] }
  | { kind: "array"; value: Value[] }
  | { kind: "struct"; value: Record<string, Value> }
  | { kind: "time"; value: { sec: number; nanosec: number } }
  | { kind: "duration"; value: { sec: number; nanosec: number } };

export const NULL_VALUE: Value = { kind: "null" };
export const EMPTY_STRUCT: Value = { kind: "struct", value: {} };

export type ResultTab = "raw" | "visualize" | "plot";

export interface RequestView {
  tab: ResultTab;
  visualizerId: string | null;
  configs: Record<string, Record<string, unknown>>;
}

export interface Request {
  id: RequestId;
  collection_id: CollectionId | null;
  connection_id: ConnectionId | null;
  name: string;
  kind: RequestKind;
  target: string;
  schema: SchemaRef | null;
  input: Value;
  visualization?: RequestView | null;
  created_at: string;
  updated_at: string;
}

export interface Collection {
  id: CollectionId;
  parent_id: CollectionId | null;
  name: string;
  created_at: string;
}

export type TransportConfig =
  | { kind: "foxglove_ws"; url: string; headers: [string, string][] }
  | { kind: "rosbridge"; url: string }
  | { kind: "native_ros2"; domain_id: number }
  | { kind: "dummy" };

export interface Connection {
  id: ConnectionId;
  name: string;
  config: TransportConfig;
  auto_connect: boolean;
  created_at: string;
  updated_at: string;
}

export type SchemaKind = "message" | "service" | "action";

export interface SchemaSummary {
  name: string;
  hash: string;
  kind: SchemaKind;
  dependency_count: number;
}

export type PrimitiveType =
  | "bool"
  | "byte"
  | "char"
  | "int8"
  | "uint8"
  | "int16"
  | "uint16"
  | "int32"
  | "uint32"
  | "int64"
  | "uint64"
  | "float32"
  | "float64";

export type FieldType =
  | { kind: "primitive"; value: PrimitiveType }
  | { kind: "string"; value: { bound: number | null } }
  | { kind: "w_string"; value: { bound: number | null } }
  | { kind: "array"; value: { element: FieldType; length: ArrayLength } }
  | { kind: "complex"; value: { type_name: string } }
  | { kind: "time" }
  | { kind: "duration" };

export type ArrayLength =
  | { kind: "unbounded" }
  | { kind: "bounded"; value: number }
  | { kind: "fixed"; value: number };

export interface FieldDef {
  name: string;
  field_type: FieldType;
  default?: Value | null;
  comment?: string | null;
}

export interface ConstantDef {
  name: string;
  field_type: FieldType;
  value: Value;
}

export interface MessageDef {
  fields: FieldDef[];
  constants: ConstantDef[];
}

export type ParsedSchema =
  | { kind: "message"; fields: FieldDef[]; constants: ConstantDef[] }
  | { kind: "service"; request: MessageDef; response: MessageDef }
  | { kind: "action"; goal: MessageDef; result: MessageDef; feedback: MessageDef };

export interface SchemaDefinition {
  name: string;
  kind: SchemaKind;
  hash: string;
  definition: string;
  parsed: ParsedSchema;
  dependencies: string[];
}

export interface NewRequest {
  collection_id: CollectionId | null;
  connection_id: ConnectionId | null;
  name: string;
  kind: RequestKind;
  target: string;
  schema: SchemaRef | null;
  input: Value;
  visualization?: RequestView | null;
}

export interface NewCollection {
  parent_id: CollectionId | null;
  name: string;
}

export interface NewConnection {
  name: string;
  config: TransportConfig;
  auto_connect: boolean;
}

export type ImportMode = "replace" | "merge";

export interface ImportConflict {
  entity: "connection" | "collection" | "request" | "schema";
  name: string;
  reason: string;
}

export interface ImportReport {
  connections_added: number;
  connections_skipped: number;
  collections_added: number;
  requests_added: number;
  schemas_added: number;
  conflicts: ImportConflict[];
}

export interface Workspace {
  connections: Connection[];
  collections: Collection[];
  requests: Request[];
  schemas: SchemaDefinition[];
}

export type LayoutBadge = "live" | "sim" | null;

export interface Layout {
  id: number;
  name: string;
  badge: LayoutBadge;
  treeId: string;
  created_at: string;
  updated_at: string;
}

export interface WorkspaceFile extends Workspace {
  format: string;
  version: number;
  exported_at: string;
}

export const WORKSPACE_FORMAT = "robot-whisperer/workspace";
export const WORKSPACE_VERSION = 1;

export type TransportStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "reconnecting"
  | "failed";

export interface SessionInfo {
  connection_id: ConnectionId;
  session_id: string;
  status: TransportStatus;
  last_error: string | null;
}

export interface SessionStatusEvent {
  connection_id: ConnectionId;
  session_id: string;
  status: TransportStatus;
  error: string | null;
}

export interface TopicInfo {
  name: string;
  schema: SchemaRef;
}

export interface ServiceInfo {
  name: string;
  schema: SchemaRef;
}

export interface ActionInfo {
  name: string;
  schema: SchemaRef;
}

export interface Discovery {
  topics: TopicInfo[];
  services: ServiceInfo[];
  actions: ActionInfo[];
}

export interface SessionDiscoveryEvent {
  connection_id: ConnectionId;
  session_id: string;
  discovery: Discovery;
}

export interface SubscribeResponse {
  handle: string;
}

export interface SubscriptionLifecycleEvent {
  connection_id: ConnectionId;
  session_id: string;
}

export type PlainValue =
  | number
  | boolean
  | string
  | Uint8Array
  | null
  | { sec: number; nanosec: number }
  | PlainValue[]
  | { [key: string]: PlainValue };
export interface PlainStruct {
  [key: string]: PlainValue;
}

export function valueToPlain(value: Value | null | undefined): PlainValue {
  if (!value) return null;
  switch (value.kind) {
    case "null":
      return null;
    case "bool":
      return value.value;
    case "int":
      return value.value;
    case "uint":
      return value.value;
    case "f32":
      return value.value;
    case "f64":
      return value.value;
    case "string":
      return value.value;
    case "bytes":
      return new Uint8Array(value.value);
    case "array":
      return value.value.map(valueToPlain);
    case "struct": {
      const result: PlainStruct = {};
      for (const [key, val] of Object.entries(value.value)) {
        result[key] = valueToPlain(val);
      }
      return result;
    }
    case "time":
      return value.value;
    case "duration":
      return value.value;
  }
}

const PREVIEW_MAX_ARRAY_ITEMS = 200;
const PREVIEW_NODE_BUDGET = 20_000;

export function valuePreviewText(value: Value | null | undefined): string {
  let budget = PREVIEW_NODE_BUDGET;

  function walk(node: Value | null | undefined): PlainValue {
    if (!node) return null;
    switch (node.kind) {
      case "array": {
        const limit = Math.min(node.value.length, PREVIEW_MAX_ARRAY_ITEMS);
        const out: PlainValue[] = [];
        for (let index = 0; index < limit && budget > 0; index += 1) {
          budget -= 1;
          out.push(walk(node.value[index]));
        }
        if (node.value.length > limit) out.push(`…(${node.value.length - limit} more)`);
        return out;
      }
      case "bytes": {
        const limit = Math.min(node.value.length, PREVIEW_MAX_ARRAY_ITEMS);
        budget -= limit;
        const head: PlainValue[] = node.value.slice(0, limit);
        if (node.value.length > limit) head.push(`…(${node.value.length - limit} more)`);
        return head;
      }
      case "struct": {
        const result: PlainStruct = {};
        for (const [key, child] of Object.entries(node.value)) {
          if (budget <= 0) {
            result["…"] = "(truncated)";
            break;
          }
          result[key] = walk(child);
        }
        return result;
      }
      default:
        return valueToPlain(node);
    }
  }

  return JSON.stringify(walk(value), null, 2);
}
