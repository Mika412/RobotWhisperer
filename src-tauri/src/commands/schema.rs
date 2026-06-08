use std::sync::Arc;

use rw_core::domain::SchemaRef;
use rw_core::schema::{SchemaDefinition, SchemaKind, SchemaRegistry, SchemaSummary};
use tauri::State;

use super::workspace::{to_rpc, RpcResult};

pub type RegistryState<'a> = State<'a, Arc<SchemaRegistry>>;

#[tauri::command]
pub async fn list_schemas_summary(registry: RegistryState<'_>) -> RpcResult<Vec<SchemaSummary>> {
    Ok(registry.list_summaries())
}

#[tauri::command]
pub async fn get_schema_by_hash(
    registry: RegistryState<'_>,
    hash: String,
) -> RpcResult<Option<SchemaDefinition>> {
    Ok(registry.get_by_hash(&hash))
}

#[tauri::command]
pub async fn list_schemas_by_name(
    registry: RegistryState<'_>,
    name: String,
) -> RpcResult<Vec<SchemaDefinition>> {
    Ok(registry.get_by_name(&name))
}

#[tauri::command]
pub async fn register_schema(
    registry: RegistryState<'_>,
    name: String,
    kind: SchemaKind,
    definition: String,
) -> RpcResult<SchemaRef> {
    to_rpc(registry.register(&name, kind, &definition).await)
}
