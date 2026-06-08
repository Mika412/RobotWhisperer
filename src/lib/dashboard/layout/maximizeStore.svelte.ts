let layoutId = $state<string | null>(null);
let paneId = $state<string | null>(null);

export const maximizeStore = {
  maximizedNodeFor(layout: string): string | null {
    return layoutId === layout ? paneId : null;
  },
  isMaximized(layout: string, pane: string): boolean {
    return layoutId === layout && paneId === pane;
  },
  toggle(layout: string, pane: string): void {
    const same = layoutId === layout && paneId === pane;
    layoutId = same ? null : layout;
    paneId = same ? null : pane;
  },
  clear(): void {
    layoutId = null;
    paneId = null;
  },
};
