<script lang="ts">
  import { Trash2 } from "@lucide/svelte";
  import { modalStore } from "$lib/stores/modalStore.svelte";
  import { settings } from "$lib/stores/settingsStore.svelte";
  import { workspaceRpc } from "$lib/core/workspaceRpc";
  import { THEMES } from "$lib/themes";
  import Select from "$lib/components/common/Select.svelte";
  import Button from "$lib/components/common/Button.svelte";

  const themeOptions = THEMES.map(({ id, name }) => ({ value: id, label: name }));

  let clearing = $state(false);
  let clearError = $state<string | null>(null);

  async function clearCache() {
    const ok = confirm(
      "Wipe all workspace state?\n\nThis deletes every request, collection, connection and " +
        "schema from the workspace database. The theme preference is preserved, and the app " +
        "reloads afterwards.",
    );
    if (!ok) return;
    clearing = true;
    clearError = null;
    try {
      await workspaceRpc.clearWorkspaceStorage();
      location.reload();
    } catch (err) {
      clearError = err instanceof Error ? err.message : String(err);
      clearing = false;
    }
  }
</script>

<div class="settings">
  <h2 class="title">Settings</h2>

  <div class="field">
    <span class="label">Theme</span>
    <Select
      value={settings.theme}
      options={themeOptions}
      onchange={(value) => (settings.theme = value)}
    />
  </div>

  <div class="field section">
    <span class="label">Workspace</span>
    <Button variant="danger" disabled={clearing} onclick={clearCache}>
      <Trash2 size={14} />
      {clearing ? "Clearing…" : "Clear workspace data"}
    </Button>
    <p class="hint">
      Wipes every request, collection, connection and schema. The theme is preserved; the app
      reloads after.
    </p>
    {#if clearError}<p class="err">{clearError}</p>{/if}
  </div>

  <div class="footer">
    <Button onclick={modalStore.close}>Done</Button>
  </div>
</div>

<style>
  .settings {
    padding: 24px;
  }
  .title {
    margin-bottom: 20px;
    font-size: 20px;
    font-weight: 700;
    color: var(--color-text-main);
  }
  .field {
    margin-bottom: 20px;
  }
  .label {
    display: block;
    margin-bottom: 8px;
    font-size: 13px;
    font-weight: 500;
    color: var(--color-text-dimmer);
  }
  .section {
    border-top: 1px solid var(--color-border);
    padding-top: 16px;
  }
  .hint {
    margin-top: 6px;
    font-size: 11px;
    line-height: 1.4;
    color: var(--color-text-disabled);
  }
  .err {
    margin-top: 6px;
    font-size: 11px;
    color: var(--color-danger);
    font-family: var(--font-mono);
  }
  .footer {
    margin-top: 28px;
    display: flex;
    justify-content: flex-end;
  }
</style>
