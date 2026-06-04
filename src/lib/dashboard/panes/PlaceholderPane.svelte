<script lang="ts">
  import { LayoutGrid } from "@lucide/svelte";
  import type { PaneComponentProps } from "$lib/dashboard/registry/paneRegistry";

  let { config: _config, ctx: _ctx }: PaneComponentProps = $props();
</script>

<div class="placeholder">
  <LayoutGrid size={28} />
  <p>Empty pane</p>
  <span>Pick a pane type from the dropdown above to start visualizing a topic.</span>
</div>

<style>
  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 100%;
    padding: 16px;
    text-align: center;
    color: var(--color-text-disabled);
  }
  p {
    font-size: 14px;
    font-weight: 600;
    color: var(--color-text-dimmer);
  }
  span {
    font-size: 12px;
    max-width: 240px;
  }
</style>
