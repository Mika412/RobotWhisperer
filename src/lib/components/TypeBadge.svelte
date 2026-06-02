<script lang="ts">
  import type { RequestKind } from "$lib/core/types";

  let { kind }: { kind: RequestKind } = $props();

  const LABELS: Record<RequestKind, string> = {
    topic: "TOP",
    service: "SER",
    action: "ACT",
    param: "PAR",
  };

  const CLASSES: Record<RequestKind, string> = {
    topic: "badge-topic",
    service: "badge-service",
    action: "badge-action",
    param: "badge-param",
  };
</script>

<span class={CLASSES[kind]}>{LABELS[kind]}</span>
