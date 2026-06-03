<script lang="ts">
  import Button from "$lib/components/common/Button.svelte";

  let {
    label,
    run,
    oncancel,
    disabled = false,
  }: {
    label: string;
    run: () => Promise<void>;
    oncancel?: () => void;
    disabled?: boolean;
  } = $props();

  let phase = $state<"idle" | "pending" | "success" | "error">("idle");
  let error = $state<string | null>(null);

  async function trigger() {
    if (phase === "pending") return;
    phase = "pending";
    error = null;
    try {
      await run();
      phase = "success";
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      phase = "error";
    }
  }
</script>

<div class="action">
  <Button onclick={trigger} disabled={disabled || phase === "pending"}>
    {phase === "pending" ? "Running…" : label}
  </Button>
  {#if oncancel && phase === "pending"}
    <Button variant="ghost" onclick={oncancel}>Cancel</Button>
  {/if}
  {#if phase === "error" && error}
    <span class="err">{error}</span>
  {/if}
</div>

<style>
  .action {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .err {
    color: var(--color-danger);
    font-family: var(--font-mono);
    font-size: 12px;
  }
</style>
