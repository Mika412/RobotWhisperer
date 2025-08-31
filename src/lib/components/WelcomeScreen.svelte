<script lang="ts">
    import { Bot, FilePlus } from "@lucide/svelte";

    import { requests, addRequest } from '$lib/stores/requestStore';
</script>

<div
    class="flex flex-col items-center justify-center h-full text-text-disabled bg-bg-deep p-8"
>
    <div class="text-center">
        <Bot size={64} class="mx-auto text-accent mb-4" strokeWidth={1.5} />
        <h2 class="text-2xl font-bold text-text-main mb-2">
            Welcome to the Robot Whisperer
        </h2>
        <p class="max-w-md mx-auto mb-6 text-text-dimmer">
            Your friendly neighborhood tool for interacting with the ROS 2
            graph. Select a request from the sidebar or create a new one to
            begin.
        </p>
        <button
            class="bg-accent text-white px-6 py-3 rounded-lg hover:bg-accent-dark text-sm font-semibold flex items-center gap-2 mx-auto transition-colors"
            onclick={() => addRequest()}
        >
            <FilePlus size={16} /> Create Your First Request
        </button>
    </div>
</div>
