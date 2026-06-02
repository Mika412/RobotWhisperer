import { requestsStore } from "./requestsStore.svelte";
import type { Request, RequestId } from "$lib/core/types";

export interface RequestTab {
  kind: "request";
  tabId: string;
  requestId: RequestId;
  draft: Request;
  dirty: boolean;
}

export interface DashboardTab {
  kind: "dashboard";
  tabId: string;
  dashboardId: string;
}

export type WorkspaceTab = RequestTab | DashboardTab;

type CloseListener = (tab: WorkspaceTab) => void;

function cloneRequest(request: Request): Request {
  return $state.snapshot(request) as Request;
}

class TabsStore {
  tabs = $state<WorkspaceTab[]>([]);
  activeTabId = $state<string | null>(null);
  private closeListeners = new Set<CloseListener>();

  get active(): WorkspaceTab | null {
    return this.tabs.find((tab) => tab.tabId === this.activeTabId) ?? null;
  }

  onClose(listener: CloseListener): () => void {
    this.closeListeners.add(listener);
    return () => this.closeListeners.delete(listener);
  }

  openRequest(requestId: RequestId): void {
    const tabId = `request:${requestId}`;
    if (this.tabs.some((tab) => tab.tabId === tabId)) {
      this.activeTabId = tabId;
      return;
    }
    const request = requestsStore.get(requestId);
    if (!request) return;
    this.tabs = [
      ...this.tabs,
      { kind: "request", tabId, requestId, draft: cloneRequest(request), dirty: false },
    ];
    this.activeTabId = tabId;
  }

  openDashboard(dashboardId: string): void {
    const tabId = `dashboard:${dashboardId}`;
    if (this.tabs.some((tab) => tab.tabId === tabId)) {
      this.activeTabId = tabId;
      return;
    }
    this.tabs = [...this.tabs, { kind: "dashboard", tabId, dashboardId }];
    this.activeTabId = tabId;
  }

  requestTab(requestId: RequestId): RequestTab | undefined {
    return this.tabs.find(
      (tab): tab is RequestTab => tab.kind === "request" && tab.requestId === requestId,
    );
  }

  isActive(tabId: string): boolean {
    return this.activeTabId === tabId;
  }

  setActive(tabId: string): void {
    this.activeTabId = tabId;
  }

  setDirty(requestId: RequestId, value: boolean): void {
    const tab = this.requestTab(requestId);
    if (tab && tab.dirty !== value) tab.dirty = value;
  }

  async save(requestId: RequestId): Promise<void> {
    const tab = this.requestTab(requestId);
    if (!tab || !tab.dirty) return;
    await requestsStore.update(cloneRequest(tab.draft));
    tab.dirty = false;
  }

  revert(requestId: RequestId): void {
    const tab = this.requestTab(requestId);
    const canonical = requestsStore.get(requestId);
    if (!tab || !canonical) return;
    tab.draft = cloneRequest(canonical);
    tab.dirty = false;
  }

  close(tabId: string): void {
    const index = this.tabs.findIndex((tab) => tab.tabId === tabId);
    if (index < 0) return;
    const closed = this.tabs[index];
    this.tabs = this.tabs.filter((tab) => tab.tabId !== tabId);
    for (const listener of this.closeListeners) listener(closed);
    if (this.activeTabId === tabId) {
      const neighbour = this.tabs[Math.min(index, this.tabs.length - 1)];
      this.activeTabId = neighbour?.tabId ?? null;
    }
  }

  closeRequest(requestId: RequestId): void {
    this.close(`request:${requestId}`);
  }

  closeDashboard(dashboardId: string): void {
    this.close(`dashboard:${dashboardId}`);
  }
}

export const tabsStore = new TabsStore();
