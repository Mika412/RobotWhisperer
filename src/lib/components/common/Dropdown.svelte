<script lang="ts">
  import { ChevronDown } from "@lucide/svelte";
  import { slide } from "svelte/transition";
  import { onMount } from "svelte";

  export let options: { key: string; name: string }[];
  export let selected: string;
  export let placeholder = "Select an option";

  let isOpen = false;
  let node: HTMLElement;

  function handleSelect(key: string) {
    selected = key;
    isOpen = false;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      isOpen = false;
    }
  }

  onMount(() => {
    const handleClick = (event: MouseEvent) => {
      if (node && !node.contains(event.target as Node)) {
        isOpen = false;
      }
    };
    document.addEventListener("click", handleClick, true);
    return () => {
      document.removeEventListener("click", handleClick, true);
    };
  });
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="relative w-full" bind:this={node}>
  <button
    on:click={() => (isOpen = !isOpen)}
    class="flex h-11 w-full items-center justify-between rounded-lg border border-border bg-bg-input px-3 py-2 text-text-main transition-colors focus:outline-none focus:ring-2 focus:ring-accent"
  >
    <span
      >{options.find((opt) => opt.key === selected)?.name ?? placeholder}</span
    >
    <div
      class="transform text-text-dimmer transition-transform duration-200"
      class:rotate-180={isOpen}
    >
      <ChevronDown size={16} />
    </div>
  </button>

  {#if isOpen}
    <div
      class="absolute z-10 mt-1 w-full overflow-hidden rounded-lg border border-border bg-bg-main shadow-lg"
      transition:slide={{ duration: 150 }}
    >
      <ul class="max-h-60 overflow-y-auto py-1">
        {#each options as option (option.key)}
          <li
            on:click={() => handleSelect(option.key)}
            on:keydown={(e) => e.key === "Enter" && handleSelect(option.key)}
            class="cursor-pointer px-3 py-2 text-text-main transition-colors hover:bg-bg-hover"
            class:bg-bg-hover={selected === option.key}
            role="option"
            aria-selected={selected === option.key}
            tabindex="0"
          >
            {option.name}
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>
