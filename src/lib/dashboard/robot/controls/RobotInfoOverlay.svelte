<script lang="ts">
  let { model, brand }: { model: string; brand: string } = $props();
</script>

<div class="info">
  <div class="row">
    <span class="label">Model</span>
    <span class="value">{model}</span>
  </div>
  {#if brand}
    <div class="row">
      <span class="label">Brand</span>
      <span class="value">{brand}</span>
    </div>
  {/if}
</div>

<style>
  .info {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 11px 13px;
    border-radius: 12px;
    background: color-mix(in srgb, var(--color-bg-main) 70%, transparent);
    backdrop-filter: blur(16px) saturate(1.1);
    -webkit-backdrop-filter: blur(16px) saturate(1.1);
    border: 1px solid color-mix(in srgb, var(--color-text-main) 11%, transparent);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.16);
  }
  .row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }
  .label {
    flex: none;
    width: 42px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-text-dimmer);
  }
  .value {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--color-text-main);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
