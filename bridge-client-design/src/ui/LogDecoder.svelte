<script lang="ts">
  import { parse } from "@foxglove/rosmsg";
  import { MessageReader, MessageWriter } from "@foxglove/rosmsg2-serialization";
  // import { Buffer } from "buffer";

  // // Polyfill Buffer for the browser environment
  // if (typeof window !== "undefined") {
  //   window.Buffer = Buffer;
  // }

  let decodedData = $state<Record<string, unknown> | null>(null);
  let error = $state<string | null>(null);

  // 1. Your raw message definition string, exactly as you provided it.
  // The '===' separator is the standard way to include multiple definitions.
  const messageDefinitionString = `
    uint8 DEBUG=10
    uint8 INFO=20
    uint8 WARN=30
    uint8 ERROR=40
    uint8 FATAL=50

    builtin_interfaces/Time stamp
    uint8 level
    string name
    string msg
    string file
    string function
    uint32 line

    ================================================================================
    MSG: builtin_interfaces/Time
    int32 sec
    uint32 nanosec
  `;

  // 2. Parse the definition string into a schema object.
  // The `parse` function returns an array of definitions found in the string.
  const parsedDefinitions = parse(messageDefinitionString, { ros2: true });

  // The first definition in the array is our main Log message.
  const logMessageSchema = parsedDefinitions[0];

  // 3. Generate sample CDR data for demonstration using MessageWriter.
  // This creates the binary buffer that your app would typically receive.
  const writer = new MessageWriter(parsedDefinitions);
  const sampleLogObject = {
    stamp: { sec: 1725319200, nanosec: 123456789 },
    level: 30, // WARN
    name: "ros2_svelte_app",
    msg: "Using the correct @foxglove/rosmsg2-serialization library!",
    file: "Ros2LogDecoder.svelte",
    function: "onMount",
    line: 99,
  };
  const sampleCdrData = writer.writeMessage(sampleLogObject);

  // 4. Decoding Logic using MessageReader
  function decodeCdrMessage() {
    try {
      error = null;
      // Instantiate the reader with the schema for the main message.
      const reader = new MessageReader(parsedDefinitions);

      // Use the readMessage method to deserialize the buffer.
      decodedData = reader.readMessage(sampleCdrData);

      console.log("Decoded ROS 2 Log Message:", decodedData);
    } catch (e) {
      error = (e as Error).message;
      console.error("Decoding failed:", e);
    }
  }
</script>

<div class="decoder-card">
  <h2>ROS 2 Log Message Decoder (Corrected) ✔️</h2>
  <p>
    Using <code>@foxglove/rosmsg</code> to parse the schema and
    <code>@foxglove/rosmsg2-serialization</code> to read the CDR buffer.
  </p>

  <button onclick={decodeCdrMessage}>Decode Log CDR</button>

  {#if decodedData}
    <div class="result">
      <h3>Decoded Log Data:</h3>
      <pre>{JSON.stringify(decodedData, null, 2)}</pre>
    </div>
  {/if}

  {#if error}
    <div class="error">
      <h3>Error:</h3>
      <p>{error}</p>
    </div>
  {/if}
</div>

<style>
  .decoder-card {
    font-family: sans-serif;
    border: 1px solid #ccc;
    padding: 20px;
    border-radius: 8px;
    max-width: 600px;
    margin: 20px auto;
  }
  .result, .error { margin-top: 15px; }
  pre { background-color: #f4f4f4; padding: 10px; border-radius: 4px; white-space: pre-wrap; }
  .error { color: red; }
</style>
