<script lang="ts">
  import { untrack } from "svelte";
  import * as THREE from "three";
  import type { DecodedFrame } from "$lib/workers/decoderManager";
  import type { VisualizerProps } from "../types";
  import { extractCloud } from "../data/markerPoints";
  import { OrbitCameraController } from "../three/OrbitCameraController";

  type CloudConfig = { pointSize: number };
  let { source, config, overlay }: VisualizerProps<CloudConfig> = $props();

  let container = $state<HTMLDivElement>();
  let canvasEl = $state<HTMLCanvasElement>();
  let pointCount = $state(0);

  let renderer: THREE.WebGLRenderer | null = null;
  let scene: THREE.Scene | null = null;
  let camera: THREE.PerspectiveCamera | null = null;
  let controller: OrbitCameraController | null = null;
  let geometry: THREE.BufferGeometry | null = null;
  let material: THREE.PointsMaterial | null = null;
  let capacity = 0;
  let raf = 0;
  let needsRender = true;

  function ensureCapacity(count: number) {
    if (!geometry || count <= capacity) return;
    capacity = Math.max(count, capacity === 0 ? 1024 : capacity * 2);
    geometry.setAttribute("position", new THREE.BufferAttribute(new Float32Array(capacity * 3), 3));
  }

  function consume(frame: DecodedFrame) {
    if (!geometry) return;
    const cloud = extractCloud(frame.json);
    if (!cloud) return;
    const count = cloud.positions.length / 3;
    ensureCapacity(count);
    const attribute = geometry.getAttribute("position") as THREE.BufferAttribute;
    const target = attribute.array as Float32Array;
    for (let i = 0; i < count; i += 1) {
      target[i * 3] = cloud.positions[i * 3];
      target[i * 3 + 1] = cloud.positions[i * 3 + 2];
      target[i * 3 + 2] = -cloud.positions[i * 3 + 1];
    }
    geometry.setDrawRange(0, count);
    attribute.needsUpdate = true;
    pointCount = count;
    if (cloud.color && material)
      material.color.setRGB(cloud.color[0], cloud.color[1], cloud.color[2]);
    needsRender = true;
  }

  $effect(() => source.onFrame(consume));

  $effect(() => {
    if (material) material.size = config.pointSize || 0.04;
    needsRender = true;
  });

  function frame() {
    raf = requestAnimationFrame(frame);
    if (!renderer || !scene || !camera || !container || container.offsetParent === null) return;
    if (!needsRender) return;
    needsRender = false;
    renderer.render(scene, camera);
  }

  $effect(() => {
    if (!container || !canvasEl) return;
    const initialSize = untrack(() => config.pointSize) || 0.04;
    const rect = container.getBoundingClientRect();
    scene = new THREE.Scene();
    camera = new THREE.PerspectiveCamera(55, rect.width / Math.max(1, rect.height), 0.01, 1000);
    renderer = new THREE.WebGLRenderer({ canvas: canvasEl, antialias: true, alpha: true });
    renderer.setPixelRatio(window.devicePixelRatio);
    renderer.setSize(rect.width, rect.height, false);

    scene.add(new THREE.GridHelper(10, 10, 0x444b5a, 0x2a2f3a));

    geometry = new THREE.BufferGeometry();
    geometry.setAttribute("position", new THREE.BufferAttribute(new Float32Array(1024 * 3), 3));
    capacity = 1024;
    geometry.setDrawRange(0, 0);
    material = new THREE.PointsMaterial({ size: initialSize, color: 0xec4899 });
    scene.add(new THREE.Points(geometry, material));

    controller = new OrbitCameraController(camera, canvasEl, {
      distance: 6,
      onChange: () => {
        needsRender = true;
      },
    });

    const cached = untrack(() => source.latest);
    if (cached) consume(cached);

    raf = requestAnimationFrame(frame);

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        if (!renderer || !camera || width < 4 || height < 4) continue;
        camera.aspect = width / height;
        camera.updateProjectionMatrix();
        renderer.setSize(width, height, false);
        needsRender = true;
      }
    });
    observer.observe(container);

    return () => {
      cancelAnimationFrame(raf);
      observer.disconnect();
      controller?.dispose();
      geometry?.dispose();
      material?.dispose();
      renderer?.dispose();
      renderer = null;
      scene = null;
      camera = null;
      geometry = null;
      material = null;
    };
  });
</script>

<div class="cloud" bind:this={container}>
  <canvas class="viewport" bind:this={canvasEl}></canvas>
  <div class="overlay">
    {#if source.status === "error"}
      <span class="err">{source.error}</span>
    {:else}
      {pointCount} pts
    {/if}
  </div>
  {@render overlay?.()}
</div>

<style>
  .cloud {
    position: relative;
    height: 100%;
    min-height: 0;
    background: var(--color-bg-deep, var(--color-bg-main));
  }
  .viewport {
    position: absolute;
    inset: 0;
  }
  .overlay {
    position: absolute;
    bottom: 8px;
    left: 10px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-text-disabled);
    pointer-events: none;
  }
  .err {
    color: var(--color-danger);
  }
</style>
