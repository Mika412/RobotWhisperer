<script lang="ts">
  import { X, Plus, LayoutGrid, Inbox } from "@lucide/svelte";
  import { tabsStore } from "$lib/stores/tabsStore.svelte";
  import { dashboardStore } from "$lib/stores/dashboardStore.svelte";
  import { requestsStore } from "$lib/stores/requestsStore.svelte";
  import { subscriptionStore } from "$lib/stores/subscriptionStore.svelte";
  import TypeBadge from "$lib/components/TypeBadge.svelte";
  import AnimatedBot from "$lib/components/AnimatedBot.svelte";
  import Popover from "$lib/components/common/Popover.svelte";

  let strip: HTMLDivElement;
  let addOpen = $state(false);

  function close(event: MouseEvent, tabId: string) {
    event.stopPropagation();
    tabsStore.close(tabId);
  }

  function onWheel(event: WheelEvent) {
    if (!strip) return;
    event.preventDefault();
    strip.scrollLeft += event.deltaY;
  }

  async function addRequest() {
    addOpen = false;
    const request = await requestsStore.create();
    tabsStore.openRequest(request.id);
  }

  function addDashboard() {
    addOpen = false;
    const dashboard = dashboardStore.create();
    tabsStore.openDashboard(dashboard.id);
  }
</script>

<div class="top-tab-bar">
  <div class="brand">
    <AnimatedBot size={24} strokeWidth={2.2} />
    <span>Robot Whisperer</span>
  </div>

  <div class="tab-strip scrollbar-custom-overlay-x" bind:this={strip} onwheel={onWheel}>
    {#each tabsStore.tabs as tab (tab.tabId)}
      {@const active = tabsStore.activeTabId === tab.tabId}
      <div
        class="tab-chip"
        class:is-active={active}
        role="tab"
        tabindex="0"
        aria-selected={active}
        onclick={() => tabsStore.setActive(tab.tabId)}
        onkeydown={(event) => event.key === "Enter" && tabsStore.setActive(tab.tabId)}
      >
        {#if tab.kind === "request"}
          <TypeBadge kind={tab.draft.kind} />
          <span class="tab-name">{tab.draft.name}{tab.dirty ? "*" : ""}</span>
          {#if subscriptionStore.isActive(tab.requestId)}
            <span class="status-dot running"></span>
          {/if}
        {:else}
          <LayoutGrid size={15} class="tab-icon" />
          <span class="tab-name">{dashboardStore.get(tab.dashboardId)?.title ?? "Dashboard"}</span>
        {/if}
        <button
          class="tab-close"
          title="Close tab"
          aria-label="Close tab"
          onclick={(event) => close(event, tab.tabId)}
        >
          <X size={14} />
        </button>
      </div>
    {/each}

    <Popover bind:open={addOpen} align="start">
      {#snippet trigger({ toggle })}
        <button class="tab-add" title="New tab" aria-label="New tab" onclick={toggle}>
          <Plus size={14} />
        </button>
      {/snippet}
      <div class="add-menu" role="menu">
        <button class="add-item" role="menuitem" onclick={addRequest}>
          <Inbox size={15} /> New request
        </button>
        <button class="add-item" role="menuitem" onclick={addDashboard}>
          <LayoutGrid size={15} /> New dashboard
        </button>
      </div>
    </Popover>
  </div>
</div>

<style>
  .top-tab-bar {
    display: flex;
    align-items: stretch;
    height: 44px;
    background: var(--color-bg-sidebar);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
    min-width: 0;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 240px;
    padding-left: 14px;
    color: var(--color-accent);
    font-weight: 800;
    font-size: 16px;
    flex-shrink: 0;
    border-right: 1px solid var(--color-border);
    box-sizing: border-box;
    user-select: none;
  }
  .brand span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tab-strip {
    flex: 1;
    min-width: 0;
    display: flex;
    gap: 4px;
    padding: 0 6px;
    overflow-x: auto;
    align-items: center;
  }
  .tab-strip::-webkit-scrollbar {
    height: 4px;
  }
  .tab-name {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  :global(.tab-icon) {
    color: var(--color-accent);
    flex: none;
  }
  .tab-close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    margin-left: 2px;
    color: var(--color-text-disabled);
    background: transparent;
    border: 0;
    border-radius: 4px;
    cursor: pointer;
  }
  .tab-close:hover {
    background: var(--color-bg-hover);
    color: var(--color-text-main);
  }
  .tab-strip :global(.popover) {
    flex: none;
    margin-left: 2px;
  }
  .tab-add {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    color: var(--color-text-dimmer);
    background: transparent;
    border: 0;
    border-radius: 6px;
    cursor: pointer;
    flex: none;
  }
  .tab-add:hover {
    background: var(--color-bg-hover);
    color: var(--color-accent);
  }
  .add-menu {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 168px;
  }
  .add-item {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 7px 10px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--color-text-main);
    font-size: 13px;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }
  .add-item:hover {
    background: var(--color-bg-hover);
    color: var(--color-accent);
  }
</style>
