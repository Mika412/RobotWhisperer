<script lang="ts">
    import SidebarHeader from "./SidebarHeader.svelte";
    import SidebarFooter from "./SidebarFooter.svelte";
    import TreeView from "$lib/components/TreeView/TreeView.svelte";
</script>

<div class="flex flex-col w-80 bg-bg-sidebar p-2 border-r border-border">
    <SidebarHeader />

    <div class="flex-grow overflow-y-auto">
        <TreeView />
    </div>

    <SidebarFooter />
</div>
