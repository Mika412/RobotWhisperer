export interface TopicInfo {
	name: string;
	type: string;
}
export interface ServiceInfo {
	name: string;
	type: string;
}
export interface ActionInfo {
	name: string;
	type: string;
}

export interface IncomingMessage {
	resourceName: string;
	messageType: string;
	stamp?: number;
	raw?: ArrayBuffer;
	data?: any;
	error?: string;
}

export interface SchemaRecord {
	key: string; // `${messageType}|${hash}`
	messageType: string;
	encoding: "ros2msg" | "ros2idl" | "jsonschema" | "protobuf" | "flatbuffers" | "unknown";
	definition: string; // raw text or base64 JSON descriptor
	createdAt: number;
}
