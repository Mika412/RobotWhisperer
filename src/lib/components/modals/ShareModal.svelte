<script>
    import { Share2 } from "@lucide/svelte";
    import { modalStore } from "$lib/stores/modalStore.svelte";
</script>

<div class="p-6">
    <div class="flex items-center mb-4">
        <Share2 class="text-gray-400 mr-3" size={20} />
        <h2 class="text-xl font-bold text-white">Import / Export Data</h2>
    </div>
    <p class="text-gray-400 mb-6">You can import or export your application data as a JSON file.</p>
    <div class="flex gap-4">
        <button class="flex-1 px-6 py-2 rounded-lg bg-green-600 hover:bg-green-700 transition-colors text-white font-semibold">
            Import
        </button>
        <button class="flex-1 px-6 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 transition-colors text-white font-semibold">
            Export
        </button>
    </div>
     <div class="mt-6 text-right">
        <button onclick={modalStore.close} class="text-sm text-gray-400 hover:text-white">
            Close
        </button>
    </div>
</div>
