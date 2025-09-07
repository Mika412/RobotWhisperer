/// <reference lib="webworker" />
import { decodeRos2Cdr } from "../codecs/ros2cdr_base";

self.onmessage = (e) => {
  const { type, channel, data } = e.data;
  // console.log("FUFUFUFUFUFUF", type, channel, data);

  if (type !== "data") return;
  try {
    let obj: any = undefined;
    if (
      channel.encoding === "cdr" ||
      channel.schemaEncoding.startsWith("ros2")
    ) {
      obj = decodeRos2Cdr(channel.type, channel.schema, data);
    } else if (channel.encoding === "json") {
      obj = JSON.parse(new TextDecoder().decode(data));
    } else {
      obj = { _raw: data };
    }

    (self as any).postMessage({
      resourceName: channel.topic,
      messageType: channel.type,
      data: obj,
    });
  } catch (err) {
    (self as any).postMessage({
      resourceName: channel.topic,
      messageType: channel.type,
      error: String(err),
    });
  }
};
