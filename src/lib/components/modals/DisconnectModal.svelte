<script>
    import { PowerOff } from "@lucide/svelte";
    import { modalStore } from "$lib/stores/modalStore.svelte";
</script>

<div class="p-6 text-center">
    <div class="flex justify-center mb-4">
        <div class="w-12 h-12 rounded-full bg-red-500/20 flex items-center justify-center">
            <PowerOff class="text-red-300" size={24} />
        </div>
    </div>
    <h3 class="text-lg font-bold text-white mb-2">Disconnect</h3>
    <p class="text-gray-400 mb-6">Are you sure you want to disconnect? This will terminate your current session.</p>
    <div class="flex justify-center gap-4">
        <button onclick={modalStore.close} class="px-6 py-2 rounded-lg bg-gray-700 hover:bg-gray-600 transition-colors text-white font-semibold">
            Cancel
        </button>
        <button onclick={() => { /* Add disconnect logic here */ alert('Disconnected!'); modalStore.close(); }} class="px-6 py-2 rounded-lg bg-red-500 hover:bg-red-600 transition-colors text-white font-semibold">
            Disconnect
        </button>
    </div>
</div>
