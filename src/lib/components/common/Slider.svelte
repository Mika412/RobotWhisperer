<script lang="ts">
  let {
    value,
    min,
    max,
    step = 1,
    label = "",
    display = "",
    oninput,
  }: {
    value: number;
    min: number;
    max: number;
    step?: number;
    label?: string;
    display?: string;
    oninput: (value: number) => void;
  } = $props();
</script>

<label class="slider">
  {#if label || display}
    <span class="head">
      {#if label}<span class="lbl">{label}</span>{/if}
      {#if display}<span class="val">{display}</span>{/if}
    </span>
  {/if}
  <input
    type="range"
    {min}
    {max}
    {step}
    {value}
    oninput={(event) => oninput(Number(event.currentTarget.value))}
  />
</label>

<style>
  .slider {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .head {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    align-items: baseline;
  }
  .lbl {
    font-family: var(--font-mono);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-text-dimmer);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .val {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-text-dim);
    flex: none;
  }
  input[type="range"] {
    width: 100%;
    accent-color: var(--color-accent);
  }
</style>
