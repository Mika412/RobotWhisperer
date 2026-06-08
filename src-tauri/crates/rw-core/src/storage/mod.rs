use crate::domain::{
    Collection, Connection, Request, RequestKind, SchemaRef, TransportConfig, Value, Visualization,
};
use crate::ids::{CollectionId, ConnectionId, RequestId};
use crate::schema::SchemaDefinition;
use crate::CoreResult;
use serde::{Deserialize, Serialize};

pub mod export;
#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(all(target_family = "wasm", feature = "wasm-storage"))]
pub mod indexeddb;

pub use export::{export_workspace, import_workspace, ImportConflict, ImportMode, ImportReport};
#[cfg(all(target_family = "wasm", feature = "wasm-storage"))]
pub use indexeddb::IdbStorage;
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteStorage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRequest {
    pub name: String,
    pub kind: RequestKind,
    pub target: String,
    pub collection_id: Option<CollectionId>,
    pub connection_id: Option<ConnectionId>,
    pub schema: Option<SchemaRef>,
    pub input: Value,
    #[serde(default)]
    pub visualization: Option<Visualization>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCollection {
    pub name: String,
    pub parent_id: Option<CollectionId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConnection {
    pub name: String,
    pub config: TransportConfig,
    pub auto_connect: bool,
    pub color: Option<String>,
}

#[cfg(not(target_family = "wasm"))]
#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    async fn list_requests(&self) -> CoreResult<Vec<Request>>;
    async fn list_requests_by_connection(
        &self,
        connection_id: ConnectionId,
    ) -> CoreResult<Vec<Request>>;
    async fn get_request(&self, id: RequestId) -> CoreResult<Option<Request>>;
    async fn create_request(&self, draft: NewRequest) -> CoreResult<Request>;
    async fn update_request(&self, request: &Request) -> CoreResult<()>;
    async fn delete_request(&self, id: RequestId) -> CoreResult<()>;

    async fn list_collections(&self) -> CoreResult<Vec<Collection>>;
    async fn create_collection(&self, draft: NewCollection) -> CoreResult<Collection>;
    async fn update_collection(&self, collection: &Collection) -> CoreResult<()>;
    async fn delete_collection(&self, id: CollectionId) -> CoreResult<()>;

    async fn list_connections(&self) -> CoreResult<Vec<Connection>>;
    async fn get_connection(&self, id: ConnectionId) -> CoreResult<Option<Connection>>;
    async fn create_connection(&self, draft: NewConnection) -> CoreResult<Connection>;
    async fn update_connection(&self, connection: &Connection) -> CoreResult<()>;
    async fn delete_connection(&self, id: ConnectionId) -> CoreResult<()>;

    async fn put_schema(&self, definition: &SchemaDefinition) -> CoreResult<()>;
    async fn get_schema(&self, hash: &str) -> CoreResult<Option<SchemaDefinition>>;
    async fn list_schemas(&self) -> CoreResult<Vec<SchemaDefinition>>;

    async fn clear_all(&self) -> CoreResult<()>;
}

#[cfg(target_family = "wasm")]
#[async_trait::async_trait(?Send)]
pub trait Storage {
    async fn list_requests(&self) -> CoreResult<Vec<Request>>;
    async fn list_requests_by_connection(
        &self,
        connection_id: ConnectionId,
    ) -> CoreResult<Vec<Request>>;
    async fn get_request(&self, id: RequestId) -> CoreResult<Option<Request>>;
    async fn create_request(&self, draft: NewRequest) -> CoreResult<Request>;
    async fn update_request(&self, request: &Request) -> CoreResult<()>;
    async fn delete_request(&self, id: RequestId) -> CoreResult<()>;

    async fn list_collections(&self) -> CoreResult<Vec<Collection>>;
    async fn create_collection(&self, draft: NewCollection) -> CoreResult<Collection>;
    async fn update_collection(&self, collection: &Collection) -> CoreResult<()>;
    async fn delete_collection(&self, id: CollectionId) -> CoreResult<()>;

    async fn list_connections(&self) -> CoreResult<Vec<Connection>>;
    async fn get_connection(&self, id: ConnectionId) -> CoreResult<Option<Connection>>;
    async fn create_connection(&self, draft: NewConnection) -> CoreResult<Connection>;
    async fn update_connection(&self, connection: &Connection) -> CoreResult<()>;
    async fn delete_connection(&self, id: ConnectionId) -> CoreResult<()>;

    async fn put_schema(&self, definition: &SchemaDefinition) -> CoreResult<()>;
    async fn get_schema(&self, hash: &str) -> CoreResult<Option<SchemaDefinition>>;
    async fn list_schemas(&self) -> CoreResult<Vec<SchemaDefinition>>;

    async fn clear_all(&self) -> CoreResult<()>;
}
