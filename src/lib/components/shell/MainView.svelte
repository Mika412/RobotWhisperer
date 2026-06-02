<script lang="ts">
  import { tabsStore } from "$lib/stores/tabsStore.svelte";
  import RequestEditor from "$lib/components/RequestEditor.svelte";
  import DashboardView from "$lib/components/shell/DashboardView.svelte";
  import WelcomeScreen from "$lib/components/WelcomeScreen.svelte";
</script>

{#if !tabsStore.active}
  <WelcomeScreen />
{:else}
  <div class="main-view">
    {#each tabsStore.tabs as tab (tab.tabId)}
      <div class="page" class:hidden={tabsStore.activeTabId !== tab.tabId}>
        {#if tab.kind === "request"}
          <RequestEditor requestId={tab.requestId} />
        {:else}
          <DashboardView dashboardId={tab.dashboardId} />
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .main-view {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    background: var(--color-bg-deep);
  }
  .page {
    position: absolute;
    inset: 0;
    display: flex;
    min-height: 0;
    min-width: 0;
    overflow: auto;
  }
  .hidden {
    display: none;
  }
</style>
