<!-- <script lang="ts">
    import { Bot } from "@lucide/svelte";
    import { requests, addRequest, deleteRequest } from '$lib/stores/requestStore';
    import type { RosRequest } from '$lib/db';

   	const handleAddRequest = () => {
		addRequest('topic');
	};
</script>

<div class="flex-shrink-0 p-2 pb-4 border-b border-border">
    <div class="flex items-center gap-2">
        <Bot size={28} class="text-accent" strokeWidth={2} />
        <h1
            class="text-xl font-bold text-text-main tracking-tight leading-tight"
        >
            Robot Whisperer
        </h1>
    </div>
    <div class="flex items-center gap-2"></div>
</div> -->


<script lang="ts">
	import { Bot, FilePlus, FolderPlus } from '@lucide/svelte';
	import { requests, addRequest } from '$lib/stores/requestStore';
</script>

<div class="flex-shrink-0 p-2 border-b border-border mb-2">
	<div class="flex items-center gap-2 mb-4">
		<Bot size={28} class="text-accent" strokeWidth={2} />
		<h1 class="text-xl font-bold text-text-main tracking-tight">Robot Whisperer</h1>
	</div>
	<!-- <div class="flex items-center gap-2">
		<button
			class="flex-1 flex items-center justify-center gap-2 bg-accent text-white px-3 py-2 rounded-lg hover:bg-accent-dark text-sm font-semibold transition-colors"
			onclick={() => addRequest()}
		>
			<FilePlus size={16} /> New Request
		</button>
		<button
			title="New Collection"
			class="p-2 bg-bg-hover rounded-lg hover:bg-bg-main border border-border"
			onclick={() => addCollection()}
		>
			<FolderPlus size={16} class="text-text-dimmer" />
		</button>
	</div> -->
</div>
