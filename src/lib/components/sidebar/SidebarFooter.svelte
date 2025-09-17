<script lang="ts">
	import { PowerOff, Settings, Share2, Github } from '@lucide/svelte';
	import { modalStore } from '$lib/stores/modalStore.svelte';
	import SettingsModal from '$lib/components/modals/SettingsModal.svelte';
</script>

<div class="flex-shrink-0 p-2 pt-4 border-t border-border flex flex-col items-center gap-2">
	

	<a
		href="https://github.com/mika412/robotwhisperer"
		target="_blank"
		rel="noopener noreferrer"
		title="GitHub Repository"
		class="w-full"
	>
		<button
			class="h-10 w-full flex items-center justify-center gap-2.5 rounded-lg bg-bg-main border border-border hover:bg-bg-hover text-text-dimmer hover:text-text-main transition-colors font-semibold"
		>
			<Github size={16} />
			GitHub
		</button>
	</a>
	<div class="flex items-center gap-2 w-full">
		<button
			class="h-10 flex-grow flex items-center justify-center gap-2.5 text-sm px-2.5 rounded-lg font-semibold transition-colors bg-red-500/20 text-red-300 hover:bg-red-500/30"
		>
			<PowerOff size={16} /> Disconnected
		</button>

		<button
			onclick={() => modalStore.open(SettingsModal)}
			title="Settings"
			class="h-10 w-10 flex-shrink-0 flex items-center justify-center rounded-lg bg-bg-main border border-border hover:bg-bg-hover text-text-dimmer hover:text-text-main transition-colors"
		>
			<Settings size={16} />
		</button>
		<button
			title="Import/Export"
			class="h-10 w-10 flex-shrink-0 flex items-center justify-center rounded-lg bg-bg-main border border-border hover:bg-bg-hover text-text-dimmer hover:text-text-main transition-colors"
		>
			<Share2 size={16} />
		</button>
	</div>
</div>