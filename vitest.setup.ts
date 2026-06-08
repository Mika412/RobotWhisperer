// `fake-indexeddb/auto` patches `globalThis.indexedDB` so jsdom-hosted tests
// can exercise Dexie-backed code without a real browser. Pulled in once at
// module load so individual test files don't need to import it.
import "fake-indexeddb/auto";
