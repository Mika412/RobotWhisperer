<script lang="ts">
    import RequestView from "../ui/components/RequestView.svelte";
    import { activeTabId } from "../ui/store";
</script>

{#if $activeTabId}
    <RequestView active={$activeTabId} />
{:else}
    <div class="p-6 text-gray-500">Open or create a request…</div>
{/if}