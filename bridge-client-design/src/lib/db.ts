import Dexie, { type Table } from 'dexie'
import type { SavedRequest, SchemaRecord } from './types'


export class AppDB extends Dexie {
    requests!: Table<SavedRequest, string>
    schemas!: Table<SchemaRecord, string>


    constructor() {
        super('ros-postman-db')
        this.version(1).stores({
            requests: '&id, kind, resourceName, messageType, updatedAt, createdAt', // <-- Add it here
            schemas: '&key, messageType, encoding, createdAt'
        })
    }
}


export const db = new AppDB()