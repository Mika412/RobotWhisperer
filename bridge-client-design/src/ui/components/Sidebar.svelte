<script lang="ts">
    import { onMount } from 'svelte';
    import { db } from "../../lib/db";
    import { nanoid } from "nanoid";
    import type { SavedRequest } from "../../lib/types";
    import { openTabs, activeTabId } from "../store";

    let items: SavedRequest[] = [];

    async function loadDatabase() {
        const allRequests = await db.requests.orderBy('createdAt').toArray();
        items = allRequests;
    }

    // Run database calls only on the client
    onMount(() => {
        loadDatabase();
    });

    async function add(kind: "topic" | "service" | "action") {
        const now = Date.now();
        const req: SavedRequest = {
            id: nanoid(),
            name: "New " + kind,
            kind,
            resourceName: "",
            messageType: "",
            createdAt: now,
            updatedAt: now,
        };
        await db.requests.put(req);
        items.unshift(req); // Or re-run loadDatabase() to respect sorting
        $openTabs = [...$openTabs, req];
        $activeTabId = req.id;
    }

    function openItem(it: SavedRequest) {
        if (!$openTabs.find((t) => t.id === it.id))
            $openTabs = [...$openTabs, it];
        $activeTabId = it.id;
    }
</script>

<div class="border-r h-full flex flex-col">
     <div class="p-3 flex gap-2">
        <button class="btn" onclick={() => add("topic")}>+ Topic</button>
        <button class="btn" onclick={() => add("service")}>+ Service</button>
        <button class="btn" onclick={() => add("action")}>+ Action</button>
    </div>
    <div class="overflow-auto">
        {#each items as it (it.id)}
            <div
                class="px-3 py-2 hover:bg-gray-100 cursor-pointer"
                onclick={() => openItem(it)}
            >
                <div class="text-sm font-medium">{it.name}</div>
                <div class="text-xs text-gray-500">
                    {it.kind} • {it.resourceName || "(unset)"}
                </div>
            </div>
        {/each}
    </div>
</div>

<!-- <style>
    .btn {
        @apply bg-gray-800 text-white text-sm px-2 py-1 rounded;
    }
</style> -->