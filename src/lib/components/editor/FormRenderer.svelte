<script lang="ts">
  import { untrack } from "svelte";
  import type { MessageDef, Value } from "$lib/core/types";
  import { defaultValueFor } from "$lib/core/schema";
  import FieldInput from "./FieldInput.svelte";

  let { message, value = $bindable() }: { message: MessageDef; value: Value } = $props();

  untrack(() => {
    if (value.kind === "struct") {
      for (const field of message.fields) {
        if (!(field.name in value.value)) {
          value.value[field.name] = defaultValueFor(field.field_type);
        }
      }
    }
  });
</script>

{#if value.kind === "struct" && message.fields.length > 0}
  <div class="form">
    {#each message.fields as field (field.name)}
      <label class="row">
        <span class="label" title={field.comment ?? undefined}>{field.name}</span>
        <FieldInput field={field.field_type} bind:value={value.value[field.name]} />
      </label>
    {/each}
  </div>
{:else if message.fields.length === 0}
  <p class="empty">This message has no fields.</p>
{/if}

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .row {
    display: grid;
    grid-template-columns: 160px 1fr;
    align-items: center;
    gap: 12px;
  }
  .label {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--color-text-dimmer);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty {
    font-size: 12px;
    color: var(--color-text-disabled);
  }
</style>
