<script lang="ts">
  import { onMount } from "svelte";
  import { Github } from "@lucide/svelte";
  import AnimatedBot from "$lib/components/AnimatedBot.svelte";
  import { isMobileDevice } from "$lib/core/platform";

  const GITHUB_URL = "https://github.com/Mika412/RobotWhisperer";

  let show = $state(false);
  let card = $state<HTMLDivElement>();
  let title = $state<HTMLHeadingElement>();
  onMount(() => {
    show = isMobileDevice();
  });

  $effect(() => {
    if (!show) return;
    const html = document.documentElement;
    const body = document.body;
    const prevHtml = html.style.overflow;
    const prevBody = body.style.overflow;
    html.style.overflow = "hidden";
    body.style.overflow = "hidden";
    return () => {
      html.style.overflow = prevHtml;
      body.style.overflow = prevBody;
    };
  });
</script>

{#if show}
  <div class="gate" role="dialog" aria-modal="true" aria-label="Larger display required">
    <div class="card" bind:this={card}>
      <AnimatedBot
        class="logo"
        size={60}
        strokeWidth={2.2}
        interactive
        readArea={() => title?.getBoundingClientRect()}
        bounds={() => card?.getBoundingClientRect()}
      />
      <h1 bind:this={title}>Robot Whisperer</h1>
      <p>Designed for larger displays. Please open it on a desktop or laptop.</p>
      <a class="gh" href={GITHUB_URL} target="_blank" rel="noreferrer noopener">
        <Github size={16} /> View on GitHub
      </a>
    </div>
  </div>
{/if}

<style>
  .gate {
    position: fixed;
    inset: 0;
    z-index: 10000;
    height: 100dvh;
    display: flex;
    padding: 24px;
    overflow-y: auto;
    overscroll-behavior: contain;
    background: color-mix(in srgb, var(--color-bg-deep) 58%, transparent);
    backdrop-filter: blur(26px) saturate(140%);
    -webkit-backdrop-filter: blur(26px) saturate(140%);
  }
  .card {
    width: 100%;
    max-width: 340px;
    margin: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 12px;
    padding: 32px 28px;
    border-radius: 20px;
    background: color-mix(in srgb, var(--color-bg-main) 72%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-border-strong) 60%, transparent);
    box-shadow: 0 20px 56px rgba(0, 0, 0, 0.4);
  }
  .gate :global(.logo) {
    color: var(--color-accent);
  }
  h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 800;
    letter-spacing: -0.01em;
    color: var(--color-text-main);
  }
  p {
    margin: 0;
    font-size: 14px;
    line-height: 1.5;
    color: var(--color-text-dimmer);
  }
  .gh {
    margin-top: 6px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    text-decoration: none;
    font-size: 13px;
    font-weight: 600;
    color: var(--color-text-main);
    padding: 10px 18px;
    border-radius: 10px;
    border: 1px solid var(--color-border-strong);
    background: var(--color-bg-input);
    transition:
      background 0.15s ease,
      border-color 0.15s ease;
  }
  .gh:hover {
    background: var(--color-bg-hover);
    border-color: var(--color-accent);
  }
</style>
