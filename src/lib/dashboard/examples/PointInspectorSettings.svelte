<script lang="ts">
  import type { PaneSettingsProps } from "$lib/dashboard/registry/paneRegistry";
  import TextInput from "$lib/components/common/TextInput.svelte";
  import LiveFieldTree from "$lib/visualizers/LiveFieldTree.svelte";

  interface InspectorConfig {
    connectionId: number | null;
    topic: string;
    field: string;
    service: string;
  }

  let { config, source, onchange }: PaneSettingsProps<InspectorConfig> = $props();

  const selected = $derived(new Set(config.field ? [config.field] : []));
</script>

<div class="field">
  <span class="lbl">Field: pick a numeric value from the live message</span>
  <LiveFieldTree {source} {selected} onpick={(path) => onchange({ field: path })} />
</div>

<label class="field">
  <span class="lbl">Service to call (optional)</span>
  <TextInput
    size="sm"
    value={config.service ?? ""}
    placeholder="/my/service"
    oninput={(event) => onchange({ service: event.currentTarget.value })}
  />
</label>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .lbl {
    font-family: var(--font-mono);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-text-dimmer);
  }
</style>
