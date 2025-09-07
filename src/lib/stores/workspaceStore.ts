import { writable, derived, get } from "svelte/store";
import { requests, updateRequest } from "$lib/stores/requestStore";
import type { RosRequest } from "$lib/db";

export interface WorkspaceItem {
  id: number; // The ID of the original request
  data: RosRequest; // The current, potentially modified, data (the "draft")
  isDirty: boolean;
}

export const openItems = writable<WorkspaceItem[]>([]);
export const activeItemId = writable<number | null>(null);
export const activeItem = derived(
  [openItems, activeItemId],
  ([$openItems, $activeItemId]) => {
    return $openItems.find((item) => item.id === $activeItemId) ?? null;
  },
);

/**
 * Updates a specific field of the active item's data and marks it as dirty.
 * @param field - The key of the data object to update.
 * @param value - The new value for the field.
 */
export function updateActiveItemField(field: keyof RosRequest, value: any) {
	openItems.update((items) => {
		const activeId = get(activeItemId);
		return items.map((item) => {
			if (item.id === activeId) {
				// Create a new data object with the updated field to ensure reactivity
				const newData = { ...item.data, [field]: value };
				return { ...item, data: newData, isDirty: true };
			}
			return item;
		});
	});
}


/**
 * Opens a request in the workspace.
 * If already open, it just becomes the active item.
 * @param itemId - The ID of the request to open.
 */
export function openItem(itemId: number) {
  const currentOpenItems = get(openItems);
  if (currentOpenItems.some((item) => item.id === itemId)) {
    // Item is already open, just make it active
    activeItemId.set(itemId);
    return;
  }

  const originalRequest = get(requests).find((req) => req.id === itemId);
  if (!originalRequest) return;

  // Create a new workspace item with a deep copy of the request data
  const newItem: WorkspaceItem = {
    id: itemId,
    data: JSON.parse(JSON.stringify(originalRequest)), // Deep copy
    isDirty: false,
  };

  openItems.update((items) => [...items, newItem]);
  activeItemId.set(itemId);
}

/**
 * Closes an item in the workspace.
 * @param itemId - The ID of the item to close.
 */
export function closeItem(itemId: number) {
  openItems.update((items) => items.filter((item) => item.id !== itemId));

  // If the closed item was the active one, select another one
  if (get(activeItemId) === itemId) {
    const remainingItems = get(openItems);
    activeItemId.set(remainingItems.length > 0 ? remainingItems[0].id : null);
  }
}

/**
 * Saves the changes from the active workspace item back to the database.
 */
export async function saveActiveItem() {
	const itemToSave = get(activeItem);
	if (!itemToSave || !itemToSave.isDirty) return;

	await updateRequest(itemToSave.data);

	// After saving, mark the item as not dirty anymore
	openItems.update((items) =>
		items.map((item) => {
			if (item.id === itemToSave.id) {
				return { ...item, isDirty: false };
			}
			return item;
		}),
	);
}

