import { writable } from "svelte/store";
import {
  db,
  type RosRequest,
  type RequestType,
} from "$lib/db";
import { closeItem, openItem } from './workspaceStore';

export const requests = writable<RosRequest[]>([]);

export async function loadDatabase() {
  const allRequests = await db.requests.orderBy("createdAt").toArray();
  requests.set(allRequests);
}

export async function addRequest() {
  try {
    const newRequest: RosRequest = {
      name: `New Request`,
      collectionId: null,
      type: "topic",
      target: "",
      interface: "",
      createdAt: new Date(),
    };

    // Add to Dexie DB. The 'id' will be auto-generated and returned.
    const id = await db.requests.add(newRequest);

    // Update the Svelte store with the complete request object including the new ID.
    requests.update((currentRequests) => [
      ...currentRequests,
      { ...newRequest, id },
    ]);

    openItem(id);
  } catch (error) {
    console.error("Failed to add request:", error);
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
    requests.update((currentRequests) =>
      currentRequests.filter((req) => req.id !== id),
    );

    closeItem(id);
  } catch (error) {
    console.error(`Failed to delete request with id ${id}:`, error);
  }
}

/**
 * Updates an existing request in the database and the store.
 * @param requestData - The full request object to update (must include id).
 */
export async function updateRequest(requestData: RosRequest) {
  if (!requestData.id) {
    throw new Error("Cannot update request without an ID.");
  }

  try {
    // Use 'put' which updates an existing item or adds it if it doesn't exist.
    await db.requests.put(requestData);

    // Update the master list of requests in the store
    requests.update((currentRequests) =>
      currentRequests.map((req) =>
        req.id === requestData.id ? requestData : req
      )
    );
  } catch (error) {
    console.error(`Failed to update request with id ${requestData.id}:`, error);
    throw error; // Re-throw to be handled by the caller
  }
}

// /**
//  * Updates an existing request in the database and the Svelte store.
//  * @param id - The ID of the request to update.
//  * @param updates - An object containing the fields to update.
//  */
// export async function updateRequest(id: number, updates: Partial<RosRequest>) {
// 	try {
// 		await db.requests.update(id, updates);

// 		// Also update the request in our in-memory store for immediate UI feedback.
// 		requests.update((currentRequests) =>
// 			currentRequests.map((req) => (req.id === id ? { ...req, ...updates } : req))
// 		);
// 	} catch (error) {
// 		console.error(`Failed to update request ${id}:`, error);
// 	}
// }
