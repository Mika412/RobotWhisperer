import type { ROSProvider, ConnectionEvents } from "./provider";
import type { TopicInfo, ServiceInfo, ActionInfo, IncomingMessage, SchemaRecord } from "./types";
import { db } from "$lib/db"; // Assuming you want to use the main app's Dexie instance

export class WSFoxgloveProvider implements ROSProvider {
	private worker?: Worker;
	private events: ConnectionEvents = {};

	setEvents(ev: ConnectionEvents) {
		this.events = ev;
	}

	async connect(cfg: { url?: string }) {
		if (this.worker) await this.disconnect();
		// IMPORTANT: Update the path to the worker
		const ConnectionWorker = new URL("../workers/connection.worker.ts", import.meta.url);
		this.worker = new Worker(ConnectionWorker, { type: "module" });

		this.worker.onmessage = (e) => {
			const { type, payload } = e.data as { type: string; payload: any };
			switch (type) {
				case "status":
					this.events.onStatus?.(payload);
					break;
				case "topics":
					this.events.onTopics?.(payload as TopicInfo[]);
					break;
				// ... other cases remain the same
			}
		};

		this.worker.postMessage({ type: "connect", payload: cfg });
	}

	async disconnect() {
		if (!this.worker) return;
		this.worker.terminate();
		this.worker = undefined;
		this.events.onStatus?.({ connected: false });
	}

    // ... (rest of the provider methods like listTopics, subscribe, etc. remain the same)
}
