// src/lib/stores/tabStore.ts
import { writable, get } from 'svelte/store';

/** A store that holds an array of the IDs of all currently open requests (tabs). */
export const openTabIds = writable<number[]>([]);

/** A store that holds the ID of the currently active tab. */
export const activeTabId = writable<number | undefined>(undefined);

/**
 * Opens a request in a new tab.
 * If the tab is already open, it just makes it active.
 * @param requestId - The ID of the request to open.
 */
export function openTab(requestId: number) {
	const tabs = get(openTabIds);
	const isAlreadyOpen = tabs.includes(requestId);

	if (!isAlreadyOpen) {
		openTabIds.update((currentIds) => [...currentIds, requestId]);
	}


	activeTabId.set(requestId);
}

/**
 * Closes a specific tab.
 * If the closed tab was the active one, it activates the next available tab.
 * @param idToClose - The ID of the request/tab to close.
 */
export function closeTab(idToClose: number) {
	const currentActiveId = get(activeTabId);
	const currentOpenIds = get(openTabIds);
	const tabIndex = currentOpenIds.findIndex((id) => id === idToClose);

	// Remove the tab from the list
	openTabIds.update((ids) => ids.filter((id) => id !== idToClose));

	// If the closed tab was the active one, determine the next active tab
	if (currentActiveId === idToClose && tabIndex !== -1) {
		const remainingIds = get(openTabIds);
		if (remainingIds.length > 0) {
			// Activate the tab at the same index (which is now the next tab),
			// or the last tab if the closed one was the last in the list.
			const newActiveIndex = Math.min(tabIndex, remainingIds.length - 1);
			activeTabId.set(remainingIds[newActiveIndex]);
		} else {
			// No tabs left, so no active tab.
			activeTabId.set(undefined);
		}
	}
}
