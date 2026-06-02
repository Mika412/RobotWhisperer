import { SvelteSet } from "svelte/reactivity";

class SubscriptionStore {
  private active = new SvelteSet<number>();

  setActive(requestId: number, value: boolean): void {
    if (value) this.active.add(requestId);
    else this.active.delete(requestId);
  }

  isActive(requestId: number): boolean {
    return this.active.has(requestId);
  }
}

export const subscriptionStore = new SubscriptionStore();
