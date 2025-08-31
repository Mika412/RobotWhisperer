<script lang="ts">
    import { Sun, Moon } from "@lucide/svelte";
    import { modalStore } from "$lib/stores/modalStore.svelte";
    import { settings } from "$lib/stores/settingsStore.svelte";
    const themes = {
        dark: { name: "Dark", icon: Moon },
        light: { name: "Light", icon: Sun },
        "one-dark": { name: "One Dark", icon: Moon },
        dracula: { name: "Dracula", icon: Moon },
        nord: { name: "Nord", icon: Moon },
        "solarized-light": { name: "Solarized Light", icon: Sun },
        "rose-pine-dawn": { name: "Rosé Pine Dawn", icon: Sun },
    };
</script>

<div class="p-6">
    <h2 class="text-xl font-bold mb-6 text-text-main">Settings</h2>
    <div class="space-y-6">
        <div>
            <label
                for="theme-select"
                class="block text-sm font-medium text-text-dimmer mb-2"
                >Theme</label
            >
            <select
                id="theme-select"
                bind:value={settings.theme}
                class="w-full bg-bg-input border border-border rounded-lg px-3 py-2 text-text-main focus:outline-none focus:ring-2 focus:ring-accent"
            >
                {#each Object.entries(themes) as [key, { name }]}
                    <option value={key}>{name}</option>
                {/each}
            </select>
        </div>
        <div>
            <label
                for="ws-url"
                class="block text-sm font-medium text-text-dimmer mb-2"
                >WebSocket URL</label
            >
            <input
                id="ws-url"
                type="text"
                class="w-full bg-bg-input border border-border rounded-lg px-3 py-2 text-text-main focus:outline-none focus:ring-2 focus:ring-accent"
            />
        </div>
    </div>
    <div class="mt-8 flex justify-end">
        <button
            onclick={modalStore.close}
            class="text-white px-6 py-2 rounded-lg font-semibold transition-colors bg-accent hover:bg-accent-dark"
        >
            Done
        </button>
    </div>
</div>
