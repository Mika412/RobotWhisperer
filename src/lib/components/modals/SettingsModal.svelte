<script lang="ts">
	import { Sun, Moon } from '@lucide/svelte';
	import { modalStore } from '$lib/stores/modalStore.svelte';
	import { settings } from '$lib/stores/settingsStore.svelte';
	import Dropdown from '$lib/components/common/Dropdown.svelte';

	const themes = {
		dark: { name: 'Dark', icon: Moon },
		light: { name: 'Light', icon: Sun },
		'one-dark': { name: 'One Dark', icon: Moon },
		dracula: { name: 'Dracula', icon: Moon },
		nord: { name: 'Nord', icon: Moon },
		'solarized-light': { name: 'Solarized Light', icon: Sun },
		'rose-pine-dawn': { name: 'Rosé Pine Dawn', icon: Sun }
	};

	const themeOptions = Object.entries(themes).map(([key, { name }]) => ({
		key,
		name
	}));
</script>

<div class="p-6">
	<h2 class="mb-6 text-xl font-bold text-text-main">Settings</h2>
	<div class="space-y-6">
		<div>
			<label for="theme-select" class="mb-2 block text-sm font-medium text-text-dimmer"
				>Theme</label
			>
			<Dropdown options={themeOptions} bind:selected={settings.theme} />
		</div>
		<div>
			<label for="ws-url" class="mb-2 block text-sm font-medium text-text-dimmer"
				>WebSocket URL</label
			>
			<input
				id="ws-url"
				type="text"
				class="h-11 w-full rounded-lg border border-border bg-bg-input px-3 py-2 text-text-main focus:outline-none focus:ring-2 focus:ring-accent"
			/>
		</div>
	</div>
	<div class="mt-8 flex justify-end">
		<button
			onclick={modalStore.close}
			class="rounded-lg bg-accent px-6 py-2 font-semibold text-white transition-colors hover:bg-accent-dark"
		>
			Done
		</button>
	</div>
</div>