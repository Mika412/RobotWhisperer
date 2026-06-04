import { browser } from "$app/environment";
import {
  attach,
  detach,
  emptyLayout,
  getNode,
  isPlaceholder,
  makePane,
  newId,
  normaliseSizes,
  validate,
  gc,
  type DropZone,
  type Layout,
  type NodeId,
} from "$lib/dashboard/layout/layout";
import { getPane } from "$lib/dashboard/registry/paneRegistry";

const STORAGE_KEY = "rw:dashboards:v2";
const SAVE_DEBOUNCE_MS = 250;

const state = $state<{ layouts: Record<string, Layout>; order: string[] }>({
  layouts: {},
  order: [],
});

let saveTimer: ReturnType<typeof setTimeout> | null = null;

function persist(): void {
  if (!browser) return;
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    saveTimer = null;
    const ordered = state.order.map((id) => state.layouts[id]).filter(Boolean);
    localStorage.setItem(STORAGE_KEY, JSON.stringify($state.snapshot(ordered)));
  }, SAVE_DEBOUNCE_MS);
}

function load(): void {
  if (!browser) return;
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return;
  try {
    for (const layout of JSON.parse(raw) as Layout[]) {
      if (validate(layout).length > 0) continue;
      state.layouts[layout.id] = layout;
      state.order.push(layout.id);
    }
  } catch {}
}

load();

function mutate(layoutId: string, fn: (layout: Layout) => void): void {
  const layout = state.layouts[layoutId];
  if (!layout) return;
  fn(layout);
  gc(layout);
  if (import.meta.env.DEV) {
    const errors = validate(layout);
    if (errors.length > 0) console.error(`[dashboard] ${layoutId}:\n${errors.join("\n")}`);
  }
  persist();
}

export const dashboardState = state;

export const dashboardStore = {
  get layouts(): Layout[] {
    return state.order.map((id) => state.layouts[id]).filter(Boolean);
  },
  get(id: string): Layout | undefined {
    return state.layouts[id];
  },
  create(title = "New dashboard"): Layout {
    const id = newId("dash");
    const layout = emptyLayout(id, title);
    state.layouts[id] = layout;
    state.order.push(id);
    persist();
    return layout;
  },
  remove(id: string): void {
    delete state.layouts[id];
    state.order = state.order.filter((entry) => entry !== id);
    persist();
  },
  rename(id: string, title: string): void {
    mutate(id, (layout) => {
      layout.title = title;
    });
  },
};

export function getNodeTitle(layoutId: string, nodeId: NodeId): string | null {
  const layout = state.layouts[layoutId];
  const node = layout ? getNode(layout, nodeId) : undefined;
  return node?.kind === "pane" ? (node.title ?? null) : null;
}

export function renameNode(layoutId: string, nodeId: NodeId, title: string): void {
  mutate(layoutId, (layout) => {
    const node = layout.nodes[nodeId];
    if (node?.kind === "pane") node.title = title.trim() || undefined;
  });
}

export function updatePaneConfig(layoutId: string, paneId: NodeId, patch: unknown): void {
  mutate(layoutId, (layout) => {
    const node = layout.nodes[paneId];
    if (node?.kind === "pane" && patch && typeof patch === "object") {
      node.config = { ...node.config, ...(patch as Record<string, unknown>) };
    }
  });
}

export interface NewPaneSpec {
  paneType: string;
  title?: string;
  config?: Record<string, unknown>;
}

function buildPane(spec: NewPaneSpec) {
  const defaults = getPane(spec.paneType)?.defaultConfig ?? {};
  return makePane(spec.paneType, { ...defaults, ...(spec.config ?? {}) }, spec.title);
}

export function addPane(layoutId: string, spec: NewPaneSpec): NodeId {
  let paneId = "";
  mutate(layoutId, (layout) => {
    const root = layout.nodes[layout.root];
    if (isPlaceholder(root)) {
      const pane = buildPane(spec);
      paneId = pane.id;
      delete layout.nodes[layout.root];
      layout.nodes[pane.id] = pane;
      layout.root = pane.id;
      return;
    }
    const pane = buildPane(spec);
    paneId = pane.id;
    layout.nodes[pane.id] = pane;
    attach(layout, layout.root, "right", pane.id);
  });
  return paneId;
}

export function dropNewPane(
  layoutId: string,
  targetId: NodeId,
  where: DropZone,
  spec: NewPaneSpec,
): NodeId {
  let paneId = "";
  mutate(layoutId, (layout) => {
    if (isPlaceholder(layout.nodes[layout.root])) {
      const pane = buildPane(spec);
      paneId = pane.id;
      delete layout.nodes[layout.root];
      layout.nodes[pane.id] = pane;
      layout.root = pane.id;
      return;
    }
    const pane = buildPane(spec);
    paneId = pane.id;
    layout.nodes[pane.id] = pane;
    attach(layout, targetId, where, pane.id);
  });
  return paneId;
}

export function moveNode(
  layoutId: string,
  sourceId: NodeId,
  targetId: NodeId,
  where: DropZone,
): void {
  mutate(layoutId, (layout) => {
    let effectiveTarget = targetId;
    const target = layout.nodes[targetId];
    if (where !== "center" && target?.kind === "group" && target.tabs.includes(sourceId)) {
      const survivors = target.tabs.filter((tab) => tab !== sourceId);
      if (survivors.length === 1) effectiveTarget = survivors[0];
    }
    detach(layout, sourceId);
    if (!layout.nodes[effectiveTarget]) return;
    attach(layout, effectiveTarget, where, sourceId);
  });
}

export function removeNode(layoutId: string, nodeId: NodeId): void {
  mutate(layoutId, (layout) => {
    if (nodeId === layout.root) {
      const placeholder = makePane("rw.placeholder");
      layout.nodes = { [placeholder.id]: placeholder };
      layout.root = placeholder.id;
      return;
    }
    detach(layout, nodeId);
  });
}

export function resizeSplit(layoutId: string, splitId: NodeId, sizes: number[]): void {
  mutate(layoutId, (layout) => {
    const node = layout.nodes[splitId];
    if (node?.kind === "split" && sizes.length === node.children.length) {
      node.sizes = normaliseSizes(sizes);
    }
  });
}

export function setActiveTab(layoutId: string, groupId: NodeId, tabId: NodeId): void {
  mutate(layoutId, (layout) => {
    const node = layout.nodes[groupId];
    if (node?.kind === "group" && node.tabs.includes(tabId)) node.activeTab = tabId;
  });
}
