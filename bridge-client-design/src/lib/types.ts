export type RequestType = 'topic' | 'service' | 'action'


export interface SavedRequest {
    id: string
    name: string
    kind: RequestType
    resourceName: string // topic/service/action name
    messageType: string // e.g. sensor_msgs/Image
    schemaKey?: string // content-hash key of schema used
    payload?: unknown // last used request body for service/action
    createdAt: number
    updatedAt: number
}


export interface SchemaRecord {
    key: string // `${messageType}|${hash}`
    messageType: string
    encoding: 'ros2msg' | 'ros2idl' | 'jsonschema' | 'protobuf' | 'flatbuffers'
    definition: string // raw text or base64 JSON descriptor
    createdAt: number
}


export interface TopicInfo { name: string; type: string; }
export interface ServiceInfo { name: string; type: string; }
export interface ActionInfo { name: string; type: string; }


export interface ConnectionCapabilities {
    topics: boolean; services: boolean; actions: boolean
}


export interface IncomingMessage {
    resourceName: string
    messageType: string
    stamp?: number
    raw?: ArrayBuffer
    data?: any
}