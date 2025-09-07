<script lang="ts">
    import { openTabs, activeTabId } from "../store";
    function close(id: string) {
        $openTabs = $openTabs.filter((t) => t.id !== id);
        if ($activeTabId === id) $activeTabId = $openTabs.at(-1)?.id ?? null;
    }
</script>

<div class="border-b flex gap-1 px-2 h-10 items-center">
    {#each $openTabs as t}
        <div
            class="px-3 py-1 rounded-t border-b-2"
            class:border-b-gray-800={t.id === $activeTabId}
            on:click={() => ($activeTabId = t.id)}
        >
            {t.name}
            <button
                class="ml-2 text-xs"
                on:click|stopPropagation={() => close(t.id)}>×</button
            >
        </div>
    {/each}
</div>
