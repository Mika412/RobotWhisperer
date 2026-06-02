<script lang="ts">
  import { ExternalLink } from "@lucide/svelte";
  import { connectionStore } from "$lib/stores/connectionStore.svelte";

  const PROJECT_NAME = "Robot Whisperer";
  const APP_VERSION = "v0.1.0";

  const connectedCount = $derived(connectionStore.connected.length);
</script>

<footer class="status-bar">
  <div class="cluster">
    <span class="status-dot {connectedCount > 0 ? 'running' : ''}"></span>
    <span class="mono">{connectedCount > 0 ? "connected" : "disconnected"}</span>
    <span class="sep">·</span>
    <span class="mono">{connectedCount} {connectedCount === 1 ? "bridge" : "bridges"}</span>
  </div>

  <div class="cluster right">
    <a
      class="credit"
      href="https://github.com/Mika412/RobotWhisperer"
      target="_blank"
      rel="noopener noreferrer"
      title={`${PROJECT_NAME}: open-source ROS client`}
    >
      <span>Built with</span>
      <span class="credit-name">{PROJECT_NAME}</span>
      <ExternalLink size={11} class="credit-ext" />
    </a>
    <a
      class="version"
      href="https://github.com/Mika412/RobotWhisperer/releases"
      target="_blank"
      rel="noopener noreferrer"
    >
      {APP_VERSION}
    </a>
  </div>
</footer>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 4px 16px;
    background: var(--color-bg-sidebar);
    border-top: 1px solid var(--color-border);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-text-disabled);
    flex-shrink: 0;
    min-height: 26px;
  }
  .cluster {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .cluster.right {
    gap: 10px;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .sep {
    opacity: 0.4;
  }
  .credit {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 7px;
    border-radius: 999px;
    text-decoration: none;
    color: var(--color-text-disabled);
    border: 1px solid transparent;
    transition:
      color 0.12s,
      background 0.12s,
      border-color 0.12s;
  }
  .credit-name {
    color: var(--color-accent);
    font-weight: 600;
    letter-spacing: 0.04em;
  }
  .credit :global(.credit-ext) {
    opacity: 0.5;
    flex: none;
    transition: opacity 0.12s;
  }
  .credit:hover {
    color: var(--color-text-main);
    background: color-mix(in srgb, var(--color-accent) 8%, transparent);
    border-color: color-mix(in srgb, var(--color-accent) 30%, transparent);
  }
  .credit:hover :global(.credit-ext) {
    opacity: 0.9;
  }
  .version {
    color: var(--color-text-disabled);
    text-decoration: none;
    transition: color 0.12s;
  }
  .version:hover {
    color: var(--color-text-main);
  }
</style>
