import { browser } from "$app/environment";
import { DEFAULT_THEME } from "$lib/themes";

function getFromStorage<T>(key: string, defaultValue: T): T {
  if (!browser) {
    return defaultValue;
  }
  const storedValue = localStorage.getItem(key);
  if (storedValue) {
    try {
      return JSON.parse(storedValue) as T;
    } catch (error) {
      console.error(`Error parsing localStorage key "${key}":`, error);
      return defaultValue;
    }
  }
  return defaultValue;
}

export const settings = $state(
  getFromStorage("settings", {
    theme: DEFAULT_THEME,
  }),
);
