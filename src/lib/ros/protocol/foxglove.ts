// Minimal Foxglove WS messages we use
export type ServerMsg =
	| { op: "serverInfo"; name: string; capabilities: string[] }
	| {
			op: "advertise";
			channels: Array<{
				id: number;
				topic: string;
				schemaName: string;
				schema: string;
				encoding: string;
			}>;
	  }
	| { op: "message"; channelId: number; data: ArrayBuffer; timestamp?: number }
	| { op: "serviceResponse"; id: number; encoding: string; data: ArrayBuffer | string; ok: boolean; error?: string; };

export type ClientMsg =
	| { op: "subscribe"; subscriptions: Array<{ id: number; channelId: number }> } // Add 'id' here
	| { op: "unsubscribe"; subscriptions: Array<{ id: number }> } // Change to use 'id'
	| { op: "callService"; id: number; service: string; encoding: string; data: ArrayBuffer | string }
	| { op: "getList"; kinds: Array<"topics" | "services"> };

export interface Channel {
	topic: string;
	type: string;
	encoding: string;
	schema: string;
	schemaEncoding: string;
}
export interface ChannelIndex {
	byId: Map<number, Channel>;
	byTopic: Map<string, number>;
}
