<script lang="ts">
  import { FilePlus } from "@lucide/svelte";
  import AnimatedBot from "$lib/components/AnimatedBot.svelte";
  import Button from "$lib/components/common/Button.svelte";
  import { requestsStore } from "$lib/stores/requestsStore.svelte";
  import { tabsStore } from "$lib/stores/tabsStore.svelte";

  let heading = $state<HTMLHeadingElement>();

  async function createFirstRequest() {
    const request = await requestsStore.create();
    tabsStore.openRequest(request.id);
  }
</script>

<div
  class="text-text-disabled bg-bg-deep flex h-full w-full flex-1 flex-col items-center justify-center p-8"
>
  <div class="text-center">
    <AnimatedBot
      size={64}
      interactive
      readArea={() => heading?.getBoundingClientRect()}
      class="text-accent mx-auto mb-4"
    />
    <h2 bind:this={heading} class="text-text-main mb-2 text-2xl font-bold">
      Welcome to Robot Whisperer
    </h2>
    <p class="text-text-dimmer mx-auto mb-6 max-w-md text-center">
      A Postman-style client for ROS. Add a connection in the sidebar, then create a request or open
      an existing one to begin.
    </p>
    <div class="flex justify-center">
      <Button onclick={createFirstRequest}>
        <FilePlus size={16} /> Create your first request
      </Button>
    </div>
  </div>
</div>
