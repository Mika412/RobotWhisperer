import { isTauri } from "./platform";

let observerCount = 0;

async function setBackendEnabled(enabled: boolean): Promise<void> {
  if (import.meta.env.RW_WEB) {
    try {
      const { getWasmInstance } = await import("./pipelineRpc");
      const wasm = await getWasmInstance();
      wasm.setPerfTraceEnabled(enabled);
    } catch (err) {
      console.warn("[diagnostics] setPerfTraceEnabled (wasm) failed", err);
    }
    return;
  }
  if (!isTauri()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke<void>("set_perf_trace_enabled", { enabled });
}

export function acquirePerfTrace(): () => void {
  observerCount += 1;
  if (observerCount === 1) {
    void setBackendEnabled(true);
  }
  let released = false;
  return () => {
    if (released) return;
    released = true;
    observerCount = Math.max(0, observerCount - 1);
    if (observerCount === 0) {
      void setBackendEnabled(false);
    }
  };
}

export function deltaMs(a: number, b: number): number {
  if (!a || !b) return NaN;
  return (b - a) / 1e6;
}
