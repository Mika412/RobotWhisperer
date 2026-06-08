export type DragPayload =
  | { kind: "new-pane"; paneType: string; title: string; defaultConfig: Record<string, unknown> }
  | { kind: "move-node"; nodeId: string };

interface DropController {
  hover(clientX: number, clientY: number): void;
  commit(): void;
  clear(): void;
}

interface DragSession {
  active: DragPayload | null;
  label: string;
  x: number;
  y: number;
}

const session = $state<DragSession>({ active: null, label: "", x: 0, y: 0 });

export const dragState = session;

let dropController: DropController | null = null;

export function registerDropController(controller: DropController): () => void {
  dropController = controller;
  return () => {
    if (dropController === controller) dropController = null;
  };
}

function beginDrag(payload: DragPayload, label: string, x: number, y: number): void {
  session.active = payload;
  session.label = label;
  session.x = x;
  session.y = y;
}

function moveDrag(x: number, y: number): void {
  session.x = x;
  session.y = y;
  dropController?.hover(x, y);
}

function dropDrag(): void {
  dropController?.commit();
  endDrag();
}

export function endDrag(): void {
  session.active = null;
  dropController?.clear();
}

interface DragSpec {
  payload: () => DragPayload;
  label: () => string;
}

const DRAG_THRESHOLD = 6;

export function draggable(node: HTMLElement, spec: DragSpec) {
  let current = spec;
  let pointerId = -1;
  let startX = 0;
  let startY = 0;
  let dragging = false;
  let swallow: ((event: Event) => void) | null = null;
  let swallowTimer: ReturnType<typeof setTimeout> | null = null;
  node.style.touchAction = "none";

  function clearSwallow(): void {
    if (swallowTimer !== null) {
      clearTimeout(swallowTimer);
      swallowTimer = null;
    }
    if (swallow) {
      node.removeEventListener("click", swallow, true);
      swallow = null;
    }
  }

  function teardown(): void {
    window.removeEventListener("pointermove", onMove, true);
    window.removeEventListener("pointerup", onUp, true);
    window.removeEventListener("pointercancel", onCancel, true);
    dragging = false;
    pointerId = -1;
  }

  function onDown(event: PointerEvent): void {
    if (event.pointerType === "mouse" && event.button !== 0) return;
    const claim = event as PointerEvent & { __rwDragClaimed?: boolean };
    if (claim.__rwDragClaimed) return;
    const target = event.target as HTMLElement | null;
    if (target?.closest("button, input, textarea, select, a, [data-no-drag]")) return;
    claim.__rwDragClaimed = true;
    pointerId = event.pointerId;
    startX = event.clientX;
    startY = event.clientY;
    dragging = false;
    window.addEventListener("pointermove", onMove, true);
    window.addEventListener("pointerup", onUp, true);
    window.addEventListener("pointercancel", onCancel, true);
  }

  function onMove(event: PointerEvent): void {
    if (event.pointerId !== pointerId) return;
    if (!dragging) {
      if (Math.hypot(event.clientX - startX, event.clientY - startY) < DRAG_THRESHOLD) return;
      dragging = true;
      node.setPointerCapture?.(pointerId);
      beginDrag(current.payload(), current.label(), event.clientX, event.clientY);
    }
    event.preventDefault();
    moveDrag(event.clientX, event.clientY);
  }

  function onUp(event: PointerEvent): void {
    if (event.pointerId !== pointerId) return;
    const wasDragging = dragging;
    teardown();
    if (wasDragging) {
      dropDrag();
      clearSwallow();
      swallow = (click: Event) => {
        click.stopPropagation();
        click.preventDefault();
      };
      node.addEventListener("click", swallow, { capture: true, once: true });
      swallowTimer = setTimeout(clearSwallow, 350);
    }
  }

  function onCancel(event: PointerEvent): void {
    if (event.pointerId !== pointerId) return;
    const wasDragging = dragging;
    teardown();
    if (wasDragging) endDrag();
  }

  node.addEventListener("pointerdown", onDown);

  return {
    update(next: DragSpec) {
      current = next;
    },
    destroy() {
      node.removeEventListener("pointerdown", onDown);
      teardown();
      clearSwallow();
    },
  };
}
