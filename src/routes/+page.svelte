<script lang="ts">
    import Sidebar from "$lib/components/sidebar/Sidebar.svelte";
    import WelcomeScreen from '$lib/components/WelcomeScreen.svelte';
</script>

<main class="flex min-h-screen w-full text-text-main font-sans antialiased">
    <Sidebar />

    <main class="flex-1">
		<WelcomeScreen />
	</main>
</main>
