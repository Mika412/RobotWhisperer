import type { ActionInfo, IncomingMessage, ServiceInfo, TopicInfo } from '../types'


export type SubscribeHandler = (msg: IncomingMessage) => void
export type ConnectionEvents = {
    onTopics?: (topics: TopicInfo[]) => void
    onServices?: (services: ServiceInfo[]) => void
    onActions?: (actions: ActionInfo[]) => void
    onMessage?: SubscribeHandler
    onStatus?: (s: { connected: boolean; info?: string }) => void
}


export interface ROSProvider {
    connect(cfg: { url?: string }): Promise<void>
    disconnect(): Promise<void>
    listTopics(): Promise<TopicInfo[]>
    listServices(): Promise<ServiceInfo[]>
    listActions(): Promise<ActionInfo[]>
    subscribe(name: string, type?: string, handlerId?: string): Promise<string> // returns sub id
    unsubscribe(subId: string): Promise<void>
    callService(name: string, type: string, request: unknown): Promise<unknown>
    sendActionGoal(actionName: string, type: string, goal: unknown): Promise<{ goalId: string }>
    cancelActionGoal(actionName: string, goalId: string): Promise<void>
    setEvents(ev: ConnectionEvents): void
}