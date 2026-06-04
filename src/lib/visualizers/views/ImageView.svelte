<script lang="ts">
  import type { DecodedFrame } from "$lib/workers/decoderManager";
  import type { Value } from "$lib/core/types";
  import type { VisualizerProps } from "../types";

  let { source, overlay }: VisualizerProps = $props();

  let canvas = $state<HTMLCanvasElement>();
  let dimensions = $state<{ width: number; height: number } | null>(null);
  let context: CanvasRenderingContext2D | null = null;
  let scratch: ImageData | null = null;

  let pendingFrame: DecodedFrame | null = null;
  let scheduled = 0;
  let rendering = false;

  function compressedBytes(frame: DecodedFrame): Uint8Array | null {
    const root = frame.json as Value | null;
    if (!root || root.kind !== "struct") return null;
    const data = root.value.data;
    if (!data) return null;
    if (data.kind === "bytes") return Uint8Array.from(data.value);
    if (data.kind === "array") {
      const out = new Uint8Array(data.value.length);
      for (let index = 0; index < data.value.length; index += 1) {
        const item = data.value[index];
        out[index] = item.kind === "uint" || item.kind === "int" ? item.value : 0;
      }
      return out;
    }
    return null;
  }

  function blit(width: number, height: number, draw: (ctx: CanvasRenderingContext2D) => void) {
    if (!canvas) return;
    if (!context) context = canvas.getContext("2d", { willReadFrequently: false });
    if (!context) return;
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
    }
    draw(context);
    if (!dimensions || dimensions.width !== width || dimensions.height !== height) {
      dimensions = { width, height };
    }
  }

  function renderRgba(rgba: Uint8ClampedArray, width: number, height: number) {
    const needed = width * height * 4;
    if (rgba.length < needed) return;
    if (!scratch || scratch.width !== width || scratch.height !== height) {
      scratch = new ImageData(width, height);
    }
    const buffer = scratch;
    buffer.data.set(rgba.length === needed ? rgba : rgba.subarray(0, needed));
    blit(width, height, (ctx) => ctx.putImageData(buffer, 0, 0));
  }

  async function renderCompressed(bytes: Uint8Array) {
    const bitmap = await createImageBitmap(new Blob([bytes]));
    blit(bitmap.width, bitmap.height, (ctx) => ctx.drawImage(bitmap, 0, 0));
    bitmap.close();
  }

  async function renderPending() {
    scheduled = 0;
    const frame = pendingFrame;
    pendingFrame = null;
    if (!frame || !canvas) return;
    rendering = true;
    try {
      if (frame.rgba && frame.width > 0 && frame.height > 0) {
        renderRgba(frame.rgba, frame.width, frame.height);
      } else {
        const bytes = compressedBytes(frame);
        if (bytes) await renderCompressed(bytes);
      }
    } catch {
      /* drop an undecodable frame */
    } finally {
      rendering = false;
      if (pendingFrame) schedule();
    }
  }

  function schedule() {
    if (scheduled || rendering) return;
    scheduled = requestAnimationFrame(renderPending);
  }

  function onFrame(frame: DecodedFrame) {
    pendingFrame = frame;
    schedule();
  }

  $effect(() => {
    const off = source.onFrame(onFrame);
    return () => {
      off();
      if (scheduled) cancelAnimationFrame(scheduled);
      scheduled = 0;
      pendingFrame = null;
    };
  });
</script>

<div class="image">
  <canvas bind:this={canvas}></canvas>
  {#if source.status === "error"}
    <div class="overlay err">{source.error}</div>
  {:else if !dimensions}
    <div class="overlay">Waiting for frames…</div>
  {:else}
    <div class="overlay dims">{dimensions.width}×{dimensions.height}</div>
  {/if}
  {@render overlay?.()}
</div>

<style>
  .image {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 8px;
    background: var(--color-bg-deep, var(--color-bg-main));
  }
  canvas {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border: 1px solid var(--color-border);
    border-radius: 4px;
    image-rendering: pixelated;
  }
  .overlay {
    position: absolute;
    bottom: 10px;
    left: 12px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-text-disabled);
  }
  .dims {
    background: color-mix(in srgb, var(--color-bg-main) 70%, transparent);
    padding: 1px 6px;
    border-radius: 4px;
  }
  .err {
    color: var(--color-danger);
  }
</style>
