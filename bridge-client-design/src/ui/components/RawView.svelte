<script lang="ts">
    import { latest } from "../store";
    export let resourceName: string;
    $: msg = $latest[resourceName];
</script>

<pre class="text-xs whitespace-pre-wrap">{JSON.stringify(
        msg?.data ?? msg,
        null,
        2,
    )}</pre>
