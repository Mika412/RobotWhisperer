<script lang="ts">
    import "../app.css";

    import { onMount } from 'svelte';
	import { loadDatabase } from '$lib/stores/requestStore';
	import Modal from '$lib/components/modals/Modal.svelte';

	onMount(() => {
		loadDatabase();
	});

    let { children } = $props();
</script>

<Modal />

{@render children()}
