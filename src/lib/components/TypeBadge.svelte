<script lang="ts">
	let { type }: { type?: string } = $props();

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

	function getBadgeLabel(tag?: string) {
		switch (tag) {
			case 'topic':
				return 'TOP';
			case 'service':
				return 'SER';
			case 'action':
				return 'ACT';
			default:
				return 'GEN'; // A fallback for any other type
		}
	}
	
</script>

{#if type}
	<span
		class="text-[10px] font-bold px-1.5 py-0.5 rounded-md border {getBadgeClass(type)}"
	>
		{getBadgeLabel(type)}
	</span>
{/if}
