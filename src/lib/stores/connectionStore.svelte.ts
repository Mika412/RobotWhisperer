import { SvelteMap } from "svelte/reactivity";
import { workspaceRpc } from "$lib/core/workspaceRpc";
import { pipelineHub } from "$lib/core/pipelineHub";
import type {
  Connection,
  ConnectionId,
  NewConnection,
  SessionId,
  TransportStatus,
} from "$lib/core/types";

interface SessionState {
  sessionId: SessionId;
  status: TransportStatus;
  error: string | null;
}

async function openTransport(connection: Connection): Promise<SessionId> {
  const config = connection.config;
  switch (config.kind) {
    case "foxglove_ws":
      return pipelineHub.openFoxglove(config.url);
    case "rosbridge":
      return pipelineHub.openRosbridge(config.url);
    case "dummy":
      return pipelineHub.openDummy();
    case "native_ros2":
      throw new Error("native ROS 2 transport is not available yet");
  }
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  if (error && typeof error === "object") {
    const obj = error as Record<string, unknown>;
    if (typeof obj.message === "string" && obj.message.length > 0) {
      return typeof obj.kind === "string" ? `${obj.kind}: ${obj.message}` : obj.message;
    }
    try {
      return JSON.stringify(error);
    } catch {
      return "unknown error";
    }
  }
  return String(error);
}

class ConnectionStore {
  connections = $state<Connection[]>([]);
  initialized = $state(false);
  private sessions = new SvelteMap<ConnectionId, SessionState>();

  async load(): Promise<void> {
    try {
      this.connections = await workspaceRpc.listConnections();
    } catch (error) {
      console.error("[connections] failed to load", error);
      this.connections = [];
    }
    this.initialized = true;
    for (const connection of this.connections) {
      if (connection.auto_connect) {
        void this.activate(connection.id).catch(() => {});
      }
    }
  }

  async create(draft: NewConnection): Promise<Connection> {
    const connection = await workspaceRpc.createConnection(draft);
    this.connections = [...this.connections, connection];
    return connection;
  }

  async update(connection: Connection): Promise<void> {
    await workspaceRpc.updateConnection(connection);
    this.connections = this.connections.map((c) => (c.id === connection.id ? connection : c));
  }

  async remove(id: ConnectionId): Promise<void> {
    await this.deactivate(id);
    await workspaceRpc.deleteConnection(id);
    this.connections = this.connections.filter((c) => c.id !== id);
  }

  async activate(id: ConnectionId): Promise<SessionId> {
    const connection = this.connections.find((c) => c.id === id);
    if (!connection) throw new Error(`connection ${id} not found`);

    this.setSession(id, { sessionId: "", status: "connecting", error: null });
    try {
      const sessionId = await openTransport(connection);
      this.setSession(id, { sessionId, status: "connected", error: null });
      return sessionId;
    } catch (error) {
      this.setSession(id, { sessionId: "", status: "failed", error: errorMessage(error) });
      throw error;
    }
  }

  async deactivate(id: ConnectionId): Promise<void> {
    const session = this.sessions.get(id);
    if (session?.sessionId) {
      await pipelineHub.close(session.sessionId);
    }
    this.clearSession(id);
  }

  sessionId(id: ConnectionId): SessionId | null {
    const session = this.sessions.get(id);
    return session && session.status === "connected" ? session.sessionId : null;
  }

  status(id: ConnectionId): TransportStatus {
    return this.sessions.get(id)?.status ?? "disconnected";
  }

  error(id: ConnectionId): string | null {
    return this.sessions.get(id)?.error ?? null;
  }

  get connected(): Connection[] {
    return this.connections.filter((c) => this.status(c.id) === "connected");
  }

  private setSession(id: ConnectionId, state: SessionState): void {
    this.sessions.set(id, state);
  }

  private clearSession(id: ConnectionId): void {
    this.sessions.delete(id);
  }
}

export const connectionStore = new ConnectionStore();
