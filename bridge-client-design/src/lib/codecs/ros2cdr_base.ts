import { parse } from "@foxglove/rosmsg";
import { type MessageDefinition } from "@foxglove/message-definition";
import { MessageReader } from "@foxglove/rosmsg2-serialization";

/**
 * Parses a ROS 2 message definition string into its JSON object representation.
 * This is useful for caching the parsed schema to avoid re-parsing it repeatedly.
 *
 * @param schemaText The raw text of the message definition (.msg).
 * @returns The parsed schema as an array of JavaScript objects.
 */
export function schemaTextToJson(schemaText: string): string {
  // The parse function is the core of the conversion.
  // It returns the structured object array that serves as the "JSON representation".
  const messageDefinition = parse(schemaText, { ros2: true });

  // print the parsed message definition structure
  return JSON.stringify(messageDefinition, null, 2);
}

/**
 * Decodes a ROS 2 CDR buffer using a pre-parsed JSON schema object.
 * This is more efficient than decodeRos2Cdr if you are decoding multiple messages
 * with the same schema, as it avoids re-parsing the schema text each time.
 *
 * @param typeName The name of the top-level message type to decode.
 * @param schemaJson The pre-parsed schema from `schemaTextToJson`.
 * @param buf The binary CDR data to decode as an ArrayBuffer.
 * @returns The decoded message as a JavaScript object.
 */
export function decodeRos2CdrFromJson(
  typeName: string,
  schemaJson: string,
  buf: ArrayBuffer,
): any {
  // 1. This function receives the schema already parsed, so we can use it directly.
  console.log(schemaJson);
  // const messageDefinition = parse(schemaJson, { ros2: true });
  const reader = new MessageReader(schemaJson);

  // 2. Convert the ArrayBuffer to a Uint8Array.
  const uint8Array = new Uint8Array(buf);

  // 3. Read the message from the buffer.
  try {
    const message = reader.readMessage(uint8Array);
    return message;
  } catch (e) {
    console.error(`Failed to decode CDR message of type "${typeName}" from JSON schema:`, e);
    throw e;
  }
}

/**
 * Decodes a ROS 2 CDR binary buffer using a raw message schema string.
 * This function parses the schema on every call, which can be less efficient.
 *
 * @param typeName The name of the top-level message type to decode.
 * @param schemaText The raw text of the message definition (.msg).
 * @param buf The binary CDR data to decode as an ArrayBuffer.
 * @returns The decoded message as a JavaScript object.
 */
export function decodeRos2Cdr(typeName: string, schemaText: string, buf: ArrayBuffer): any {
  // This function now simply composes the two new functions.
  // const schemaJson = schemaTextToJson(schemaText);
  return decodeRos2CdrFromJson(typeName, schemaText, buf);
}
