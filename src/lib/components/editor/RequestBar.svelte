<script lang="ts">
  import { Play, Square, ArrowLeftRight, Send } from "@lucide/svelte";
  import type { RequestKind } from "$lib/core/types";
  import type { RequestTab } from "$lib/stores/tabsStore.svelte";
  import type { TargetSuggestion } from "$lib/stores/discoveryStore.svelte";
  import Select from "$lib/components/common/Select.svelte";
  import TargetAutocomplete from "$lib/components/editor/TargetAutocomplete.svelte";

  let {
    tab,
    connectionValue,
    connectionOptions,
    suggestions,
    locked,
    canRun,
    pending,
    running = $bindable(),
    setConnection,
    onkind,
    ontarget,
    onrefresh,
    oncall,
    onsend,
    oncancel,
  }: {
    tab: RequestTab;
    connectionValue: string;
    connectionOptions: { value: string; label: string }[];
    suggestions: TargetSuggestion[];
    locked: boolean;
    canRun: boolean;
    pending: boolean;
    running: boolean;
    setConnection: (value: string) => void;
    onkind: (kind: RequestKind) => void;
    ontarget: (value: string) => void;
    onrefresh: () => void;
    oncall: () => void;
    onsend: () => void;
    oncancel: () => void;
  } = $props();

  const KIND_OPTIONS = [
    { value: "topic", label: "Topic" },
    { value: "service", label: "Service" },
    { value: "action", label: "Action" },
    { value: "param", label: "Param" },
  ];
</script>

<div class="bar">
  <div class="seg kind" style="--kind-color: var(--color-badge-{tab.draft.kind}-text)">
    <Select
      bare
      value={tab.draft.kind}
      options={KIND_OPTIONS}
      disabled={locked}
      onchange={(value) => onkind(value as RequestKind)}
    />
  </div>
  <div class="seg conn">
    <Select
      bare
      value={connectionValue}
      options={connectionOptions}
      disabled={locked}
      onchange={setConnection}
    />
  </div>
  <div class="seg target">
    <TargetAutocomplete
      bare
      value={tab.draft.target}
      onchange={ontarget}
      {suggestions}
      placeholder="/topic_or_service_name"
      {onrefresh}
      disabled={locked}
    />
  </div>

  {#if tab.draft.kind === "topic"}
    <button
      class="action-seg"
      class:stop={running}
      onclick={() => (running = !running)}
      disabled={!canRun && !running}
    >
      {#if running}<Square size={14} /> Stop{:else}<Play size={14} /> Subscribe{/if}
    </button>
  {:else if tab.draft.kind === "service"}
    <button class="action-seg" onclick={oncall} disabled={!canRun || pending}>
      <ArrowLeftRight size={14} />
      {pending ? "Calling…" : "Call"}
    </button>
  {:else if tab.draft.kind === "action"}
    {#if pending}
      <button class="action-seg cancel" onclick={oncancel}>Cancel</button>
    {/if}
    <button class="action-seg" onclick={onsend} disabled={!canRun || pending}>
      <Send size={14} />
      {pending ? "Sending…" : "Send goal"}
    </button>
  {/if}
</div>

<style>
  .bar {
    display: flex;
    align-items: stretch;
    height: 42px;
    border: 1px solid var(--color-border);
    border-radius: 10px;
    background: var(--color-bg-input);
  }
  .bar:focus-within {
    border-color: var(--color-accent);
  }
  .seg {
    display: flex;
    align-items: center;
    border-right: 1px solid var(--color-border);
    min-width: 0;
  }
  .seg.kind {
    width: 122px;
    flex: none;
    border-top-left-radius: 9px;
    border-bottom-left-radius: 9px;
    background: color-mix(in srgb, var(--kind-color) 16%, transparent);
  }
  .seg.conn {
    width: 180px;
    flex: none;
  }
  .seg.target {
    flex: 1;
    min-width: 0;
  }
  .action-seg {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0 16px;
    flex: none;
    border: 0;
    border-left: 1px solid var(--color-border);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    background: color-mix(in srgb, var(--color-accent) 18%, transparent);
    color: var(--color-accent);
    transition:
      background 0.12s,
      filter 0.12s;
  }
  .action-seg:last-child {
    border-top-right-radius: 9px;
    border-bottom-right-radius: 9px;
  }
  .action-seg:hover:not(:disabled) {
    background: color-mix(in srgb, var(--color-accent) 26%, transparent);
  }
  .action-seg:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .action-seg.stop {
    background: color-mix(in srgb, var(--color-danger) 16%, transparent);
    color: var(--color-danger);
  }
  .action-seg.stop:hover:not(:disabled) {
    background: color-mix(in srgb, var(--color-danger) 24%, transparent);
  }
  .action-seg.cancel {
    background: transparent;
    color: var(--color-text-dimmer);
    font-weight: 500;
  }
  .action-seg.cancel:hover:not(:disabled) {
    background: var(--color-bg-hover);
    color: var(--color-text-main);
  }
</style>
