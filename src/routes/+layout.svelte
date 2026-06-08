<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { browser } from "$app/environment";
  import Modal from "$lib/components/modals/Modal.svelte";
  import MobileNotice from "$lib/components/shell/MobileNotice.svelte";
  import { settings } from "$lib/stores/settingsStore.svelte";
  import { connectionStore } from "$lib/stores/connectionStore.svelte";
  import { requestsStore } from "$lib/stores/requestsStore.svelte";
  import { registerBuiltinPanes } from "$lib/dashboard/registry/builtins";
  import { registerExamplePanes } from "$lib/dashboard/examples";
  import { registerRobotPanes } from "$lib/dashboard/robot";

  registerBuiltinPanes();
  registerExamplePanes();
  registerRobotPanes();

  onMount(() => {
    void requestsStore.load();
    void connectionStore.load();
  });

  $effect(() => {
    if (browser) {
      if (document.documentElement.className !== settings.theme) {
        document.documentElement.className = settings.theme;
      }
      localStorage.setItem("settings", JSON.stringify(settings));
    }
  });

  let { children } = $props();
</script>

<Modal />

{@render children()}

<MobileNotice />
