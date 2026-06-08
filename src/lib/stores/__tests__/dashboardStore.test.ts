import { describe, it, expect } from "vitest";
import {
  dashboardStore,
  dashboardState,
  addPane,
  dropNewPane,
  moveNode,
} from "../dashboardStore.svelte";
import type { GroupNode, PaneNode } from "$lib/dashboard/layout/layout";

function paneIds(layoutId: string): string[] {
  return Object.values(dashboardState.layouts[layoutId].nodes)
    .filter((node): node is PaneNode => node.kind === "pane")
    .map((node) => node.id);
}

function groupOf(layoutId: string): GroupNode | undefined {
  return Object.values(dashboardState.layouts[layoutId].nodes).find(
    (node): node is GroupNode => node.kind === "group",
  );
}

describe("moveNode", () => {
  it("keeps both panes when a 2-tab group member is dropped on the group edge", () => {
    const dash = dashboardStore.create("test");
    const a = addPane(dash.id, { paneType: "rw.test" });
    const c = dropNewPane(dash.id, a, "center", { paneType: "rw.test" });

    const group = groupOf(dash.id);
    expect(group).toBeTruthy();
    expect(group?.tabs).toHaveLength(2);

    moveNode(dash.id, c, group!.id, "left");

    const ids = paneIds(dash.id);
    expect(ids).toContain(a);
    expect(ids).toContain(c);
    expect(groupOf(dash.id)).toBeUndefined();

    dashboardStore.remove(dash.id);
  });

  it("splits beside a sibling without dropping a pane", () => {
    const dash = dashboardStore.create("test");
    const a = addPane(dash.id, { paneType: "rw.test" });
    const b = dropNewPane(dash.id, a, "right", { paneType: "rw.test" });

    moveNode(dash.id, b, a, "bottom");

    const ids = paneIds(dash.id);
    expect(ids).toContain(a);
    expect(ids).toContain(b);
    expect(ids).toHaveLength(2);

    dashboardStore.remove(dash.id);
  });
});
