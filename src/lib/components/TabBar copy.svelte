<script lang="ts">
	import { X } from '@lucide/svelte';
	import { openTabIds, activeTabId, closeTab } from '$lib/stores/tabStore';
	import { requests } from '$lib/stores/requestStore';

	// Reactive statement to derive the full tab objects from our two stores
	$: openTabs = $openTabIds.map((id) => $requests.find((req) => req.id === id)).filter(Boolean);
	const typeStyles: Record<string, string> = {
		topic: 'text-blue-400',
		service: 'text-purple-400',
		action: 'text-green-400'
	};
	function getBadgeClass(tag?: string) {
		switch (tag) {
			case 'topic':
				return 'badge-topic';
			case 'service':
				return 'badge-service';
			case 'action':
				return 'badge-action';
			default:
				return 'bg-gray-500/10 text-gray-400 border-gray-500/30';
		}
	}
</script>

<div class="flex border-b border-border bg-bg-sidebar">
	<div class="flex items-center overflow-x-auto">
		<div class="flex space-x-1 p-1">
			{#each openTabs as tab (tab.id)}
				{@const isActive = $activeTabId === tab.id}
				{@const badgeClass = getBadgeClass(tab.tag)}

				<div
					on:click={() => activeTabId.set(tab.id)}
					class="flex-shrink-0 flex items-center gap-2 pl-3 pr-1 py-1.5 rounded-lg cursor-pointer text-sm transition-all duration-150 group {isActive ?
'bg-bg-main shadow-sm' : 'hover:bg-bg-hover'}"
				>
					<span class="uppercase text-xs font-bold px-1.5 py-0.5 rounded-md border {badgeClass}"
						>{(tab.type ?? 'item').slice(0, 4)}</span
					>
					<span class={isActive ? 'font-semibold text-text-main' : 'text-text-dimmer'}
						>{tab.name}</span
					>
					<button on:click={() => closeTab(tab.id)} class="ml-2 p-1 rounded-full hover:bg-bg-main">
						<X size={14} class="text-text-disabled group-hover:text-text-dimmer" />
					</button>
				</div>
			{/each}
		</div>
	</div>
</div>
