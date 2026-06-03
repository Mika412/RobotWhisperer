<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  let {
    variant = "primary",
    type = "button",
    children,
    ...rest
  }: {
    variant?: "primary" | "ghost" | "danger";
    children: Snippet;
  } & HTMLButtonAttributes = $props();
</script>

<button {type} class="btn-{variant}" {...rest}>
  {@render children()}
</button>
