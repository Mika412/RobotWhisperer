<script lang="ts">
  import type { VisualizerSettingsProps } from "../types";
  import Slider from "$lib/components/common/Slider.svelte";

  type CloudConfig = { pointSize: number };
  let { config, onchange }: VisualizerSettingsProps<CloudConfig> = $props();

  const pointSize = $derived(config.pointSize || 0.04);
</script>

<Slider
  label="Point size"
  display={pointSize.toFixed(3)}
  value={pointSize}
  min={0.01}
  max={0.2}
  step={0.005}
  oninput={(value) => onchange({ pointSize: value })}
/>
