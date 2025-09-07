<script lang="ts">
    import { latest } from "../store";
    export let resourceName: string;
    export let messageType: string;
    $: msg = $latest[resourceName]?.data;
</script>

{#if msg}
    <div class="text-sm">
        {#each Object.entries(msg) as [k, v]}
            <div class="py-0.5">
                <span class="font-mono text-gray-600">{k}</span>:
                <span
                    >{typeof v === "object"
                        ? JSON.stringify(v)
                        : String(v)}</span
                >
            </div>
        {/each}
    </div>
{:else}
    <div class="text-gray-500 text-sm">No data</div>
{/if}
