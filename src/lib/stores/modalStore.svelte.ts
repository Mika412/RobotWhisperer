import type { Component } from "svelte";

type ModalProps = Record<string, unknown>;

const modal = $state({
  component: null as Component<ModalProps> | null,
  props: {} as ModalProps,
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

  open: <Props extends ModalProps>(component: Component<Props>, props?: Props) => {
    modal.component = component as Component<ModalProps>;
    modal.props = props ?? {};
  },

  close: () => {
    modal.component = null;
    modal.props = {};
  },
};
