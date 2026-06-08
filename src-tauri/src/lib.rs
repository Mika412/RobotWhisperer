use std::sync::Arc;

use rw_core::schema::SchemaRegistry;
use rw_core::storage::{SqliteStorage, Storage};
use rw_core::util::{Clock, SystemClock};
use tauri::Manager;

mod commands;
mod ingest_ws;
mod pipeline;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    if std::env::var_os("RW_FORCE_NVIDIA").is_none() {
        for var in [
            "__NV_PRIME_RENDER_OFFLOAD",
            "__GLX_VENDOR_LIBRARY_NAME",
            "__VK_LAYER_NV_optimus",
        ] {
            std::env::remove_var(var);
        }
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir().expect("resolve app data dir");
            std::fs::create_dir_all(&app_data).expect("create app data dir");
            let db_path = app_data.join("workspace.db");

            let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
            let storage: Arc<dyn Storage> =
                Arc::new(SqliteStorage::open(&db_path, clock.clone()).expect("open workspace.db"));
            tracing::info!(path = %db_path.display(), "workspace storage opened");

            let registry_storage = storage.clone();
            let registry: Arc<SchemaRegistry> = tauri::async_runtime::block_on(async move {
                let registry = SchemaRegistry::new(registry_storage)
                    .await
                    .expect("build schema registry");
                registry
                    .ensure_defaults()
                    .await
                    .expect("install bundled schemas");
                Arc::new(registry)
            });
            tracing::info!("schema registry bootstrapped");

            let canonical_pipeline =
                pipeline::CanonicalPipeline::with_schema_registry(registry.clone());

            app.manage(storage);
            app.manage(clock);
            app.manage(registry);
            app.manage(canonical_pipeline);

            app.manage(ingest_ws::start());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_requests,
            get_request,
            create_request,
            update_request,
            delete_request,
            list_collections,
            create_collection,
            update_collection,
            delete_collection,
            list_connections,
            get_connection,
            create_connection,
            update_connection,
            delete_connection,
            list_schemas_summary,
            get_schema_by_hash,
            list_schemas_by_name,
            register_schema,
            export_workspace_command,
            import_workspace_command,
            clear_workspace_storage,
            pipeline_open_foxglove,
            pipeline_open_rosbridge,
            pipeline_open_dummy,
            pipeline_close,
            pipeline_status,
            pipeline_discovery,
            pipeline_subscribe_topic,
            pipeline_unsubscribe,
            pipeline_call_service,
            pipeline_send_action_goal,
            pipeline_cancel_action_goal,
            ingest_ws_port,
            set_perf_trace_enabled,
            perf_trace_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
