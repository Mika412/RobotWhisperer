<script lang="ts">
  import { Save, RotateCcw } from "@lucide/svelte";
  import {
    valuePreviewText,
    type MessageDef,
    type ParsedSchema,
    type Value,
    type RequestView,
  } from "$lib/core/types";
  import { tabsStore } from "$lib/stores/tabsStore.svelte";
  import { requestsStore } from "$lib/stores/requestsStore.svelte";
  import { connectionStore } from "$lib/stores/connectionStore.svelte";
  import { discoveryStore } from "$lib/stores/discoveryStore.svelte";
  import { subscriptionStore } from "$lib/stores/subscriptionStore.svelte";
  import { listSchemasByName, forgetUnresolvedSchemas } from "$lib/core/schema";
  import { pipelineHub } from "$lib/core/pipelineHub";
  import { useTopicSource } from "$lib/visualizers/frameSource.svelte";
  import Button from "$lib/components/common/Button.svelte";
  import RequestBar from "$lib/components/editor/RequestBar.svelte";
  import FormRenderer from "$lib/components/editor/FormRenderer.svelte";
  import ResultPanel from "$lib/components/editor/ResultPanel.svelte";
  import TopicResult from "$lib/components/editor/TopicResult.svelte";

  let { requestId }: { requestId: number } = $props();
  const tab = $derived(tabsStore.requestTab(requestId)!);

  let running = $state(false);

  let serviceResponse = $state("");
  let feedbackText = $state("");
  let resultText = $state("");
  let goalId = $state<string | null>(null);
  let calling = $state(false);
  let actionError = $state<string | null>(null);

  const sessionId = $derived(
    tab.draft.connection_id != null ? connectionStore.sessionId(tab.draft.connection_id) : null,
  );
  const target = $derived(tab.draft.target.trim());
  const canRun = $derived(!!sessionId && target.length > 0);

  const source = useTopicSource(
    () => tab.draft.connection_id,
    () => (running && tab.draft.kind === "topic" ? target : ""),
  );
  const rawText = $derived.by(() => {
    if (source.value == null) return "";
    return valuePreviewText(source.value as Value);
  });
  const topicStatus = $derived(source.status === "waiting" ? "connecting" : source.status);

  function persistView(next: RequestView) {
    tab.draft.visualization = next;
    requestsStore.setVisualization(tab.requestId, next);
  }
  const locked = $derived((tab.draft.kind === "topic" && running) || calling);
  const suggestions = $derived(discoveryStore.suggestions(tab.draft.connection_id, tab.draft.kind));
  const targetSchemaName = $derived(
    suggestions.find((entry) => entry.name === target)?.schemaName ?? null,
  );

  const connectionValue = $derived(
    tab.draft.connection_id == null ? "" : String(tab.draft.connection_id),
  );
  const connectionOptions = $derived([
    { value: "", label: "No connection" },
    ...connectionStore.connections.map((connection) => ({
      value: String(connection.id),
      label: connection.name,
    })),
  ]);

  const displayStatus = $derived(
    tab.draft.kind === "topic" ? topicStatus : calling ? "pending" : actionError ? "error" : "idle",
  );
  const displaySchema = $derived(
    tab.draft.kind === "topic"
      ? source.schemaName || targetSchemaName || ""
      : targetSchemaName || "",
  );
  const displayError = $derived(tab.draft.kind === "topic" ? source.error : actionError);
  const dotClass = $derived(
    displayStatus === "active" || displayStatus === "pending"
      ? "running"
      : displayStatus === "error"
        ? "danger"
        : "",
  );

  let parsedSchema = $state<ParsedSchema | null>(null);
  $effect(() => {
    const name = targetSchemaName;
    if (!name) {
      parsedSchema = null;
      return;
    }
    const wantKind =
      tab.draft.kind === "action" ? "action" : tab.draft.kind === "service" ? "service" : "message";
    let cancelled = false;
    void listSchemasByName(name).then((defs) => {
      if (cancelled) return;
      const matched = defs.find((def) => def.parsed?.kind === wantKind) ?? defs[0];
      parsedSchema = matched?.parsed ?? null;
    });
    return () => {
      cancelled = true;
    };
  });

  const requestMessage = $derived<MessageDef | null>(
    parsedSchema?.kind === "service" ? parsedSchema.request : null,
  );
  const goalMessage = $derived<MessageDef | null>(
    parsedSchema?.kind === "action" ? parsedSchema.goal : null,
  );

  $effect(() => {
    if (
      (tab.draft.kind === "service" || tab.draft.kind === "action") &&
      tab.draft.input.kind !== "struct"
    ) {
      tab.draft.input = { kind: "struct", value: {} };
    }
  });

  $effect(() => {
    const cid = tab.draft.connection_id;
    if (cid == null || connectionStore.status(cid) !== "connected") return;
    void discoveryStore.refresh(cid);
    const interval = setInterval(() => {
      void discoveryStore.refresh(cid);
      forgetUnresolvedSchemas();
    }, 1500);
    return () => clearInterval(interval);
  });

  $effect(() => {
    const canonical = requestsStore.get(tab.requestId);
    const snapshot = JSON.stringify($state.snapshot(tab.draft));
    tabsStore.setDirty(tab.requestId, canonical ? snapshot !== JSON.stringify(canonical) : true);
  });

  $effect(() => {
    subscriptionStore.setActive(tab.requestId, source.status === "active");
    return () => subscriptionStore.setActive(tab.requestId, false);
  });

  function setConnection(value: string) {
    tab.draft.connection_id = value === "" ? null : Number(value);
  }

  function refreshDiscovery() {
    if (tab.draft.connection_id != null)
      void discoveryStore.refresh(tab.draft.connection_id, { force: true });
  }

  function errorText(err: unknown): string {
    if (err instanceof Error) return err.message;
    if (err && typeof err === "object") {
      const record = err as { message?: unknown; kind?: unknown };
      if (typeof record.message === "string" && record.message.length > 0) {
        return typeof record.kind === "string"
          ? `${record.kind}: ${record.message}`
          : record.message;
      }
      try {
        return JSON.stringify(err);
      } catch {
        return String(err);
      }
    }
    return String(err);
  }

  async function callService() {
    serviceResponse = "";
    actionError = null;
    calling = true;
    try {
      const response = await pipelineHub.callService(
        sessionId!,
        target,
        $state.snapshot(tab.draft.input) as Value,
      );
      serviceResponse = valuePreviewText(response);
    } catch (err) {
      actionError = errorText(err);
    } finally {
      calling = false;
    }
  }

  function sendActionGoal() {
    feedbackText = "";
    resultText = "";
    actionError = null;
    goalId = null;
    calling = true;
    const goal = new Promise<void>((resolve, reject) => {
      pipelineHub
        .sendActionGoal(
          sessionId!,
          target,
          $state.snapshot(tab.draft.input) as Value,
          (envelope) => {
            if (envelope.kind === "feedback") {
              feedbackText = valuePreviewText(envelope.value);
            } else if (envelope.kind === "result") {
              resultText = valuePreviewText(envelope.value);
            } else if (envelope.kind === "error") {
              reject(new Error(envelope.message));
            } else if (envelope.kind === "closed") {
              resolve();
            }
          },
        )
        .then((id) => {
          goalId = id;
        })
        .catch(reject);
    });
    void goal
      .catch((err) => {
        actionError = errorText(err);
      })
      .finally(() => {
        calling = false;
      });
  }

  function cancelGoal() {
    if (goalId) void pipelineHub.cancelActionGoal(goalId);
  }
</script>

<div class="editor">
  <div class="head">
    <div class="title-row">
      <div class="title-block">
        <input class="name" bind:value={tab.draft.name} placeholder="Request name" />
      </div>
      <div class="title-actions">
        <Button
          variant="ghost"
          onclick={() => tabsStore.revert(tab.requestId)}
          disabled={!tab.dirty}
        >
          <RotateCcw size={14} /> Revert
        </Button>
        <Button onclick={() => tabsStore.save(tab.requestId)} disabled={!tab.dirty}>
          <Save size={14} /> Save
        </Button>
      </div>
    </div>

    <RequestBar
      {tab}
      {connectionValue}
      {connectionOptions}
      {suggestions}
      {locked}
      {canRun}
      pending={calling}
      bind:running
      {setConnection}
      onkind={(kind) => (tab.draft.kind = kind)}
      ontarget={(value) => (tab.draft.target = value)}
      onrefresh={refreshDiscovery}
      oncall={callService}
      onsend={sendActionGoal}
      oncancel={cancelGoal}
    />

    <div class="status-line">
      <span class="status-dot {dotClass}"></span>
      <span class="status-text">{displayStatus}</span>
      {#if displaySchema}<span class="schema mono">{displaySchema}</span>{/if}
      {#if displayError}<span class="err">{displayError}</span>{/if}
    </div>
  </div>

  <div class="body scrollbar-custom">
    {#if tab.draft.kind === "topic"}
      <TopicResult
        {source}
        {rawText}
        view={tab.draft.visualization ?? null}
        onviewchange={persistView}
      />
    {:else if tab.draft.kind === "service"}
      <div class="section-card form-card">
        <div class="form-head"><span class="section-label">REQUEST</span></div>
        <div class="form-body">
          {#if requestMessage}
            <FormRenderer message={requestMessage} bind:value={tab.draft.input} />
          {:else}
            <p class="hint">Pick a service target on a connection to load its request fields.</p>
          {/if}
        </div>
      </div>
      {#if serviceResponse}
        <ResultPanel label="RESPONSE" text={serviceResponse} />
      {/if}
    {:else if tab.draft.kind === "action"}
      <div class="section-card form-card">
        <div class="form-head"><span class="section-label">GOAL</span></div>
        <div class="form-body">
          {#if goalMessage}
            <FormRenderer message={goalMessage} bind:value={tab.draft.input} />
          {:else}
            <p class="hint">Pick an action target on a connection to load its goal fields.</p>
          {/if}
        </div>
      </div>
      <div class="result-grid">
        <ResultPanel label="FEEDBACK" text={feedbackText} />
        <ResultPanel label="RESULT" text={resultText} />
      </div>
    {:else}
      <div class="placeholder">Parameter requests aren't supported yet.</div>
    {/if}
  </div>
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-height: 0;
  }
  .head {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }
  .title-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .title-block {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 1;
    min-width: 0;
  }
  .name {
    flex: 1;
    min-width: 0;
    padding: 4px 8px;
    margin-left: -4px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    color: var(--color-text-main);
    font-size: 20px;
    font-weight: 700;
    cursor: text;
    transition:
      background 0.12s ease,
      border-color 0.12s ease;
  }
  .name::placeholder {
    color: var(--color-text-disabled);
  }
  .name:hover {
    background: var(--color-bg-hover);
  }
  .name:focus {
    outline: none;
    background: var(--color-bg-input);
    border-color: var(--color-accent);
  }
  .title-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }
  .status-line {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--color-text-dimmer);
    min-height: 16px;
  }
  .status-text {
    text-transform: capitalize;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .schema {
    color: var(--color-text-disabled);
  }
  .err {
    color: var(--color-danger);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 20px;
    overflow: auto;
  }
  .form-card {
    flex: none;
  }
  .form-head {
    padding: 8px 12px;
    border-bottom: 1px solid var(--color-border);
  }
  .form-body {
    padding: 16px;
  }
  .hint {
    font-size: 12px;
    color: var(--color-text-disabled);
    margin: 0;
  }
  .result-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    flex: 1;
    min-height: 0;
  }
  .placeholder {
    border: 1px dashed var(--color-border);
    border-radius: 10px;
    padding: 24px;
    color: var(--color-text-disabled);
    font-size: 13px;
  }
</style>
