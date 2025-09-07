<!-- <script lang="ts">
	import "../app.css"; // Import your global styles
	import Sidebar from "../ui/components/Sidebar.svelte";
	import Tabs from "../ui/components/Tabs.svelte";
	import RequestView from "../ui/components/RequestView.svelte";
	import { openTabs, activeTabId } from "../ui/store";

	$: tabs = $openTabs;
	$: active = $activeTabId;
</script>

<div class="h-screen grid grid-cols-[300px_1fr]">
	<Sidebar />
	<div class="flex flex-col h-full">
		<Tabs />
		{#if active}
			<RequestView {active} />
		{:else}
			<div class="p-6 text-gray-500">Open or create a request…</div>
		{/if}
	</div>
</div> -->

<!-- <script lang="ts">
	import "../app.css";
    import Sidebar from "../ui/components/Sidebar.svelte";
    import Tabs from "../ui/components/Tabs.svelte";
</script>

<div class="h-screen grid grid-cols-[300px_1fr]">
    <Sidebar />
    <div class="flex flex-col h-full">
        <Tabs />
        <main class="overflow-auto h-full">
            <slot />
        </main>
    </div>
</div> -->
<script lang="ts">
  import "../app.css";

    import Sidebar from "../ui/components/Sidebar.svelte";
    import Tabs from "../ui/components/Tabs.svelte";
    import RequestView from "../ui/components/RequestView.svelte";
    import { openTabs, activeTabId } from "../ui/store";
    // import LogDecoder from "./ui/LogDecoder.svelte";
    $: tabs = $openTabs;
    $: active = $activeTabId;
</script>

<div class="h-screen grid grid-cols-[300px_1fr]">
    <Sidebar />
    <div class="flex flex-col h-full">
        <Tabs />
        {#if active}
            <RequestView {active} />
        {:else}
            <div class="p-6 text-gray-500">Open or create a request…</div>
        {/if}
    </div>
    <!-- <LogDecoder/> -->
</div>
