/* tslint:disable */
/* eslint-disable */

export class WasmRobotWhisperer {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    clearWorkspaceStorage(): Promise<void>;
    static create(): Promise<WasmRobotWhisperer>;
    createCollection(draft_json: string): Promise<string>;
    createConnection(draft_json: string): Promise<string>;
    createRequest(draft_json: string): Promise<string>;
    deleteCollection(id: number): Promise<void>;
    deleteConnection(id: number): Promise<void>;
    deleteRequest(id: number): Promise<void>;
    exportWorkspace(): Promise<string>;
    getConnection(id: number): Promise<any>;
    getRequest(id: number): Promise<any>;
    getSchemaByHash(hash: string): any;
    importWorkspace(file_json: string, mode: string): Promise<string>;
    listCollections(): Promise<string>;
    listConnections(): Promise<string>;
    listRequests(): Promise<string>;
    listSchemasByName(name: string): string;
    listSchemasSummary(): string;
    pipelineCallService(connection_id: string, service: string, request_json: string): Promise<string>;
    pipelineCancelActionGoal(goal_id: string): Promise<void>;
    pipelineClose(connection_id: string): Promise<void>;
    pipelineGetDiscovery(connection_id: string): Promise<string>;
    pipelineOnDiscovery(connection_id: string, on_discovery: Function): Promise<void>;
    pipelineOnStatus(connection_id: string, on_status: Function): Promise<void>;
    pipelineOpenDummy(): Promise<string>;
    pipelineOpenFoxglove(url: string): Promise<string>;
    pipelineOpenRosbridge(url: string): Promise<string>;
    pipelineSendActionGoal(connection_id: string, action: string, goal_json: string, on_envelope: Function): Promise<string>;
    pipelineSubscribeTopic(connection_id: string, topic: string, on_frame: Function, options_json?: string | null): Promise<any>;
    pipelineUnsubscribe(subscription_id: string): Promise<void>;
    registerSchema(name: string, kind: string, definition: string): Promise<string>;
    setPerfTraceEnabled(enabled: boolean): void;
    updateCollection(collection_json: string): Promise<void>;
    updateConnection(connection_json: string): Promise<void>;
    updateRequest(request_json: string): Promise<void>;
}

export function __init(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __init: () => void;
    readonly __wbg_wasmrobotwhisperer_free: (a: number, b: number) => void;
    readonly wasmrobotwhisperer_clearWorkspaceStorage: (a: number) => any;
    readonly wasmrobotwhisperer_create: () => any;
    readonly wasmrobotwhisperer_createCollection: (a: number, b: number, c: number) => any;
    readonly wasmrobotwhisperer_createConnection: (a: number, b: number, c: number) => any;
    readonly wasmrobotwhisperer_createRequest: (a: number, b: number, c: number) => any;
    readonly wasmrobotwhisperer_deleteCollection: (a: number, b: number) => any;
    readonly wasmrobotwhisperer_deleteConnection: (a: number, b: number) => any;
    readonly wasmrobotwhisperer_deleteRequest: (a: number, b: number) => any;
    readonly wasmrobotwhisperer_exportWorkspace: (a: number) => any;
    readonly wasmrobotwhisperer_getConnection: (a: number, b: number) => any;
    readonly wasmrobotwhisperer_getRequest: (a: number, b: number) => any;
    readonly wasmrobotwhisperer_getSchemaByHash: (a: number, b: number, c: number) => [number, number, number];
    readonly wasmrobotwhisperer_importWorkspace: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wasmrobotwhisperer_listCollections: (a: number) => any;
    readonly wasmrobotwhisperer_listConnections: (a: number) => any;
    readonly wasmrobotwhisperer_listRequests: (a: number) => any;
    readonly wasmrobotwhisperer_listSchemasByName: (a: number, b: number, c: number) => [number, number, number, number];
    readonly wasmrobotwhisperer_listSchemasSummary: (a: number) => [number, number, number, number];
    readonly wasmrobotwhisperer_pipelineCallService: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => any;
    readonly wasmrobotwhisperer_pipelineCancelActionGoal: (a: number, b: number, c: number) => any;
    readonly wasmrobotwhisperer_pipelineClose: (a: number, b: number, c: number) => any;
    readonly wasmrobotwhisperer_pipelineGetDiscovery: (a: number, b: number, c: number) => any;
    readonly wasmrobotwhisperer_pipelineOnDiscovery: (a: number, b: number, c: number, d: any) => any;
    readonly wasmrobotwhisperer_pipelineOnStatus: (a: number, b: number, c: number, d: any) => any;
    readonly wasmrobotwhisperer_pipelineOpenDummy: (a: number) => any;
    readonly wasmrobotwhisperer_pipelineOpenFoxglove: (a: number, b: number, c: number) => any;
    readonly wasmrobotwhisperer_pipelineOpenRosbridge: (a: number, b: number, c: number) => any;
    readonly wasmrobotwhisperer_pipelineSendActionGoal: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: any) => any;
    readonly wasmrobotwhisperer_pipelineSubscribeTopic: (a: number, b: number, c: number, d: number, e: number, f: any, g: number, h: number) => any;
    readonly wasmrobotwhisperer_pipelineUnsubscribe: (a: number, b: number, c: number) => any;
    readonly wasmrobotwhisperer_registerSchema: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => any;
    readonly wasmrobotwhisperer_setPerfTraceEnabled: (a: number, b: number) => void;
    readonly wasmrobotwhisperer_updateCollection: (a: number, b: number, c: number) => any;
    readonly wasmrobotwhisperer_updateConnection: (a: number, b: number, c: number) => any;
    readonly wasmrobotwhisperer_updateRequest: (a: number, b: number, c: number) => any;
    readonly wasm_bindgen__convert__closures_____invoke__h044df13664a5118e: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h6a9fbbf6e574e3a9: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h93f54dc8dceeeee7: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__hedb93e4390e90aaf: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h31031c695d9cb0e0: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h92d34af41c2735a1: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h279f1b630bab216e: (a: number, b: number) => number;
    readonly wasm_bindgen__convert__closures_____invoke__hbb05bbc55e53b8b9: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
