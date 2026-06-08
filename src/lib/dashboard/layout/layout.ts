export type NodeId = string;

export type SplitDirection = "row" | "column";

export type DropZone = "left" | "right" | "top" | "bottom" | "center";

export interface PaneNode {
  kind: "pane";
  id: NodeId;
  paneType: string;
  title?: string;
  config: Record<string, unknown>;
}

export interface SplitNode {
  kind: "split";
  id: NodeId;
  direction: SplitDirection;
  children: NodeId[];
  sizes: number[];
}

export interface GroupNode {
  kind: "group";
  id: NodeId;
  tabs: NodeId[];
  activeTab: NodeId;
}

export type LayoutNode = PaneNode | SplitNode | GroupNode;

export interface Layout {
  id: string;
  title: string;
  root: NodeId;
  nodes: Record<NodeId, LayoutNode>;
}

export const PLACEHOLDER_PANE_TYPE = "rw.placeholder";

let counter = 0;
export function newId(prefix = "n"): string {
  counter += 1;
  return `${prefix}_${counter.toString(36)}_${Math.trunc(performance.now()).toString(36)}`;
}

export function makePane(
  paneType: string,
  config: Record<string, unknown> = {},
  title?: string,
): PaneNode {
  return { kind: "pane", id: newId("pane"), paneType, title, config };
}

export function emptyLayout(id: string, title: string): Layout {
  const placeholder = makePane(PLACEHOLDER_PANE_TYPE);
  return { id, title, root: placeholder.id, nodes: { [placeholder.id]: placeholder } };
}

export function isPlaceholder(node: LayoutNode | undefined): boolean {
  return node?.kind === "pane" && node.paneType === PLACEHOLDER_PANE_TYPE;
}

export function getNode(layout: Layout, id: NodeId): LayoutNode | undefined {
  return layout.nodes[id];
}

export function evenSizes(count: number): number[] {
  return count <= 0 ? [] : Array(count).fill(1 / count);
}

export function normaliseSizes(sizes: number[]): number[] {
  const MIN = 0.05;
  if (sizes.length <= 1) return sizes.length === 1 ? [1] : [];
  const positive = sizes.map((size) => (size > 0 ? size : 0));
  const total = positive.reduce((sum, size) => sum + size, 0);
  if (total === 0 || MIN * sizes.length >= 1) return evenSizes(sizes.length);
  const scaled = positive.map((size) => size / total);
  if (scaled.every((size) => size >= MIN)) return scaled;
  const floored = scaled.map((size) => Math.max(size, MIN));
  const slack = floored.reduce((sum, size) => sum + size, 0) - 1;
  if (slack <= 0) return floored;
  const headroom = floored.map((size) => Math.max(0, size - MIN));
  const headroomTotal = headroom.reduce((sum, size) => sum + size, 0);
  if (headroomTotal === 0) return evenSizes(sizes.length);
  return floored.map((size, index) => size - slack * (headroom[index] / headroomTotal));
}

export function findParentSplit(
  layout: Layout,
  id: NodeId,
): { parent: SplitNode; index: number } | null {
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "split") continue;
    const index = node.children.indexOf(id);
    if (index >= 0) return { parent: node, index };
  }
  return null;
}

export function findGroupOf(
  layout: Layout,
  paneId: NodeId,
): { group: GroupNode; index: number } | null {
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "group") continue;
    const index = node.tabs.indexOf(paneId);
    if (index >= 0) return { group: node, index };
  }
  return null;
}

export function replaceRef(layout: Layout, oldId: NodeId, newId: NodeId): void {
  if (layout.root === oldId) {
    layout.root = newId;
    return;
  }
  const found = findParentSplit(layout, oldId);
  if (found) {
    found.parent.children = found.parent.children.map((child) => (child === oldId ? newId : child));
  }
}

export function isDescendant(layout: Layout, ancestorId: NodeId, candidateId: NodeId): boolean {
  if (ancestorId === candidateId) return true;
  const node = layout.nodes[ancestorId];
  if (node?.kind === "split") {
    return node.children.some((child) => isDescendant(layout, child, candidateId));
  }
  if (node?.kind === "group") {
    return node.tabs.some((tab) => isDescendant(layout, tab, candidateId));
  }
  return false;
}

export function detach(layout: Layout, nodeId: NodeId): void {
  const group = findGroupOf(layout, nodeId);
  if (group) {
    group.group.tabs.splice(group.index, 1);
    if (group.group.tabs.length === 1) {
      const survivor = group.group.tabs[0];
      replaceRef(layout, group.group.id, survivor);
      delete layout.nodes[group.group.id];
    } else if (group.group.activeTab === nodeId) {
      group.group.activeTab = group.group.tabs[Math.max(0, group.index - 1)];
    }
    return;
  }
  const split = findParentSplit(layout, nodeId);
  if (!split) return;
  split.parent.children.splice(split.index, 1);
  split.parent.sizes.splice(split.index, 1);
  if (split.parent.children.length === 1) {
    const survivor = split.parent.children[0];
    replaceRef(layout, split.parent.id, survivor);
    delete layout.nodes[split.parent.id];
  } else {
    split.parent.sizes = normaliseSizes(split.parent.sizes);
  }
}

function containerOf(layout: Layout, targetId: NodeId): NodeId {
  const group = findGroupOf(layout, targetId);
  return group ? group.group.id : targetId;
}

export function attach(layout: Layout, targetId: NodeId, where: DropZone, nodeId: NodeId): void {
  const containerId = containerOf(layout, targetId);
  const target = layout.nodes[containerId];
  if (!target) return;

  if (where === "center") {
    if (target.kind === "group") {
      target.tabs.push(nodeId);
      target.activeTab = nodeId;
      return;
    }
    if (target.kind === "pane") {
      const group: GroupNode = {
        kind: "group",
        id: newId("group"),
        tabs: [containerId, nodeId],
        activeTab: nodeId,
      };
      layout.nodes[group.id] = group;
      replaceRef(layout, containerId, group.id);
    }
    return;
  }

  const direction: SplitDirection = where === "left" || where === "right" ? "row" : "column";
  const before = where === "left" || where === "top";
  const found = findParentSplit(layout, containerId);

  if (found && found.parent.direction === direction) {
    const parent = found.parent;
    const insertAt = before ? found.index : found.index + 1;
    const half = parent.sizes[found.index] / 2;
    parent.sizes[found.index] = half;
    parent.children.splice(insertAt, 0, nodeId);
    parent.sizes.splice(insertAt, 0, half);
    parent.sizes = normaliseSizes(parent.sizes);
    return;
  }

  const split: SplitNode = {
    kind: "split",
    id: newId("split"),
    direction,
    children: before ? [nodeId, containerId] : [containerId, nodeId],
    sizes: [0.5, 0.5],
  };
  layout.nodes[split.id] = split;
  replaceRef(layout, containerId, split.id);
}

export function canDrop(
  layout: Layout,
  payload: { kind: "new-pane" } | { kind: "move-node"; nodeId: NodeId },
  targetId: NodeId,
  zone: DropZone,
): boolean {
  if (payload.kind === "move-node") {
    if (payload.nodeId === targetId) return false;
    if (isDescendant(layout, payload.nodeId, targetId)) return false;
    const source = layout.nodes[payload.nodeId];
    if (source?.kind === "group" && zone === "center") return false;
  }
  return true;
}

export function gc(layout: Layout): void {
  const reachable = new Set<NodeId>();
  const stack: NodeId[] = [layout.root];
  while (stack.length > 0) {
    const id = stack.pop()!;
    if (reachable.has(id)) continue;
    reachable.add(id);
    const node = layout.nodes[id];
    if (node?.kind === "split") stack.push(...node.children);
    else if (node?.kind === "group") stack.push(...node.tabs);
  }
  for (const id of Object.keys(layout.nodes)) {
    if (!reachable.has(id)) delete layout.nodes[id];
  }
}

export function validate(layout: Layout): string[] {
  const errors: string[] = [];
  if (!layout.nodes[layout.root]) errors.push(`root '${layout.root}' missing`);
  for (const [id, node] of Object.entries(layout.nodes)) {
    if (node.id !== id) errors.push(`id mismatch for '${id}'`);
    if (node.kind === "split") {
      if (node.children.length < 2) errors.push(`split '${id}' has < 2 children`);
      if (node.children.length !== node.sizes.length) errors.push(`split '${id}' size mismatch`);
      for (const child of node.children) {
        if (!layout.nodes[child]) errors.push(`split '${id}' missing child '${child}'`);
      }
    } else if (node.kind === "group") {
      if (node.tabs.length < 2) errors.push(`group '${id}' has < 2 tabs`);
      if (!node.tabs.includes(node.activeTab)) errors.push(`group '${id}' bad activeTab`);
      for (const tab of node.tabs) {
        const child = layout.nodes[tab];
        if (!child) errors.push(`group '${id}' missing tab '${tab}'`);
        else if (child.kind !== "pane") errors.push(`group '${id}' holds non-pane '${tab}'`);
      }
    }
  }
  return errors;
}
