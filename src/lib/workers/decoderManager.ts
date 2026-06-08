import { isTauri } from "$lib/core/platform";
import type { DecodedFramePayload } from "./decoder.worker";

export type DecodedFrame = DecodedFramePayload;

type DecodedCallback = (frame: DecodedFrame) => void;

export interface StreamMapping {
  handle: string;
  schemaId: string;
  schemaName: string;
  vizRole: string;
}

class DecoderWorkerManager {
  private worker: Worker | null = null;
  private callbacks = new Map<string, DecodedCallback>();
  private ingestUrl: string | null = null;
  private streamHandles = new Map<string, StreamMapping>();

  private ensureWorker(): Worker | null {
    if (this.worker) return this.worker;
    if (!isTauri() && typeof Worker === "undefined") return null;
    try {
      this.worker = new Worker(new URL("./decoder.worker.ts", import.meta.url), { type: "module" });
      this.worker.onmessage = (event: MessageEvent) => this.onWorkerMessage(event);
      this.worker.onerror = (event: ErrorEvent) => {
        console.error(
          "[decoderManager] worker crashed:",
          event.message,
          event.filename,
          event.lineno,
          event.error,
        );
        this.recreateWorker();
      };
      this.worker.onmessageerror = (event: MessageEvent) => {
        console.error("[decoderManager] worker postMessage deserialization failed:", event);
        this.recreateWorker();
      };
      if (this.ingestUrl) {
        this.worker.postMessage({ type: "connectIngest", url: this.ingestUrl });
        for (const [streamKey, mapping] of this.streamHandles) {
          this.worker.postMessage({ type: "mapStream", streamKey, ...mapping });
        }
      }
    } catch {
      this.worker = null;
    }
    return this.worker;
  }

  connectIngest(url: string): void {
    this.ingestUrl = url;
    const worker = this.ensureWorker();
    if (worker) worker.postMessage({ type: "connectIngest", url });
  }

  mapStream(streamKey: string, mapping: StreamMapping): void {
    this.streamHandles.set(streamKey, mapping);
    const worker = this.ensureWorker();
    if (worker) worker.postMessage({ type: "mapStream", streamKey, ...mapping });
  }

  unmapStream(streamKey: string, handle: string): void {
    this.streamHandles.delete(streamKey);
    this.worker?.postMessage({ type: "unmapStream", handle });
  }

  private recreateWorker(): void {
    try {
      this.worker?.terminate();
    } catch {}
    this.worker = null;
    this.ensureWorker();
  }

  registerStream(streamKey: string, callback: DecodedCallback): void {
    this.callbacks.set(streamKey, callback);
    this.ensureWorker();
  }

  unregisterStream(streamKey: string): void {
    this.callbacks.delete(streamKey);
  }

  destroy(): void {
    this.worker?.terminate();
    this.worker = null;
    this.callbacks.clear();
  }

  private onWorkerMessage(event: MessageEvent): void {
    const data = event.data as { type: string } & Record<string, unknown>;
    if (data.type === "diagnostic") {
      console.warn("[decoder worker]", data.message);
      return;
    }
    if (data.type === "decoded") {
      const callback = this.callbacks.get(data.streamKey as string);
      if (callback) callback(data.frame as DecodedFrame);
    }
  }
}

export const decoderWorker = new DecoderWorkerManager();
