<script lang="ts">
  import { createPaneContext } from "$lib/dashboard/registry/paneContext.svelte";
  import PaneBody from "$lib/dashboard/chrome/PaneBody.svelte";
  import type { PaneNode } from "$lib/dashboard/layout/layout";

  let { layoutId, pane, active }: { layoutId: string; pane: PaneNode; active: boolean } = $props();

  const ctx = createPaneContext(
    () => layoutId,
    () => pane,
  );
</script>

<div class="grouped-pane" class:hidden={!active}>
  <PaneBody {pane} {ctx} />
</div>

<style>
  .grouped-pane {
    position: absolute;
    inset: 0;
    display: flex;
    min-height: 0;
    min-width: 0;
  }
  .hidden {
    display: none;
  }
</style>
