use rw_wire;

#[tauri::command]
pub fn set_perf_trace_enabled(enabled: bool) {
    rw_wire::set_perf_trace_enabled(enabled);
}

#[tauri::command]
pub fn perf_trace_enabled() -> bool {
    rw_wire::perf_trace_enabled()
}
