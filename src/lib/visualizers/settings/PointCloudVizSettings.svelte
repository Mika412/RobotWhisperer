<script lang="ts">
  import type { VisualizerSettingsProps } from "../types";

  type CloudConfig = { pointSize: number };
  let { config, onchange }: VisualizerSettingsProps<CloudConfig> = $props();
</script>

<label class="field">
  <span class="lbl">Point size: {(config.pointSize || 0.04).toFixed(3)}</span>
  <input
    type="range"
    min="0.01"
    max="0.2"
    step="0.005"
    value={config.pointSize || 0.04}
    oninput={(event) => onchange({ pointSize: Number(event.currentTarget.value) })}
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
  input[type="range"] {
    accent-color: var(--color-accent);
  }
</style>
