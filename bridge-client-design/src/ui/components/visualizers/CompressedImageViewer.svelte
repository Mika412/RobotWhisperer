<script lang="ts">
    import { latest } from "../../store";
    export let resourceName: string;
    let imgEl: HTMLImageElement;
    $: payload = $latest[resourceName]?.data;
    $: if (imgEl && payload?.format && payload?.data) {
        const blob = new Blob([new Uint8Array(payload.data)], {
            type: payload.format.includes("jpeg") ? "image/jpeg" : "image/png",
        });
        const url = URL.createObjectURL(blob);
        imgEl.src = url;
    }
</script>

<img bind:this={imgEl} class="max-w-full" />
