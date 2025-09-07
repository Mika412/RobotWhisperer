<script lang="ts">
    import { db } from "../../lib/db";
    import type { SavedRequest } from "../../lib/types";
    import { activeTabId, latest } from "../store";
    import { get } from "svelte/store";
    import RawView from "./RawView.svelte";
    import PrettyView from "./PrettyView.svelte";
    import VisualizerResolver from "./VisualizerResolver.svelte";
    import { WSFoxgloveProvider } from "../../lib/ros/wsFoxgloveProvider";

    export let active: string;
    let req: SavedRequest | undefined;
    let provider = new WSFoxgloveProvider();

    provider.setEvents({
        onStatus: (s) => console.log("status", s),
        onTopics: (list) => console.log("topics", list),
        onMessage: (msg) => {
            latest.update((m) => ((m[msg.resourceName] = msg), m));
        },
    });

    const subState = { subId: "" };

    $: (async () => {
        if (active !== get(activeTabId)) return;
        req = await db.requests.get(active);
    })();

    async function connect() {
        await provider.connect({
            url:
                (document.getElementById("ws-url") as HTMLInputElement)
                    ?.value || "ws://localhost:8765",
        });
    }
    async function subscribe() {
        if (!req) return;
        subState.subId = await provider.subscribe(
            req.resourceName,
            req.messageType,
        );
    }
    async function unsubscribe() {
        if (subState.subId) await provider.unsubscribe(subState.subId);
    }

    async function save() {
        if (!req) return;
        req.updatedAt = Date.now();
        await db.requests.put(req);
    }
</script>

{#if req}
    <div class="p-3 flex gap-3 items-center border-b">
        <input id="ws-url" placeholder="ws://robot:8765" class="input" />
        <button class="btn" on:click={connect}>Connect</button>

        <select bind:value={req.kind} class="select" on:change={save}>
            <option value="topic">Topic</option>
            <option value="service">Service</option>
            <option value="action">Action</option>
        </select>
        <input
            class="input"
            placeholder="/topic_or_service"
            bind:value={req.resourceName}
            on:change={save}
        />
        <input
            class="input"
            placeholder="pkg/Type"
            bind:value={req.messageType}
            on:change={save}
        />

        {#if req.kind === "topic"}
            <button class="btn" on:click={subscribe}>Subscribe</button>
            <button class="btn" on:click={unsubscribe}>Unsubscribe</button>
        {/if}
    </div>

    <div class="p-3">
        <div class="tabs">
            <input type="radio" id="raw" name="tab" checked />
            <label for="raw">Raw</label>
            <div class="tab"><RawView resourceName={req.resourceName} /></div>

            <input type="radio" id="pretty" name="tab" />
            <label for="pretty">Pretty</label>
            <div class="tab">
                <PrettyView
                    resourceName={req.resourceName}
                    messageType={req.messageType}
                />
            </div>

            <input type="radio" id="viz" name="tab" />
            <label for="viz">Visualization</label>
            <div class="tab">
                <VisualizerResolver
                    resourceName={req.resourceName}
                    messageType={req.messageType}
                />
            </div>
        </div>
    </div>
    <button class="btn" on:click={async () => {
  // force list to verify worker's topic cache (and that advertise ran)
  console.log(await provider.listTopics());
}}>List topics</button>

{:else}
    <div class="p-6">Loading…</div>
{/if}

<!-- <style>
    .input {
        @apply border px-2 py-1 rounded w-48;
    }
    .select {
        @apply border px-2 py-1 rounded;
    }
    .btn {
        @apply bg-gray-800 text-white px-2 py-1 rounded;
    }
    .tabs {
        display: grid;
        grid-auto-rows: min-content;
    }
    .tabs > input {
        display: none;
    }
    .tabs > label {
        @apply inline-block mr-2 px-3 py-1 border rounded-t;
    }
    .tabs > input:checked + label + div {
        display: block;
    }
    .tabs > .tab {
        display: none;
        border: @apply border;
        padding: 0.5rem;
    }
</style> -->
