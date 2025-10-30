<script lang="ts">
    import SidebarHeader from "./SidebarHeader.svelte";
    import SidebarFooter from "./SidebarFooter.svelte";
    import TreeView from "$lib/components/TreeView/TreeView.svelte";
</script>

<div class="h-dvh flex flex-col w-80 bg-bg-sidebar p-2 border-r border-border">
    <SidebarHeader />

    <div class="flex-grow flex-nowrap overflow-y-auto scrollbar-custom">
        <TreeView />
    </div>
    <div class="flex-none">
        <SidebarFooter />
    </div>
</div>
