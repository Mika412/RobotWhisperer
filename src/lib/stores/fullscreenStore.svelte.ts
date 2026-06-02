let active = $state(false);

export const fullscreenStore = {
  get isFullscreen(): boolean {
    return active;
  },
  toggle(): void {
    active = !active;
  },
  exit(): void {
    active = false;
  },
};
