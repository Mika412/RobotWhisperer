import { describe, it, expect, beforeEach } from "vitest";
import { tabsStore, type WorkspaceTab } from "../tabsStore.svelte";
import { requestsStore } from "../requestsStore.svelte";
import type { Request } from "$lib/core/types";

function makeRequest(id: number, name: string): Request {
  return {
    id,
    collection_id: null,
    connection_id: null,
    name,
    kind: "topic",
    target: `/topic_${id}`,
    schema: null,
    input: { kind: "struct", value: {} },
    created_at: "",
    updated_at: "",
  };
}

beforeEach(() => {
  for (const tab of [...tabsStore.tabs]) tabsStore.close(tab.tabId);
  requestsStore.requests = [makeRequest(1, "Alpha"), makeRequest(2, "Beta")];
});

describe("tabsStore", () => {
  it("opens a request tab with an isolated draft copy", () => {
    tabsStore.openRequest(1);
    expect(tabsStore.tabs).toHaveLength(1);
    expect(tabsStore.activeTabId).toBe("request:1");

    tabsStore.requestTab(1)!.draft.name = "Edited";
    expect(requestsStore.get(1)?.name).toBe("Alpha");
  });

  it("does not duplicate a tab for an already-open request", () => {
    tabsStore.openRequest(1);
    tabsStore.openRequest(2);
    tabsStore.openRequest(1);
    expect(tabsStore.tabs).toHaveLength(2);
    expect(tabsStore.activeTabId).toBe("request:1");
  });

  it("opens dashboard tabs alongside requests", () => {
    tabsStore.openRequest(1);
    tabsStore.openDashboard("dash-a");
    expect(tabsStore.tabs.map((tab) => tab.kind)).toEqual(["request", "dashboard"]);
    expect(tabsStore.activeTabId).toBe("dashboard:dash-a");
  });

  it("revert restores the draft from the canonical record", () => {
    tabsStore.openRequest(1);
    tabsStore.requestTab(1)!.draft.name = "Edited";
    tabsStore.setDirty(1, true);
    tabsStore.revert(1);
    expect(tabsStore.requestTab(1)!.draft.name).toBe("Alpha");
    expect(tabsStore.requestTab(1)!.dirty).toBe(false);
  });

  it("close removes the tab, fires onClose, and reactivates a neighbour", () => {
    const closed: string[] = [];
    const off = tabsStore.onClose((tab: WorkspaceTab) => closed.push(tab.tabId));

    tabsStore.openRequest(1);
    tabsStore.openRequest(2);
    expect(tabsStore.activeTabId).toBe("request:2");

    tabsStore.close("request:2");
    expect(closed).toEqual(["request:2"]);
    expect(tabsStore.tabs.map((tab) => tab.tabId)).toEqual(["request:1"]);
    expect(tabsStore.activeTabId).toBe("request:1");

    off();
  });
});
