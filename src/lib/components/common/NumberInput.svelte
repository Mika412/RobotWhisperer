<script lang="ts">
  import type { HTMLInputAttributes } from "svelte/elements";

  let {
    size = "md",
    mono = true,
    class: extraClass = "",
    ...rest
  }: {
    size?: "sm" | "md" | "lg";
    mono?: boolean;
    class?: string;
  } & Omit<HTMLInputAttributes, "size"> = $props();
</script>

<input type="number" class="field-input {size} {extraClass}" class:mono {...rest} />
