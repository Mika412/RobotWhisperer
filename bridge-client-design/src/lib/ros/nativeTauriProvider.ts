import type { ROSProvider, ConnectionEvents } from './provider'
import type { TopicInfo, ServiceInfo, ActionInfo, IncomingMessage } from '../types'


export class NativeTauriProvider implements ROSProvider {
    private events: ConnectionEvents = {}
    setEvents(ev: ConnectionEvents) { this.events = ev }


    async connect() { this.events.onStatus?.({ connected: true, info: 'native (stub)' }) }
    async disconnect() { this.events.onStatus?.({ connected: false }) }


    async listTopics(): Promise<TopicInfo[]> {
        // @ts-ignore
        const { invoke } = window.__TAURI__?.core || {}
        const out = (await invoke?.('native_list_topics')) || [{ name: '/dummy', type: 'std_msgs/String' }]
        return out
    }
    async listServices(): Promise<ServiceInfo[]> { return [] }
    async listActions(): Promise<ActionInfo[]> { return [] }


    async subscribe(name: string): Promise<string> { return name }
    async unsubscribe(): Promise<void> { }
    async callService(): Promise<unknown> { return {} }
    async sendActionGoal(): Promise<{ goalId: string }> { return { goalId: 'dummy' } }
    async cancelActionGoal(): Promise<void> { }
}