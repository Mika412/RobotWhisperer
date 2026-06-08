<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  let {
    label,
    size = "md",
    tone = "default",
    active = false,
    class: extraClass = "",
    children,
    ...rest
  }: {
    label: string;
    size?: "sm" | "md";
    tone?: "default" | "danger";
    active?: boolean;
    class?: string;
    children: Snippet;
  } & HTMLButtonAttributes = $props();
</script>

<button
  type="button"
  class="icon-btn {size} {tone} {extraClass}"
  class:active
  aria-label={label}
  {...rest}
>
  {@render children()}
</button>

<style>
  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 0;
    border-radius: 4px;
    color: var(--color-text-disabled);
    cursor: pointer;
    -webkit-user-drag: none;
  }
  .icon-btn.sm {
    width: 22px;
    height: 22px;
  }
  .icon-btn.md {
    width: 24px;
    height: 24px;
  }
  .icon-btn:hover:not(:disabled) {
    background: var(--color-bg-hover);
    color: var(--color-text-main);
  }
  .icon-btn.active {
    background: color-mix(in srgb, var(--color-accent) 22%, transparent);
    color: var(--color-accent);
  }
  .icon-btn.danger:hover:not(:disabled) {
    color: var(--color-danger);
    background: color-mix(in srgb, var(--color-danger) 14%, transparent);
  }
  .icon-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
