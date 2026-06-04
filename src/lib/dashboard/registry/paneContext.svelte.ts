import { connectionStore } from "$lib/stores/connectionStore.svelte";
import { pipelineHub } from "$lib/core/pipelineHub";
import { getNodeTitle, renameNode, updatePaneConfig } from "$lib/stores/dashboardStore.svelte";
import type { ConnectionSummary, PaneContext } from "$lib/dashboard/registry/paneRegistry";
import type { PaneNode } from "$lib/dashboard/layout/layout";

export function createPaneContext(getLayoutId: () => string, getPane: () => PaneNode): PaneContext {
  const connections = $derived<ConnectionSummary[]>(
    connectionStore.connections.map((connection) => ({
      id: connection.id,
      name: connection.name,
      status: connectionStore.status(connection.id),
    })),
  );

  return {
    get nodeId() {
      return getPane().id;
    },
    get title() {
      return getNodeTitle(getLayoutId(), getPane().id);
    },
    get connections() {
      return connections;
    },
    persist: (patch) => updatePaneConfig(getLayoutId(), getPane().id, patch),
    setTitle: (title) => renameNode(getLayoutId(), getPane().id, title ?? ""),
    subscribe: (connectionId, topic) => {
      return pipelineHub.subscribe(requireSession(connectionId), topic);
    },
    callService: (connectionId, service, request) => {
      return pipelineHub.callService(requireSession(connectionId), service, request);
    },
    sendActionGoal: (connectionId, action, goal, onEnvelope) => {
      return pipelineHub.sendActionGoal(requireSession(connectionId), action, goal, onEnvelope);
    },
    cancelActionGoal: (goalId) => pipelineHub.cancelActionGoal(goalId),
  };
}

function requireSession(connectionId: number): string {
  const sessionId = connectionStore.sessionId(connectionId);
  if (!sessionId) throw new Error("connection is not connected");
  return sessionId;
}
