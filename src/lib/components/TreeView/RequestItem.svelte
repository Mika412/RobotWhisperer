<script lang="ts">
	import type { RosRequest } from '$lib/db';
	import { Trash2 } from '@lucide/svelte';
	import { deleteRequest } from '$lib/stores/requestStore';

	let { request }: { request: RosRequest } = $props();
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
	<span class="flex-grow truncate font-medium">{request.name}</span>
	<button
		onclick={(e) => handleDelete(e, request.id)}
		class="invisible rounded p-1 text-gray-500 group-hover:visible hover:bg-gray-600 hover:text-red-400"
		title="Delete request"
	>
		<Trash2 class="h-4 w-4" />
	</button>
</div>
