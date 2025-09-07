<script lang="ts">
    import { latest } from "../../store";
    export let resourceName: string;
    let canvas: HTMLCanvasElement;
    $: data = $latest[resourceName]?.data;
    $: if (canvas && data && data.data && data.height && data.width) {
        // Assume raw 8UC3 RGB — adapt to encoding
        const { width, height, data: bytes } = data;
        const ctx = canvas.getContext("2d")!;
        const img = ctx.createImageData(width, height);
        // naive BGR/RGB – adapt as needed
        for (
            let i = 0, j = 0;
            i < bytes.length && j < img.data.length;
            i += 3, j += 4
        ) {
            img.data[j] = bytes[i];
            img.data[j + 1] = bytes[i + 1];
            img.data[j + 2] = bytes[i + 2];
            img.data[j + 3] = 255;
        }
        ctx.putImageData(img, 0, 0);
    }
</script>

<canvas
    bind:this={canvas}
    width={data?.width || 640}
    height={data?.height || 480}
    class="border"
/>
