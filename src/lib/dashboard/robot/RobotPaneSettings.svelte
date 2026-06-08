<script lang="ts">
  import type { PaneSettingsProps } from "$lib/dashboard/registry/paneRegistry";
  import Select from "$lib/components/common/Select.svelte";
  import { loadRobotCatalog, type RobotDefinition } from "$lib/robotkit";

  interface RobotPaneConfig {
    model: string | null;
    jointValues: Record<string, number>;
    showAxes: boolean;
    controlsCollapsed: boolean;
  }

  let { config, onchange }: PaneSettingsProps<RobotPaneConfig> = $props();

  let entries = $state<RobotDefinition[]>([]);
  void loadRobotCatalog().then((catalog) => (entries = catalog.robots));

  const options = $derived(
    entries.map((entry) => ({
      value: entry.directory,
      label: entry.displayName,
    })),
  );

  function chooseModel(directory: string) {
    if (directory === config.model) return;
    onchange({ model: directory, jointValues: {} });
  }
</script>

<div class="field">
  <span class="lbl">Robot model</span>
  <Select
    compact
    value={config.model ?? ""}
    {options}
    placeholder={entries.length === 0 ? "No models bundled" : "Select a robot…"}
    onchange={chooseModel}
  />
</div>

<label class="toggle">
  <input
    type="checkbox"
    checked={config.showAxes}
    onchange={(event) => onchange({ showAxes: event.currentTarget.checked })}
  />
  <span class="lbl">Show base axes</span>
</label>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }
  .toggle input {
    accent-color: var(--color-accent);
  }
  .lbl {
    font-family: var(--font-mono);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-text-dimmer);
  }
</style>
