declare module "$lib/wasm/generated/rw_wasm" {
  const init: (input?: unknown) => Promise<unknown>;
  export default init;

  interface WasmRobotWhispererStatics {
    create(): Promise<unknown>;
  }
  export const WasmRobotWhisperer: WasmRobotWhispererStatics;
}
