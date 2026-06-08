import { workspaceRpc } from "$lib/core/workspaceRpc";
import {
  EMPTY_STRUCT,
  type Collection,
  type NewRequest,
  type Request,
  type RequestView,
} from "$lib/core/types";

class RequestsStore {
  requests = $state<Request[]>([]);
  collections = $state<Collection[]>([]);
  initialized = $state(false);
  private viewTimers = new Map<number, ReturnType<typeof setTimeout>>();

  async load(): Promise<void> {
    try {
      const [requests, collections] = await Promise.all([
        workspaceRpc.listRequests(),
        workspaceRpc.listCollections(),
      ]);
      this.requests = requests;
      this.collections = collections;
    } catch (error) {
      console.error("[workspace] failed to load requests/collections", error);
    } finally {
      this.initialized = true;
    }
  }

  get(id: number): Request | undefined {
    return this.requests.find((request) => request.id === id);
  }

  async create(overrides: Partial<NewRequest> = {}): Promise<Request> {
    const draft: NewRequest = {
      collection_id: null,
      connection_id: null,
      name: "New request",
      kind: "topic",
      target: "",
      schema: null,
      input: EMPTY_STRUCT,
      ...overrides,
    };
    const request = await workspaceRpc.createRequest(draft);
    this.requests = [...this.requests, request];
    return request;
  }

  async update(request: Request): Promise<void> {
    await workspaceRpc.updateRequest(request);
    this.requests = this.requests.map((existing) =>
      existing.id === request.id ? request : existing,
    );
  }

  setVisualization(id: number, view: RequestView): void {
    const existing = this.get(id);
    if (!existing) return;
    this.requests = this.requests.map((request) =>
      request.id === id ? { ...request, visualization: view } : request,
    );
    const pending = this.viewTimers.get(id);
    if (pending) clearTimeout(pending);
    this.viewTimers.set(
      id,
      setTimeout(() => {
        this.viewTimers.delete(id);
        const current = this.get(id);
        if (current) void workspaceRpc.updateRequest(current);
      }, 300),
    );
  }

  async remove(id: number): Promise<void> {
    await workspaceRpc.deleteRequest(id);
    this.requests = this.requests.filter((request) => request.id !== id);
  }
}

export const requestsStore = new RequestsStore();
