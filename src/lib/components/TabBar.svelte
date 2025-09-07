<script lang="ts">
	import { X } from "@lucide/svelte";
	import {
		openItems,
		activeItemId,
		closeItem,
	} from "$lib/stores/workspaceStore";
	import TypeBadge from "$lib/components/TypeBadge.svelte";

	const handleClose = (e: MouseEvent, id: number | undefined) => {
		e.stopPropagation();
		if (id) {
			closeItem(id);
		}
	};
</script>

<div class="flex border-b border-border bg-bg-sidebar">
	<div class="flex items-center overflow-x-auto">
		<div class="flex space-x-1 p-1">
			{#each $openItems as item (item.id)}
				{@const isActive = $activeItemId === item.id}
				<div
					onclick={() => activeItemId.set(item.id)}
					class="flex-shrink-0 flex items-center gap-2 pl-3 pr-1 py-1.5 rounded-lg cursor-pointer text-sm transition-all duration-150 group {isActive
						? 'bg-bg-main shadow-sm'
						: 'hover:bg-bg-hover'}"
				>
					<TypeBadge type={item.data.type} />
					<span
						class={isActive
							? "font-semibold text-text-main"
							: "text-text-dimmer"}
					>
						{item.data.name}{#if item.isDirty}*{/if}
					</span>
					<button
						onclick={(e) => handleClose(e, item.id)}
						class="ml-2 p-1 rounded-full hover:bg-bg-main"
					>
						<X
							size={14}
							class="text-text-disabled group-hover:text-text-dimmer"
						/>
					</button>
				</div>
			{/each}
		</div>
	</div>
</div>
