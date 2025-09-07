import type { ROSProvider, ConnectionEvents } from './provider'
import type { TopicInfo, ServiceInfo, ActionInfo, IncomingMessage } from '../types'
// add imports at top:
import { db } from '../db';
import { hashString } from '../hashing';
import type { SchemaRecord } from '../types';


// Workers (Vite will inline / chunk them)
const ConnectionWorker = new URL('../workers/connection.worker.ts', import.meta.url)


export class WSFoxgloveProvider implements ROSProvider {
    private worker?: Worker
    private events: ConnectionEvents = {}
    private ready = false


    setEvents(ev: ConnectionEvents) { this.events = ev }


    async connect(cfg: { url?: string }) {
        if (this.worker) await this.disconnect()
        this.worker = new Worker(ConnectionWorker, { type: 'module' })


        this.worker.onmessage = (e) => {
        const { type, payload } = e.data as { type: string; payload: any };
        switch (type) {
            case 'status': this.events.onStatus?.(payload); break;
            case 'topics': this.events.onTopics?.(payload as TopicInfo[]); break;
            case 'services': this.events.onServices?.(payload as ServiceInfo[]); break;
            case 'actions': this.events.onActions?.(payload as ActionInfo[]); break;
            case 'message': this.events.onMessage?.(payload as IncomingMessage); break;
            case 'schemas': persistSchemas(payload as Array<{messageType:string; definition:string; encoding:string}>); break;
            case 'ready': this.ready = true; break;
        }
        };


        this.worker.postMessage({ type: 'connect', payload: cfg })
    }


    async disconnect() {
        if (!this.worker) return
        this.worker.terminate(); this.worker = undefined; this.ready = false
        this.events.onStatus?.({ connected: false })
    }


    listTopics(): Promise<TopicInfo[]> { return this.rpc('listTopics') }
    listServices(): Promise<ServiceInfo[]> { return this.rpc('listServices') }
    listActions(): Promise<ActionInfo[]> { return this.rpc('listActions') }


    subscribe(name: string, type?: string, handlerId?: string): Promise<string> {
        return this.rpc('subscribe', { name, type, handlerId })
    }
    unsubscribe(subId: string): Promise<void> { return this.rpc('unsubscribe', { subId }) }


    callService(name: string, type: string, request: unknown): Promise<unknown> {
        return this.rpc('callService', { name, type, request })
    }


    sendActionGoal(actionName: string, type: string, goal: unknown): Promise<{ goalId: string }> {
        return this.rpc('sendActionGoal', { actionName, type, goal })
    }
    cancelActionGoal(actionName: string, goalId: string): Promise<void> {
        return this.rpc('cancelActionGoal', { actionName, goalId })
    }


    private rpc<T = any>(cmd: string, payload?: any): Promise<T> {
        return new Promise((resolve, reject) => {
            if (!this.worker) return reject(new Error('not connected'))
            const id = Math.random().toString(36).slice(2)
            const onMessage = (e: MessageEvent) => {
                const { type, rid, ok, result, error } = e.data || {}
                if (type === 'rpc' && rid === id) {
                    this.worker?.removeEventListener('message', onMessage)
                    ok ? resolve(result) : reject(new Error(error || 'RPC failed'))
                }
            }
            this.worker.addEventListener('message', onMessage)
            this.worker.postMessage({ type: 'rpc', id, cmd, payload })
        })
    }
}
async function persistSchemas(list: Array<{ messageType: string; definition: string; encoding: string }>) {
  console.log("Persisting shiiit");
  for (const s of list) {
    try {
      const h = await hashString(s.definition);
      const key = `${s.messageType}|${h}`;
      const rec: SchemaRecord = {
        key,
        messageType: s.messageType,
        encoding: (s.encoding as any) ?? 'ros2msg',
        definition: s.definition,
        createdAt: Date.now()
      };
      // idempotent upsert
      const exists = await db.schemas.get(key);
      if (!exists) await db.schemas.put(rec);
    } catch (e) {
      // non-fatal
      console.warn('schema persist failed', s.messageType, e);
    }
  }
}
