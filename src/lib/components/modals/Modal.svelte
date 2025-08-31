<!-- <script>
    import { modalStore } from '$lib/stores/modalStore.svelte';
	import { onMount, onDestroy } from 'svelte';

    let { component, props } = $derived(modalStore);

	function handleKeydown(event) {
		if (event.key === 'Escape') {
			modalStore.close();
		}
	}

	onMount(() => {
		window.addEventListener('keydown', handleKeydown);
	});

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown);
	});
</script>

{#if component}
    <div
        class="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-4"
        onclick={modalStore.close}
        role="dialog"
        aria-modal="true"
    >
        <div
            class="bg-gray-800 border border-gray-700 rounded-xl shadow-xl w-full max-w-md"
            onclick={e => e.stopPropagation()}
        >
            <svelte:component this={component} {...props} />
        </div>
    </div>
{/if} -->
<script lang="ts">
    import { modalStore } from '$lib/stores/modalStore.svelte';
	import { onMount, onDestroy } from 'svelte';
	let { component, props } = $derived(modalStore);

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			modalStore.close();
		}
	}

	function handleOverlayKeydown(event: KeyboardEvent) {
		// Close on Enter or Space for the overlay
		if (event.key === 'Enter' || event.key === ' ') {
			modalStore.close();
		}
	}

	onMount(() => {
		window.addEventListener('keydown', handleKeydown);
	});

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown);
	});
</script>

{#if component}
    <div
        class="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-4"
        onclick={modalStore.close}
        onkeydown={handleOverlayKeydown}
        role="dialog"
        aria-modal="true"
        tabindex="-1"
    >
        <div
            class="bg-bg-main border border-border rounded-xl shadow-xl w-full max-w-md"
            onclick={e => e.stopPropagation()}
            onkeydown={handleOverlayKeydown}
            role="dialog"
            tabindex="-1"
        >
            {@render component({ ...props })}
        </div>
    </div>
{/if}
