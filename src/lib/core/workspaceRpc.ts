import { getWasmInstance } from "./pipelineRpc";
import type {
  Collection,
  Connection,
  ImportMode,
  ImportReport,
  NewCollection,
  NewConnection,
  NewRequest,
  Request,
  WorkspaceFile,
} from "./types";

export interface WorkspaceRpc {
  listRequests(): Promise<Request[]>;
  getRequest(id: number): Promise<Request | null>;
  createRequest(draft: NewRequest): Promise<Request>;
  updateRequest(request: Request): Promise<void>;
  deleteRequest(id: number): Promise<void>;

  listCollections(): Promise<Collection[]>;
  createCollection(draft: NewCollection): Promise<Collection>;
  updateCollection(collection: Collection): Promise<void>;
  deleteCollection(id: number): Promise<void>;

  listConnections(): Promise<Connection[]>;
  getConnection(id: number): Promise<Connection | null>;
  createConnection(draft: NewConnection): Promise<Connection>;
  updateConnection(connection: Connection): Promise<void>;
  deleteConnection(id: number): Promise<void>;

  clearWorkspaceStorage(): Promise<void>;

  exportWorkspace(): Promise<string>;
  importWorkspace(file: WorkspaceFile, mode: ImportMode): Promise<ImportReport>;
}

class TauriWorkspaceRpc implements WorkspaceRpc {
  private async invoke<T>(name: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(name, args);
  }
  listRequests() {
    return this.invoke<Request[]>("list_requests");
  }
  getRequest(id: number) {
    return this.invoke<Request | null>("get_request", { id });
  }
  createRequest(draft: NewRequest) {
    return this.invoke<Request>("create_request", { draft });
  }
  updateRequest(request: Request) {
    return this.invoke<void>("update_request", { request }).then(() => undefined);
  }
  deleteRequest(id: number) {
    return this.invoke<void>("delete_request", { id });
  }
  listCollections() {
    return this.invoke<Collection[]>("list_collections");
  }
  createCollection(draft: NewCollection) {
    return this.invoke<Collection>("create_collection", { draft });
  }
  updateCollection(collection: Collection) {
    return this.invoke<void>("update_collection", { collection });
  }
  deleteCollection(id: number) {
    return this.invoke<void>("delete_collection", { id });
  }
  listConnections() {
    return this.invoke<Connection[]>("list_connections");
  }
  getConnection(id: number) {
    return this.invoke<Connection | null>("get_connection", { id });
  }
  createConnection(draft: NewConnection) {
    return this.invoke<Connection>("create_connection", { draft });
  }
  updateConnection(connection: Connection) {
    return this.invoke<void>("update_connection", { connection });
  }
  deleteConnection(id: number) {
    return this.invoke<void>("delete_connection", { id });
  }
  clearWorkspaceStorage() {
    return this.invoke<void>("clear_workspace_storage");
  }
  exportWorkspace() {
    return this.invoke<string>("export_workspace_command");
  }
  importWorkspace(file: WorkspaceFile, mode: ImportMode) {
    return this.invoke<ImportReport>("import_workspace_command", { file, mode });
  }
}

class WasmWorkspaceRpc implements WorkspaceRpc {
  async listRequests(): Promise<Request[]> {
    const wasm = await getWasmInstance();
    return JSON.parse(await wasm.listRequests()) as Request[];
  }
  async getRequest(id: number): Promise<Request | null> {
    const wasm = await getWasmInstance();
    const json = await wasm.getRequest(id);
    return json ? (JSON.parse(json) as Request) : null;
  }
  async createRequest(draft: NewRequest): Promise<Request> {
    const wasm = await getWasmInstance();
    return JSON.parse(await wasm.createRequest(JSON.stringify(draft))) as Request;
  }
  async updateRequest(request: Request): Promise<void> {
    const wasm = await getWasmInstance();
    await wasm.updateRequest(JSON.stringify(request));
  }
  async deleteRequest(id: number): Promise<void> {
    const wasm = await getWasmInstance();
    await wasm.deleteRequest(id);
  }
  async listCollections(): Promise<Collection[]> {
    const wasm = await getWasmInstance();
    return JSON.parse(await wasm.listCollections()) as Collection[];
  }
  async createCollection(draft: NewCollection): Promise<Collection> {
    const wasm = await getWasmInstance();
    return JSON.parse(await wasm.createCollection(JSON.stringify(draft))) as Collection;
  }
  async updateCollection(collection: Collection): Promise<void> {
    const wasm = await getWasmInstance();
    await wasm.updateCollection(JSON.stringify(collection));
  }
  async deleteCollection(id: number): Promise<void> {
    const wasm = await getWasmInstance();
    await wasm.deleteCollection(id);
  }
  async listConnections(): Promise<Connection[]> {
    const wasm = await getWasmInstance();
    return JSON.parse(await wasm.listConnections()) as Connection[];
  }
  async getConnection(id: number): Promise<Connection | null> {
    const wasm = await getWasmInstance();
    const json = await wasm.getConnection(id);
    return json ? (JSON.parse(json) as Connection) : null;
  }
  async createConnection(draft: NewConnection): Promise<Connection> {
    const wasm = await getWasmInstance();
    return JSON.parse(await wasm.createConnection(JSON.stringify(draft))) as Connection;
  }
  async updateConnection(connection: Connection): Promise<void> {
    const wasm = await getWasmInstance();
    await wasm.updateConnection(JSON.stringify(connection));
  }
  async deleteConnection(id: number): Promise<void> {
    const wasm = await getWasmInstance();
    await wasm.deleteConnection(id);
  }
  async clearWorkspaceStorage(): Promise<void> {
    const wasm = await getWasmInstance();
    await wasm.clearWorkspaceStorage();
  }
  async exportWorkspace(): Promise<string> {
    const wasm = await getWasmInstance();
    return wasm.exportWorkspace();
  }
  async importWorkspace(file: WorkspaceFile, mode: ImportMode): Promise<ImportReport> {
    const wasm = await getWasmInstance();
    return JSON.parse(await wasm.importWorkspace(JSON.stringify(file), mode)) as ImportReport;
  }
}

export const workspaceRpc: WorkspaceRpc = import.meta.env.RW_WEB
  ? new WasmWorkspaceRpc()
  : new TauriWorkspaceRpc();
