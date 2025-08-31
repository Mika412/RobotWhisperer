<script lang="ts">
    import type { RosRequest } from "$lib/db";
    import { Plus, Trash2 } from "@lucide/svelte";
    import { requests, deleteRequest } from '$lib/stores/requestStore';
    let { request }: { request: RosRequest } = $props();

    // function getBadgeClass(tag?: string) {
    // 	switch (tag) {
    // 		case 'topic':
    // 			return 'badge-topic';
    // 		case 'service':
    // 			return 'badge-service';
    // 		case 'action':
    // 			return 'badge-action';
    // 		default:
    // 			return 'bg-gray-500/10 text-gray-400 border-gray-500/30';
    // 	}
    // }
    function getBadgeClass(tag?: string) {
        switch (tag) {
            case "topic":
                return "text-blue-400 border-blue-500/30 bg-blue-500/10";
            case "service":
                return "text-green-400 border-green-500/30 bg-green-500/10";
            case "action":
                return "text-purple-400 border-purple-500/30 bg-purple-500/10";
            default:
                return "text-gray-400 border-gray-500/30 bg-gray-500/10";
        }
    }

    const handleDelete = (e: MouseEvent, id: number | undefined) => {
		e.stopPropagation();
		if (id) {
			deleteRequest(id);
		}
	};
</script>

<div
    class="flex items-center gap-2 pl-2 pr-1 py-1.5 rounded-lg cursor-pointer text-sm transition-colors group hover:bg-bg-hover"
>
    {#if request.type}
        <span
            class="uppercase text-[10px] font-bold px-1.5 py-0.5 rounded-md border {getBadgeClass(
                request.type,
            )}"
        >
            {request.type.slice(0, 4)}
        </span>
    {/if}
    <span class="flex-grow truncate">{request.name}</span>
    <!-- This is temporary button, we should remove it later -->
    <button
        onclick={(e) => handleDelete(e, request.id)}
        class="invisible rounded p-1 text-gray-500 group-hover:visible hover:bg-gray-600 hover:text-red-400"
        title="Delete request"
    >
        <Trash2 class="h-4 w-4" />
    </button>
</div>
