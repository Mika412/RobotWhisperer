import { describe, it, expect } from "vitest";
import {
  attach,
  canDrop,
  detach,
  emptyLayout,
  evenSizes,
  findGroupOf,
  findParentSplit,
  gc,
  isPlaceholder,
  normaliseSizes,
  validate,
  type GroupNode,
  type Layout,
  type PaneNode,
  type SplitNode,
} from "../layout/layout";

function pane(id: string): PaneNode {
  return { kind: "pane", id, paneType: "rw.raw", config: {} };
}

function twoPane(): Layout {
  const split: SplitNode = {
    kind: "split",
    id: "s",
    direction: "row",
    children: ["a", "b"],
    sizes: [0.5, 0.5],
  };
  return { id: "d", title: "D", root: "s", nodes: { s: split, a: pane("a"), b: pane("b") } };
}

describe("emptyLayout", () => {
  it("starts with a single placeholder pane as root", () => {
    const layout = emptyLayout("d1", "Dash");
    const root = layout.nodes[layout.root];
    expect(isPlaceholder(root)).toBe(true);
    expect(validate(layout)).toEqual([]);
  });
});

describe("normaliseSizes", () => {
  it("scales sizes to sum to 1", () => {
    expect(normaliseSizes([1, 1, 2]).reduce((a, b) => a + b, 0)).toBeCloseTo(1);
  });
  it("floors tiny tracks to a 5% minimum", () => {
    const sizes = normaliseSizes([0.001, 0.999]);
    expect(sizes[0]).toBeGreaterThanOrEqual(0.05);
    expect(sizes.reduce((a, b) => a + b, 0)).toBeCloseTo(1);
  });
  it("returns even sizes for an all-zero input", () => {
    expect(normaliseSizes([0, 0])).toEqual(evenSizes(2));
  });
});

describe("queries", () => {
  const layout = twoPane();
  it("finds the split parent of a child", () => {
    expect(findParentSplit(layout, "b")).toEqual({ parent: layout.nodes.s, index: 1 });
  });
  it("returns null for the root", () => {
    expect(findParentSplit(layout, "s")).toBeNull();
  });
});

describe("attach: edges", () => {
  it("inserts into a same-direction split as a sibling", () => {
    const layout = twoPane();
    layout.nodes.c = pane("c");
    attach(layout, "a", "right", "c");
    const split = layout.nodes.s as SplitNode;
    expect(split.children).toEqual(["a", "c", "b"]);
    expect(split.sizes.reduce((x, y) => x + y, 0)).toBeCloseTo(1);
    expect(validate(layout)).toEqual([]);
  });

  it("wraps the target in a new split for the cross axis", () => {
    const layout = twoPane();
    layout.nodes.c = pane("c");
    attach(layout, "a", "bottom", "c");
    const parent = findParentSplit(layout, "c");
    expect(parent?.parent.direction).toBe("column");
    expect(parent?.parent.children).toEqual(["a", "c"]);
    expect(validate(layout)).toEqual([]);
  });
});

describe("attach: center makes/extends a group", () => {
  it("combines two panes into a group", () => {
    const layout = twoPane();
    layout.nodes.c = pane("c");
    attach(layout, "a", "center", "c");
    const group = findGroupOf(layout, "a");
    expect(group).not.toBeNull();
    expect(group?.group.tabs).toEqual(["a", "c"]);
    expect(group?.group.activeTab).toBe("c");
    expect(validate(layout)).toEqual([]);
  });

  it("adds a tab when the target is already a group", () => {
    const layout = twoPane();
    layout.nodes.c = pane("c");
    attach(layout, "a", "center", "c");
    layout.nodes.d = pane("d");
    attach(layout, "a", "center", "d");
    const group = findGroupOf(layout, "a");
    expect(group?.group.tabs).toEqual(["a", "c", "d"]);
    expect(validate(layout)).toEqual([]);
  });
});

describe("detach: collapses redundant wrappers", () => {
  it("collapses a single-child split into its survivor", () => {
    const layout = twoPane();
    detach(layout, "a");
    expect(layout.root).toBe("b");
    expect(layout.nodes.s).toBeUndefined();
  });

  it("collapses a single-tab group back to a bare pane", () => {
    const layout = twoPane();
    layout.nodes.c = pane("c");
    attach(layout, "a", "center", "c");
    const groupId = findGroupOf(layout, "a")!.group.id;
    detach(layout, "c");
    expect(findGroupOf(layout, "a")).toBeNull();
    expect(layout.nodes[groupId]).toBeUndefined();
    const split = layout.nodes.s as SplitNode;
    expect(split.children).toContain("a");
    expect(validate(layout)).toEqual([]);
  });
});

describe("canDrop", () => {
  function withGroup(): { layout: Layout; groupId: string } {
    const layout = twoPane();
    layout.nodes.c = pane("c");
    attach(layout, "a", "center", "c");
    return { layout, groupId: findGroupOf(layout, "a")!.group.id };
  }

  it("forbids dropping a node onto itself", () => {
    const layout = twoPane();
    expect(canDrop(layout, { kind: "move-node", nodeId: "a" }, "a", "left")).toBe(false);
  });

  it("forbids dropping a node into its own descendant", () => {
    const layout = twoPane();
    expect(canDrop(layout, { kind: "move-node", nodeId: "s" }, "a", "left")).toBe(false);
  });

  it("forbids dropping a group into another tabset (no nested groups)", () => {
    const { layout, groupId } = withGroup();
    expect(canDrop(layout, { kind: "move-node", nodeId: groupId }, "b", "center")).toBe(false);
  });

  it("allows re-splitting a group at an edge", () => {
    const { layout, groupId } = withGroup();
    expect(canDrop(layout, { kind: "move-node", nodeId: groupId }, "b", "right")).toBe(true);
  });

  it("always allows a fresh pane at the centre", () => {
    const layout = twoPane();
    expect(canDrop(layout, { kind: "new-pane" }, "a", "center")).toBe(true);
  });
});

describe("gc", () => {
  it("drops nodes unreachable from the root", () => {
    const layout: Layout = {
      id: "d",
      title: "D",
      root: "a",
      nodes: { a: pane("a"), orphan: pane("orphan") },
    };
    gc(layout);
    expect(Object.keys(layout.nodes)).toEqual(["a"]);
  });
});

describe("validate", () => {
  it("flags a split whose child is missing", () => {
    const layout: Layout = {
      id: "d",
      title: "D",
      root: "s",
      nodes: {
        s: {
          kind: "split",
          id: "s",
          direction: "row",
          children: ["a", "ghost"],
          sizes: [0.5, 0.5],
        },
        a: pane("a"),
      },
    };
    expect(validate(layout).some((error) => error.includes("ghost"))).toBe(true);
  });

  it("flags a group holding a non-pane", () => {
    const group: GroupNode = { kind: "group", id: "g", tabs: ["a", "s"], activeTab: "a" };
    const layout: Layout = {
      id: "d",
      title: "D",
      root: "g",
      nodes: {
        g: group,
        a: pane("a"),
        s: { kind: "split", id: "s", direction: "row", children: ["a", "a"], sizes: [0.5, 0.5] },
      },
    };
    expect(validate(layout).some((error) => error.includes("non-pane"))).toBe(true);
  });
});
