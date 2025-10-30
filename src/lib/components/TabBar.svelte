<script lang="ts">
	import { X } from "@lucide/svelte";
	import {
		openItems,
		activeItemId,
		closeItem,
	} from "$lib/stores/workspaceStore";
	import TypeBadge from "$lib/components/TypeBadge.svelte";

	let tabBarElement: HTMLDivElement;

	const handleClose = (e: MouseEvent, id: number | undefined) => {
		e.stopPropagation();
		if (id) {
			closeItem(id);
		}
	};

	const handleWheel = (e: WheelEvent) => {
		if (tabBarElement) {
			e.preventDefault();
			tabBarElement.scrollLeft += e.deltaY;
		}
	};
</script>

<div
	bind:this={tabBarElement}
	onwheel={handleWheel}
	class="w-full border-b border-border bg-bg-sidebar overflow-x-auto scrollbar-custom pb-2"
>
	<div class="inline-flex items-center min-w-max">
		<div class="flex space-x-1 pt-1 px-1">
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
