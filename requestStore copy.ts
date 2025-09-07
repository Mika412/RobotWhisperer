import { writable } from 'svelte/store';
import { db, type RosRequest, type Collection, type RequestType } from '$lib/db';

export const requests = writable<RosRequest[]>([]);
export const collections = writable<Collection[]>([]);

export async function loadDatabase() {
  const allRequests = await db.requests.orderBy('createdAt').toArray();
	const allCollections = await db.collections.orderBy('createdAt').toArray();
	requests.set(allRequests);
	collections.set(allCollections);
}


export async function addRequest() {
	try {
		const newRequest: RosRequest = {
			name: `New Request`,
			collectionId: null,
			type: 'topic',
			target: '',
			interface: '',
			createdAt: new Date()
		};

		// Add to Dexie DB. The 'id' will be auto-generated and returned.
		const id = await db.requests.add(newRequest);

		// Update the Svelte store with the complete request object including the new ID.
		requests.update((currentRequests) => [...currentRequests, { ...newRequest, id }]);
	} catch (error) {
		console.error('Failed to add request:', error);
	}
}


/**
 * Deletes a request from the database and then removes it from the store.
 * @param id - The ID of the request to delete.
 */
export async function deleteRequest(id: number) {
	try {
		await db.requests.delete(id);

		// Update the store by filtering out the deleted request.
		requests.update((currentRequests) => currentRequests.filter((req) => req.id !== id));
	} catch (error) {
		console.error(`Failed to delete request with id ${id}:`, error);
	}
}


export async function addCollection() {
	try {
		const newCollection: Collection = {
			name: `New Request`,
			parentId: null,
			createdAt: new Date()
		};

		// Add to Dexie DB. The 'id' will be auto-generated and returned.
		const id = await db.collections.add(newCollection);

		// Update the Svelte store with the complete collections object including the new ID.
		collections.update((currentCollections) => [...currentCollections, { ...newCollection, id }]);
	} catch (error) {
		console.error('Failed to add request:', error);
	}
}
