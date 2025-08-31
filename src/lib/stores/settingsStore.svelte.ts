import { browser } from '$app/environment';

function getFromStorage(key: string, defaultValue: any) {
    if (!browser) {
        return defaultValue;
    }
    const storedValue = localStorage.getItem(key);
    if (storedValue) {
        try {
            return JSON.parse(storedValue);
        } catch (e) {
            console.error(`Error parsing localStorage key "${key}":`, e);
            return defaultValue;
        }
    }
    return defaultValue;
}

export let settings = $state(getFromStorage('settings', {
    theme: 'dark'
}));
