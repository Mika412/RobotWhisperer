import Dexie, { type Table } from 'dexie';

export type RequestType = 'topic' | 'service' | 'action';

export interface RosRequest {
  id?: number;
  collectionId: number | null;
  name: string;
  type: RequestType;
  target: string; // Topic, service, or action name
  interface: string;
  createdAt: Date;
}

export interface Collection {
	id?: number;
	name: string;
	parentId: number | null;
	createdAt: Date;
}

export class AppDatabase extends Dexie {
  requests!: Table<RosRequest>;
  collections!: Table<Collection>;

  constructor() {
    super('RobotWhispererDB');
    this.version(1).stores({
      // Primary key `id` will be auto-incrementing
      requests: '++id, name, type, createdAt',
      collections: '++id, parentId, name, createdAt',
    });
  }
}

export const db = new AppDatabase();
