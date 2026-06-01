use std::sync::Arc;

use rw_core::domain::{Collection, Connection, Request, WorkspaceFile};
use rw_core::ids::{CollectionId, ConnectionId, RequestId};
use rw_core::storage::{
    export_workspace, import_workspace, ImportMode, ImportReport, NewCollection, NewConnection,
    NewRequest, Storage,
};
use rw_core::util::Clock;
use rw_core::{CoreError, CoreResult};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct RpcError {
    pub kind: &'static str,
    pub message: String,
}

impl From<CoreError> for RpcError {
    fn from(error: CoreError) -> Self {
        let kind = match &error {
            CoreError::Storage(_) => "storage",
            CoreError::Schema(_) => "schema",
            CoreError::Transport(_) => "transport",
            CoreError::NotFound(_) => "not_found",
            CoreError::InvalidArgument(_) => "invalid_argument",
            CoreError::Conflict(_) => "conflict",
            CoreError::Io(_) => "io",
            CoreError::Serde(_) => "serde",
        };
        Self {
            kind,
            message: error.to_string(),
        }
    }
}

pub type RpcResult<T> = Result<T, RpcError>;

pub(crate) fn to_rpc<T>(value: CoreResult<T>) -> RpcResult<T> {
    value.map_err(RpcError::from)
}

pub type StorageState<'a> = State<'a, Arc<dyn Storage>>;
pub type ClockState<'a> = State<'a, Arc<dyn Clock>>;

#[tauri::command]
pub async fn list_requests(storage: StorageState<'_>) -> RpcResult<Vec<Request>> {
    to_rpc(storage.list_requests().await)
}

#[tauri::command]
pub async fn get_request(storage: StorageState<'_>, id: RequestId) -> RpcResult<Option<Request>> {
    to_rpc(storage.get_request(id).await)
}

#[tauri::command]
pub async fn create_request(storage: StorageState<'_>, draft: NewRequest) -> RpcResult<Request> {
    to_rpc(storage.create_request(draft).await)
}

#[tauri::command]
pub async fn update_request(storage: StorageState<'_>, request: Request) -> RpcResult<Request> {
    to_rpc(storage.update_request(&request).await)?;
    let refreshed = to_rpc(storage.get_request(request.id).await)?;
    refreshed.ok_or(RpcError {
        kind: "not_found",
        message: format!("request {} disappeared mid-update", request.id),
    })
}

#[tauri::command]
pub async fn delete_request(storage: StorageState<'_>, id: RequestId) -> RpcResult<()> {
    to_rpc(storage.delete_request(id).await)
}

#[tauri::command]
pub async fn list_collections(storage: StorageState<'_>) -> RpcResult<Vec<Collection>> {
    to_rpc(storage.list_collections().await)
}

#[tauri::command]
pub async fn create_collection(
    storage: StorageState<'_>,
    draft: NewCollection,
) -> RpcResult<Collection> {
    to_rpc(storage.create_collection(draft).await)
}

#[tauri::command]
pub async fn update_collection(storage: StorageState<'_>, collection: Collection) -> RpcResult<()> {
    to_rpc(storage.update_collection(&collection).await)
}

#[tauri::command]
pub async fn delete_collection(storage: StorageState<'_>, id: CollectionId) -> RpcResult<()> {
    to_rpc(storage.delete_collection(id).await)
}

#[tauri::command]
pub async fn list_connections(storage: StorageState<'_>) -> RpcResult<Vec<Connection>> {
    to_rpc(storage.list_connections().await)
}

#[tauri::command]
pub async fn get_connection(
    storage: StorageState<'_>,
    id: ConnectionId,
) -> RpcResult<Option<Connection>> {
    to_rpc(storage.get_connection(id).await)
}

#[tauri::command]
pub async fn create_connection(
    storage: StorageState<'_>,
    draft: NewConnection,
) -> RpcResult<Connection> {
    to_rpc(storage.create_connection(draft).await)
}

#[tauri::command]
pub async fn update_connection(storage: StorageState<'_>, connection: Connection) -> RpcResult<()> {
    to_rpc(storage.update_connection(&connection).await)
}

#[tauri::command]
pub async fn delete_connection(storage: StorageState<'_>, id: ConnectionId) -> RpcResult<()> {
    to_rpc(storage.delete_connection(id).await)
}

#[tauri::command]
pub async fn export_workspace_command(
    storage: StorageState<'_>,
    clock: ClockState<'_>,
) -> RpcResult<String> {
    let file = to_rpc(export_workspace(storage.inner().as_ref(), clock.inner().clone()).await)?;
    serde_json::to_string_pretty(&file).map_err(|err| RpcError {
        kind: "serde",
        message: err.to_string(),
    })
}

#[tauri::command]
pub async fn import_workspace_command(
    storage: StorageState<'_>,
    file: WorkspaceFile,
    mode: ImportMode,
) -> RpcResult<ImportReport> {
    to_rpc(import_workspace(storage.inner().as_ref(), file, mode).await)
}

#[tauri::command]
pub async fn clear_workspace_storage(storage: StorageState<'_>) -> RpcResult<()> {
    to_rpc(storage.clear_all().await)
}
