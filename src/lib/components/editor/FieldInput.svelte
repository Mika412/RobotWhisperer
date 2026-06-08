<script lang="ts">
  import type { FieldType, Value } from "$lib/core/types";
  import { defaultValueFor, resolveSchemaByName } from "$lib/core/schema";
  import { Plus, X } from "@lucide/svelte";
  import FormRenderer from "./FormRenderer.svelte";
  import FieldInput from "./FieldInput.svelte";
  import Checkbox from "$lib/components/common/Checkbox.svelte";
  import IconButton from "$lib/components/common/IconButton.svelte";
  import TextInput from "$lib/components/common/TextInput.svelte";
  import NumberInput from "$lib/components/common/NumberInput.svelte";

  let { field, value = $bindable() }: { field: FieldType; value: Value } = $props();

  const FLOAT = new Set(["float32", "float64"]);

  function asNumber(): number {
    return value.kind === "int" ||
      value.kind === "uint" ||
      value.kind === "f32" ||
      value.kind === "f64"
      ? value.value
      : 0;
  }

  function setNumber(raw: string) {
    const parsed = Number(raw);
    const next = Number.isFinite(parsed) ? parsed : 0;
    if (
      value.kind === "int" ||
      value.kind === "uint" ||
      value.kind === "f32" ||
      value.kind === "f64"
    )
      value.value = next;
  }

  const nestedMessage = $derived(
    field.kind === "complex"
      ? resolveSchemaByName(field.value.type_name)?.kind === "message"
        ? resolveSchemaByName(field.value.type_name)
        : null
      : null,
  );
</script>

{#if field.kind === "primitive" && field.value === "bool"}
  <Checkbox
    checked={value.kind === "bool" && value.value}
    onchange={(checked) => (value = { kind: "bool", value: checked })}
  />
{:else if field.kind === "primitive"}
  <NumberInput
    step={FLOAT.has(field.value) ? "any" : "1"}
    value={asNumber()}
    oninput={(event) => setNumber(event.currentTarget.value)}
  />
{:else if field.kind === "string" || field.kind === "w_string"}
  <TextInput
    value={value.kind === "string" ? value.value : ""}
    oninput={(event) => (value = { kind: "string", value: event.currentTarget.value })}
  />
{:else if field.kind === "time" || field.kind === "duration"}
  {#if value.kind === "time" || value.kind === "duration"}
    <div class="pair">
      <NumberInput
        value={value.value.sec}
        oninput={(event) =>
          value.kind === "time" || value.kind === "duration"
            ? (value.value.sec = Number(event.currentTarget.value) || 0)
            : null}
      />
      <NumberInput
        value={value.value.nanosec}
        oninput={(event) =>
          value.kind === "time" || value.kind === "duration"
            ? (value.value.nanosec = Number(event.currentTarget.value) || 0)
            : null}
      />
    </div>
  {/if}
{:else if field.kind === "array"}
  {#if value.kind === "array"}
    <div class="array">
      {#each value.value as _item, index (index)}
        <div class="array-row">
          <FieldInput field={field.value.element} bind:value={value.value[index]} />
          <IconButton
            size="sm"
            tone="danger"
            label="Remove"
            title="Remove"
            onclick={() => value.kind === "array" && value.value.splice(index, 1)}
          >
            <X size={12} />
          </IconButton>
        </div>
      {/each}
      <button
        type="button"
        class="add"
        onclick={() =>
          value.kind === "array" && value.value.push(defaultValueFor(field.value.element))}
      >
        <Plus size={12} /> Add item
      </button>
    </div>
  {/if}
{:else if field.kind === "complex"}
  {#if nestedMessage && nestedMessage.kind === "message"}
    <div class="nested">
      <FormRenderer
        message={{ fields: nestedMessage.fields, constants: nestedMessage.constants }}
        bind:value
      />
    </div>
  {:else}
    <span class="unresolved">{field.value.type_name} (resolving…)</span>
  {/if}
{/if}

<style>
  .pair {
    display: flex;
    gap: 6px;
  }
  .array {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .array-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .add {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    align-self: flex-start;
    padding: 4px 8px;
    font-size: 11px;
    color: var(--color-text-dimmer);
    background: transparent;
    border: 1px dashed var(--color-border);
    border-radius: 6px;
    cursor: pointer;
  }
  .nested {
    border-left: 2px solid var(--color-border);
    padding-left: 10px;
  }
  .unresolved {
    color: var(--color-text-disabled);
    font-family: var(--font-mono);
    font-size: 12px;
  }
</style>
