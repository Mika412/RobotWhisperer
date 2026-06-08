<script lang="ts">
  import { untrack } from "svelte";
  import { modalStore } from "$lib/stores/modalStore.svelte";
  import { connectionStore } from "$lib/stores/connectionStore.svelte";
  import Select from "$lib/components/common/Select.svelte";
  import Checkbox from "$lib/components/common/Checkbox.svelte";
  import Button from "$lib/components/common/Button.svelte";
  import TextInput from "$lib/components/common/TextInput.svelte";
  import type { Connection, NewConnection, TransportConfig, TransportKind } from "$lib/core/types";

  let { connection }: { connection?: Connection } = $props();

  const kindOptions = [
    { value: "dummy", label: "Dummy (synthetic)" },
    { value: "foxglove_ws", label: "Foxglove WebSocket" },
    { value: "rosbridge", label: "rosbridge" },
  ];

  const DEFAULT_URL: Record<string, string> = {
    foxglove_ws: "ws://localhost:8765",
    rosbridge: "ws://localhost:9090",
  };

  const seed = untrack(() => {
    const config = connection?.config;
    const urlConfig =
      config && (config.kind === "foxglove_ws" || config.kind === "rosbridge") ? config : null;
    return {
      name: connection?.name ?? "New connection",
      kind: config?.kind ?? "dummy",
      url: urlConfig ? urlConfig.url : DEFAULT_URL.foxglove_ws,
      autoConnect: connection?.auto_connect ?? false,
    };
  });

  let name = $state(seed.name);
  let kind = $state<TransportKind>(seed.kind);
  let url = $state(seed.url);
  let autoConnect = $state(seed.autoConnect);
  let error = $state<string | null>(null);
  let saving = $state(false);

  const needsUrl = $derived(kind === "foxglove_ws" || kind === "rosbridge");

  function buildConfig(): TransportConfig {
    switch (kind) {
      case "foxglove_ws":
        return { kind: "foxglove_ws", url, headers: [] };
      case "rosbridge":
        return { kind: "rosbridge", url };
      case "dummy":
        return { kind: "dummy" };
      case "native_ros2":
        return { kind: "native_ros2", domain_id: 0 };
    }
  }

  async function save() {
    error = null;
    saving = true;
    try {
      const config = buildConfig();
      if (connection) {
        await connectionStore.update({ ...connection, name, config, auto_connect: autoConnect });
      } else {
        const draft: NewConnection = { name, config, auto_connect: autoConnect };
        await connectionStore.create(draft);
      }
      modalStore.close();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      saving = false;
    }
  }
</script>

<div class="p-6">
  <h2 class="text-text-main mb-6 text-xl font-bold">
    {connection ? "Edit connection" : "New connection"}
  </h2>

  <div class="space-y-4">
    <label class="block">
      <span class="text-text-dimmer mb-1 block text-sm font-medium">Name</span>
      <TextInput
        size="lg"
        mono={false}
        value={name}
        oninput={(event) => (name = event.currentTarget.value)}
      />
    </label>

    <div>
      <span class="text-text-dimmer mb-1 block text-sm font-medium">Transport</span>
      <Select
        value={kind}
        options={kindOptions}
        onchange={(value) => (kind = value as TransportKind)}
      />
    </div>

    {#if needsUrl}
      <label class="block">
        <span class="text-text-dimmer mb-1 block text-sm font-medium">URL</span>
        <TextInput
          size="lg"
          value={url}
          placeholder={DEFAULT_URL[kind]}
          oninput={(event) => (url = event.currentTarget.value)}
        />
      </label>
    {/if}

    <Checkbox
      checked={autoConnect}
      onchange={(value) => (autoConnect = value)}
      label="Connect automatically on launch"
    />

    {#if error}
      <p class="text-danger text-sm">{error}</p>
    {/if}
  </div>

  <div class="mt-8 flex justify-end gap-2">
    <Button variant="ghost" onclick={modalStore.close}>Cancel</Button>
    <Button onclick={save} disabled={saving}>
      {saving ? "Saving…" : "Save"}
    </Button>
  </div>
</div>
