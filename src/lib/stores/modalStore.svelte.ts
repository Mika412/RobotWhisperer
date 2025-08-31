import type { SvelteComponent } from 'svelte';

// A generic type for any Svelte component constructor
// FIX: This is a hack that will need to be replaced
type AnySvelteComponent = new (...args: any[]) => SvelteComponent;

let modal = $state({
  component: null as AnySvelteComponent | null,
  props: {} as Record<string, any>
});

export const modalStore = {
  get isOpen() {
    return modal.component !== null;
  },

  get component() {
    return modal.component;
  },

  get props() {
    return modal.props;
  },

  open: (component: AnySvelteComponent, props = {}) => {
    modal.component = component;
    modal.props = props;
  },

  close: () => {
    modal.component = null;
    modal.props = {};
  }
};
