<script lang="ts">
    // import ImageViewer from "./visualizers/ImageViewer.svelte";
    // import CompressedImageViewer from "./visualizers/CompressedImageViewer.svelte";
    // import PointCloudViewer from "./visualizers/PointCloudViewer.svelte";
    export let messageType: string;
    export let resourceName: string;

    $: comp = pick(messageType);
    function pick(t: string) {
        if (!t) return null;
        // if (t.endsWith("/CompressedImage")) return CompressedImageViewer;
        // if (t.endsWith("/Image")) return ImageViewer;
        // if (t.endsWith("/PointCloud2")) return PointCloudViewer;
        return null;
    }
</script>

{#if comp}
    <svelte:component this={comp} {resourceName} />
{:else}
    <div class="text-gray-500 text-sm">No visualizer for {messageType}</div>
{/if}
