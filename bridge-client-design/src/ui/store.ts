import { writable } from 'svelte/store'
import type { SavedRequest, TopicInfo, ServiceInfo, ActionInfo, IncomingMessage } from '../lib/types'


export const connection = writable<{ connected: boolean; info?: string; source: 'ws'|'native' }>()
export const topics = writable<TopicInfo[]>([])
export const services = writable<ServiceInfo[]>([])
export const actions = writable<ActionInfo[]>([])


export const openTabs = writable<SavedRequest[]>([])
export const activeTabId = writable<string | null>(null)


// last message per resource
export const latest = writable<Record<string, IncomingMessage>>({})