import { SvelteMap } from "svelte/reactivity";
import { pipelineRpc } from "$lib/core/pipelineRpc";
import type { DiscoverySnapshot } from "$lib/core/pipelineRpc.shared";
import { connectionStore } from "./connectionStore.svelte";
import type { ConnectionId, RequestKind } from "$lib/core/types";

export interface TargetSuggestion {
  name: string;
  schemaName: string;
}

const MIN_REFRESH_INTERVAL_MS = 750;

class DiscoveryStore {
  private byConnection = new SvelteMap<ConnectionId, DiscoverySnapshot>();
  private inFlight = new Set<ConnectionId>();
  private lastRefreshAt = new Map<ConnectionId, number>();

  async refresh(connectionId: ConnectionId, { force = false } = {}): Promise<void> {
    const sessionId = connectionStore.sessionId(connectionId);
    if (!sessionId || this.inFlight.has(connectionId)) return;
    const since = Date.now() - (this.lastRefreshAt.get(connectionId) ?? 0);
    if (!force && since < MIN_REFRESH_INTERVAL_MS) return;
    this.inFlight.add(connectionId);
    try {
      const snapshot = await pipelineRpc.getDiscovery(sessionId);
      if (snapshot) this.byConnection.set(connectionId, snapshot);
      this.lastRefreshAt.set(connectionId, Date.now());
    } catch (error) {
      console.warn("[discovery] refresh failed", error);
    } finally {
      this.inFlight.delete(connectionId);
    }
  }

  suggestions(connectionId: ConnectionId | null, kind: RequestKind): TargetSuggestion[] {
    if (connectionId == null) return [];
    const snapshot = this.byConnection.get(connectionId);
    if (!snapshot) return [];
    const entries =
      kind === "service"
        ? snapshot.services
        : kind === "action"
          ? snapshot.actions
          : snapshot.topics;
    return entries.map((entry) => ({ name: entry.name, schemaName: entry.schema.name }));
  }
}

export const discoveryStore = new DiscoveryStore();
