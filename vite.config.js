import path from "node:path";
import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

import urdfManifest from "./vite/urdf-manifest";
import meshOptimize from "./vite/mesh-optimize";

const host = process.env.TAURI_DEV_HOST;
const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Native builds redirect the wasm-pack specifier to a stub so the bundler
// resolves cleanly even when `src/lib/wasm/generated/` does not exist.
// SvelteKit's `$lib` resolver runs first and rewrites `$lib/wasm/...` to an
// absolute on-disk path, so the alias has to match on the resolved path rather
// than the original `$lib/...` specifier. Only the `web`/`build` scripts set
// `RW_TARGET=web`; everything else (tauri dev/build, plain `dev`) is native.
const isWebTarget = process.env.RW_TARGET === "web";
const wasmGeneratedPath = path.resolve(__dirname, "src/lib/wasm/generated/rw_wasm");
const wasmStubPath = path.resolve(__dirname, "src/lib/wasm/stub.ts");

/** @type {import('vite').Plugin | null} */
const rwNativeWasmStubPlugin = isWebTarget
  ? null
  : {
      name: "rw-native-wasm-stub",
      enforce: "pre",
      resolveId(source) {
        if (
          source === "$lib/wasm/generated/rw_wasm" ||
          source === wasmGeneratedPath ||
          source.endsWith("/wasm/generated/rw_wasm")
        ) {
          return wasmStubPath;
        }
        return null;
      },
    };

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    rwNativeWasmStubPlugin,
    urdfManifest(),
    meshOptimize(),
    tailwindcss(),
    sveltekit(),
  ].filter(Boolean),
  // Build-target constant. `import.meta.env.RW_WEB` is `true` only for the web
  // shell and `false` for native. The RPC dispatch branches on this
  // compile-time constant so Vite dead-code-eliminates the wrong implementation
  // per build: the native bundle never imports the WASM module, and the web
  // bundle never imports `@tauri-apps/api`.
  define: {
    "import.meta.env.RW_WEB": JSON.stringify(isWebTarget),
  },
  // Vite options tailored for Tauri development, only applied in `tauri dev` or
  // `tauri build`.
  clearScreen: false,
  server: {
    // Tauri requires a fixed port (1420). For a plain browser dev server use
    // `bun run web`, which overrides via --port.
    port: 1420,
    strictPort: !!host,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
