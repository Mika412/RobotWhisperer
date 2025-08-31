<script lang="ts">
    import "../app.css";

    import { onMount } from 'svelte';
	import { loadDatabase } from '$lib/stores/requestStore';

	onMount(() => {
		loadDatabase();
	});

    let { children } = $props();
</script>

{@render children()}
