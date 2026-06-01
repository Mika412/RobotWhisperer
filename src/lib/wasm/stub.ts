function unreachable(): never {
  throw new Error(
    "rw-wasm stub invoked on the native Tauri shell. Build the wasm bundle with `bun run build:wasm[:dev]` to use the web shell.",
  );
}

export default async function init(): Promise<never> {
  unreachable();
}

export const WasmRobotWhisperer = {
  async create(): Promise<never> {
    unreachable();
  },
};
