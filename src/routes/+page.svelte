<script lang="ts">
    import MainScreen from "$lib/components/MainScreen.svelte";
    import Sidebar from "$lib/components/sidebar/Sidebar.svelte";
    import WelcomeScreen from "$lib/components/WelcomeScreen.svelte";
    import { openItems } from "$lib/stores/workspaceStore";
</script>

<main
    class="flex h-screen w-full overflow-hidden text-text-main font-sans antialiased"
>
    <div class="flex-none">
        <Sidebar />
    </div>
    <main class="flex-1 min-w-0">
        {#if $openItems.length === 0}
            <WelcomeScreen />
        {:else}
            <MainScreen />
        {/if}
    </main>
</main>
