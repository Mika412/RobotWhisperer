<script lang="ts">
    import { Plus } from "@lucide/svelte";
    import {
        requests,
        addRequest,
    } from "$lib/stores/requestStore";
    import RequestItem from "./RequestItem.svelte";
    import type { RosRequest } from "$lib/db";

    const handleAddRequest = () => {
        addRequest();
    };

    const handleOpenTab = (request: RosRequest) => {
        if (request.id) {
            // openTab(request.id);
        }
    };

    const typeStyles: Record<string, string> = {
        topic: "text-blue-400",
        service: "text-purple-400",
        action: "text-green-400",
    };
</script>

<div class="px-2">
    <div class="mb-2 flex items-center justify-between px-2">
        <h2
            class="text-sm font-semibold uppercase tracking-wider text-gray-400"
        >
            Requests
        </h2>
        <button
            on:click={handleAddRequest}
            class="rounded p-1 text-gray-400 hover:bg-gray-700 hover:text-white"
            title="Create new request"
        >
            <Plus class="h-5 w-5" />
        </button>
    </div>

    <ul class="space-y-1">
        {#each $requests as request (request.id)}
            <RequestItem {request} />
        {:else}
            <li class="px-2 py-4 text-center text-xs text-gray-500">
                No requests yet.
            </li>
        {/each}
    </ul>
</div>
