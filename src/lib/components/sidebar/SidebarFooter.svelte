<script lang="ts">
    import { PowerOff, Settings, Share2 } from "@lucide/svelte";
</script>

<div
    class="flex-shrink-0 p-2 pt-4 border-t border-border flex items-center gap-2"
>
    <button
        class="h-10 flex-grow flex items-center justify-center gap-2.5 text-sm px-2.5 rounded-lg font-semibold transition-colors bg-red-500/20 text-red-300 hover:bg-red-500/30"
    >
        <PowerOff size={16} /> Disconnected
    </button>

    <button
        title="Settings"
        class="h-10 w-10 flex items-center justify-center rounded-lg bg-bg-main border border-border hover:bg-bg-hover text-text-dimmer hover:text-text-main transition-colors"
    >
        <Settings size={16} />
    </button>
    <button
        title="Import/Export"
        class="h-10 w-10 flex items-center justify-center rounded-lg bg-bg-main border border-border hover:bg-bg-hover text-text-dimmer hover:text-text-main transition-colors"
    >
        <Share2 size={16} />
    </button>
</div>
