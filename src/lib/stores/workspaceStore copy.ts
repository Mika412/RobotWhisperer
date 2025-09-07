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
 * Marks the currently active item as dirty.
 */
export function markActiveItemAsDirty() {
  const activeId = get(activeItemId);
  if (!activeId) return;

  openItems.update((items) => {
    return items.map((item) => {
      if (item.id === activeId) {
        if (!item.isDirty) {
          return { ...item, isDirty: true };
        }
      }
      return item;
    });
  });
}

/**
 * Saves the current changes in the active item to the database.
 */
export async function saveActiveItem() {
  const itemToSave = get(activeItem);
  if (!itemToSave || !itemToSave.isDirty) return;

  try {
    // Persist changes to the database and master request list
    await updateRequest(itemToSave.data);

    // After saving, find the updated original request from the main store
    const originalRequest = get(requests).find((req) => req.id === itemToSave.id);
    if (!originalRequest) throw new Error("Original request not found after saving");

    // Update the item in the workspace to mark it as not dirty and sync its data
    openItems.update((items) =>
      items.map((item) =>
        item.id === itemToSave.id
          ? {
              ...item,
              isDirty: false,
              // Sync the draft data with the now-saved data to ensure consistency
              data: JSON.parse(JSON.stringify(originalRequest)),
            }
          : item
      )
    );
  } catch (error) {
    console.error("Failed to save item:", error);
  }
}
