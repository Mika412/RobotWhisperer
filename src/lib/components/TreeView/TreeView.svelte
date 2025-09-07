<script lang="ts">
    import { Plus, Inbox } from "@lucide/svelte";
    import { requests, addRequest } from "$lib/stores/requestStore";
    import RequestItem from "./RequestItem.svelte";
    import type { RosRequest } from "$lib/db";

    const handleOpenTab = (request: RosRequest) => {
        if (request.id) {
            // openTab(request.id);
        }
    };
</script>

<div class="px-2">
    <div class="mb-2 flex items-center justify-between px-2">
        <h2
            class="text-sm font-semibold uppercase tracking-wider text-text-dimmer"
        >
            Requests
        </h2>
        <button
            onclick={addRequest}
            class="rounded p-1 text-text-dimmer hover:bg-bg-hover hover:text-text-main transition-colors"
            title="Create new request"
        >
            <Plus class="h-5 w-5" />
        </button>
    </div>

    <ul class="space-y-1">
        {#each $requests as request (request.id)}
            <RequestItem {request} />
        {:else}
            <div class="text-center py-8 px-4 text-text-disabled">
                <Inbox class="mx-auto h-10 w-10 mb-2" />
                <p class="font-semibold text-sm text-text-main mb-2">
                    No Requests Yet
                </p>
                <p class="text-xs">
                    Click the <kbd
                        class="px-1.5 py-0.5 text-xs rounded bg-bg-main border border-border"
                        >+</kbd
                    > button above to create one.
                </p>
            </div>
        {/each}
    </ul>
</div>
