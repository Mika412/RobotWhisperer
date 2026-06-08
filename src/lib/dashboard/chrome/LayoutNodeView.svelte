<script lang="ts">
  import { dashboardState } from "$lib/stores/dashboardStore.svelte";
  import PaneHost from "$lib/dashboard/chrome/PaneHost.svelte";
  import SplitView from "$lib/dashboard/chrome/SplitView.svelte";
  import GroupView from "$lib/dashboard/chrome/GroupView.svelte";

  let { layoutId, nodeId }: { layoutId: string; nodeId: string } = $props();

  const node = $derived(dashboardState.layouts[layoutId]?.nodes[nodeId]);
</script>

{#if node}
  {#if node.kind === "pane"}
    <PaneHost {layoutId} pane={node} />
  {:else if node.kind === "split"}
    <SplitView {layoutId} {node} />
  {:else if node.kind === "group"}
    <GroupView {layoutId} {node} />
  {/if}
{/if}
