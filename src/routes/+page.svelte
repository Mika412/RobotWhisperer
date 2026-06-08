<script lang="ts">
  import TopTabBar from "$lib/components/shell/TopTabBar.svelte";
  import AppSidebar from "$lib/components/shell/AppSidebar.svelte";
  import MainView from "$lib/components/shell/MainView.svelte";
  import StatusBar from "$lib/components/shell/StatusBar.svelte";
  import { fullscreenStore } from "$lib/stores/fullscreenStore.svelte";

  const fullscreen = $derived(fullscreenStore.isFullscreen);
</script>

<div class="app-shell">
  {#if !fullscreen}
    <TopTabBar />
  {/if}
  <div class="app-body">
    {#if !fullscreen}
      <AppSidebar />
    {/if}
    <MainView />
  </div>
  <StatusBar />
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    color: var(--color-text-main);
    background: var(--color-bg-deep);
  }
  .app-body {
    flex: 1;
    display: flex;
    min-height: 0;
    min-width: 0;
  }
</style>
