use idb::{
    Database, DatabaseEvent, Factory, IndexParams, KeyPath, ObjectStoreParams, TransactionMode,
};
use wasm_bindgen::JsValue;

use crate::domain::{Collection, Connection, Request};
use crate::ids::{CollectionId, ConnectionId, RequestId};
use crate::schema::SchemaDefinition;
use crate::storage::{NewCollection, NewConnection, NewRequest, Storage};
use crate::{CoreError, CoreResult};

const DB_NAME: &str = "RobotWhispererWorkspace";
const DB_VERSION: u32 = 1;

const REQUESTS: &str = "requests";
const COLLECTIONS: &str = "collections";
const CONNECTIONS: &str = "connections";
const SCHEMAS: &str = "schemas";

#[derive(Debug)]
pub struct IdbStorage {
    db: Database,
}

impl IdbStorage {
    pub async fn open() -> CoreResult<Self> {
        let factory = Factory::new().map_err(idb_err)?;
        let mut request = factory.open(DB_NAME, Some(DB_VERSION)).map_err(idb_err)?;
        request.on_upgrade_needed(|event| {
            if let Err(err) = upgrade(event) {
                tracing::error!(?err, "idb upgrade failed");
            }
        });
        let db = request.await.map_err(idb_err)?;
        Ok(IdbStorage { db })
    }

    fn rw(&self, stores: &[&str]) -> CoreResult<idb::Transaction> {
        self.db
            .transaction(stores, TransactionMode::ReadWrite)
            .map_err(idb_err)
    }

    fn ro(&self, stores: &[&str]) -> CoreResult<idb::Transaction> {
        self.db
            .transaction(stores, TransactionMode::ReadOnly)
            .map_err(idb_err)
    }
}

fn upgrade(event: idb::event::VersionChangeEvent) -> CoreResult<()> {
    let db = event.database().map_err(idb_err)?;

    let mut params = ObjectStoreParams::new();
    params.auto_increment(true);
    params.key_path(Some(KeyPath::new_single("id")));
    db.create_object_store(REQUESTS, params.clone())
        .map_err(idb_err)?;
    db.create_object_store(COLLECTIONS, params.clone())
        .map_err(idb_err)?;

    let connections = db
        .create_object_store(CONNECTIONS, params)
        .map_err(idb_err)?;
    let mut idx_params = IndexParams::new();
    idx_params.unique(true);
    connections
        .create_index("name", KeyPath::new_single("name"), Some(idx_params))
        .map_err(idb_err)?;

    let mut schema_params = ObjectStoreParams::new();
    schema_params.key_path(Some(KeyPath::new_single("hash")));
    db.create_object_store(SCHEMAS, schema_params)
        .map_err(idb_err)?;
    Ok(())
}

fn idb_err<E: std::fmt::Display>(err: E) -> CoreError {
    CoreError::Storage(format!("idb: {err}"))
}

fn to_js<T: serde::Serialize>(value: &T) -> CoreResult<JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(|err| CoreError::Storage(err.to_string()))
}

fn from_js<T: serde::de::DeserializeOwned>(value: JsValue) -> CoreResult<T> {
    serde_wasm_bindgen::from_value(value).map_err(|err| CoreError::Storage(err.to_string()))
}

async fn commit(tx: idb::Transaction) -> CoreResult<()> {
    let _result = tx.commit().map_err(idb_err)?.await.map_err(idb_err)?;
    Ok(())
}

#[async_trait::async_trait(?Send)]
impl Storage for IdbStorage {
    async fn list_requests(&self) -> CoreResult<Vec<Request>> {
        let tx = self.ro(&[REQUESTS])?;
        let store = tx.object_store(REQUESTS).map_err(idb_err)?;
        let all = store
            .get_all(None, None)
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        let mut out = Vec::with_capacity(all.len());
        for entry in all {
            out.push(from_js(entry)?);
        }
        Ok(out)
    }

    async fn list_requests_by_connection(
        &self,
        connection_id: ConnectionId,
    ) -> CoreResult<Vec<Request>> {
        let all = self.list_requests().await?;
        Ok(all
            .into_iter()
            .filter(|r| r.connection_id == Some(connection_id))
            .collect())
    }

    async fn get_request(&self, id: RequestId) -> CoreResult<Option<Request>> {
        let tx = self.ro(&[REQUESTS])?;
        let store = tx.object_store(REQUESTS).map_err(idb_err)?;
        let value = store
            .get(JsValue::from_f64(id as f64))
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        match value {
            Some(v) => Ok(Some(from_js(v)?)),
            None => Ok(None),
        }
    }

    async fn create_request(&self, draft: NewRequest) -> CoreResult<Request> {
        let now = chrono::Utc::now();
        let candidate = Request {
            id: 0,
            collection_id: draft.collection_id,
            connection_id: draft.connection_id,
            name: draft.name,
            kind: draft.kind,
            target: draft.target,
            schema: draft.schema,
            input: draft.input,
            visualization: draft.visualization,
            created_at: now,
            updated_at: now,
        };
        let tx = self.rw(&[REQUESTS])?;
        let store = tx.object_store(REQUESTS).map_err(idb_err)?;
        let js = to_js(&candidate)?;
        let _ = js_sys::Reflect::delete_property(
            js.unchecked_ref::<js_sys::Object>(),
            &JsValue::from_str("id"),
        );
        let key = store
            .add(&js, None)
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        let id = key
            .as_f64()
            .ok_or_else(|| CoreError::Storage("non-numeric key".into()))? as i64;
        commit(tx).await?;
        Ok(Request { id, ..candidate })
    }

    async fn update_request(&self, request: &Request) -> CoreResult<()> {
        let mut updated = request.clone();
        updated.updated_at = chrono::Utc::now();
        let tx = self.rw(&[REQUESTS])?;
        let store = tx.object_store(REQUESTS).map_err(idb_err)?;
        store
            .put(&to_js(&updated)?, None)
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        commit(tx).await
    }

    async fn delete_request(&self, id: RequestId) -> CoreResult<()> {
        let tx = self.rw(&[REQUESTS])?;
        let store = tx.object_store(REQUESTS).map_err(idb_err)?;
        store
            .delete(idb::Query::from(JsValue::from_f64(id as f64)))
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        commit(tx).await
    }

    async fn list_collections(&self) -> CoreResult<Vec<Collection>> {
        let tx = self.ro(&[COLLECTIONS])?;
        let store = tx.object_store(COLLECTIONS).map_err(idb_err)?;
        let all = store
            .get_all(None, None)
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        let mut out = Vec::with_capacity(all.len());
        for entry in all {
            out.push(from_js(entry)?);
        }
        Ok(out)
    }

    async fn create_collection(&self, draft: NewCollection) -> CoreResult<Collection> {
        let now = chrono::Utc::now();
        let candidate = Collection {
            id: 0,
            parent_id: draft.parent_id,
            name: draft.name,
            created_at: now,
        };
        let tx = self.rw(&[COLLECTIONS])?;
        let store = tx.object_store(COLLECTIONS).map_err(idb_err)?;
        let js = to_js(&candidate)?;
        let _ = js_sys::Reflect::delete_property(
            js.unchecked_ref::<js_sys::Object>(),
            &JsValue::from_str("id"),
        );
        let key = store
            .add(&js, None)
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        let id = key
            .as_f64()
            .ok_or_else(|| CoreError::Storage("non-numeric key".into()))? as i64;
        commit(tx).await?;
        Ok(Collection { id, ..candidate })
    }

    async fn update_collection(&self, collection: &Collection) -> CoreResult<()> {
        let tx = self.rw(&[COLLECTIONS])?;
        let store = tx.object_store(COLLECTIONS).map_err(idb_err)?;
        store
            .put(&to_js(collection)?, None)
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        commit(tx).await
    }

    async fn delete_collection(&self, id: CollectionId) -> CoreResult<()> {
        let tx = self.rw(&[COLLECTIONS, REQUESTS])?;
        let collections = tx.object_store(COLLECTIONS).map_err(idb_err)?;
        let requests = tx.object_store(REQUESTS).map_err(idb_err)?;

        let all_collections = collections
            .get_all(None, None)
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        let mut to_delete = vec![id];
        let mut queue = vec![id];
        let parsed: Vec<Collection> = all_collections
            .into_iter()
            .map(from_js)
            .collect::<CoreResult<_>>()?;
        while let Some(next) = queue.pop() {
            for c in &parsed {
                if c.parent_id == Some(next) && !to_delete.contains(&c.id) {
                    to_delete.push(c.id);
                    queue.push(c.id);
                }
            }
        }

        let all_requests = requests
            .get_all(None, None)
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        for r_js in all_requests {
            let mut r: Request = from_js(r_js)?;
            if let Some(cid) = r.collection_id {
                if to_delete.contains(&cid) {
                    r.collection_id = None;
                    requests
                        .put(&to_js(&r)?, None)
                        .map_err(idb_err)?
                        .await
                        .map_err(idb_err)?;
                }
            }
        }
        for cid in to_delete {
            collections
                .delete(idb::Query::from(JsValue::from_f64(cid as f64)))
                .map_err(idb_err)?
                .await
                .map_err(idb_err)?;
        }
        commit(tx).await
    }

    async fn list_connections(&self) -> CoreResult<Vec<Connection>> {
        let tx = self.ro(&[CONNECTIONS])?;
        let store = tx.object_store(CONNECTIONS).map_err(idb_err)?;
        let all = store
            .get_all(None, None)
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        let mut out = Vec::with_capacity(all.len());
        for entry in all {
            out.push(from_js(entry)?);
        }
        Ok(out)
    }

    async fn get_connection(&self, id: ConnectionId) -> CoreResult<Option<Connection>> {
        let tx = self.ro(&[CONNECTIONS])?;
        let store = tx.object_store(CONNECTIONS).map_err(idb_err)?;
        let value = store
            .get(JsValue::from_f64(id as f64))
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        match value {
            Some(v) => Ok(Some(from_js(v)?)),
            None => Ok(None),
        }
    }

    async fn create_connection(&self, draft: NewConnection) -> CoreResult<Connection> {
        let now = chrono::Utc::now();
        let candidate = Connection {
            id: 0,
            name: draft.name,
            config: draft.config,
            auto_connect: draft.auto_connect,
            color: draft.color,
            created_at: now,
            updated_at: now,
        };
        let tx = self.rw(&[CONNECTIONS])?;
        let store = tx.object_store(CONNECTIONS).map_err(idb_err)?;
        let js = to_js(&candidate)?;
        let _ = js_sys::Reflect::delete_property(
            js.unchecked_ref::<js_sys::Object>(),
            &JsValue::from_str("id"),
        );
        let key = store
            .add(&js, None)
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        let id = key
            .as_f64()
            .ok_or_else(|| CoreError::Storage("non-numeric key".into()))? as i64;
        commit(tx).await?;
        Ok(Connection { id, ..candidate })
    }

    async fn update_connection(&self, connection: &Connection) -> CoreResult<()> {
        let mut updated = connection.clone();
        updated.updated_at = chrono::Utc::now();
        let tx = self.rw(&[CONNECTIONS])?;
        let store = tx.object_store(CONNECTIONS).map_err(idb_err)?;
        store
            .put(&to_js(&updated)?, None)
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        commit(tx).await
    }

    async fn delete_connection(&self, id: ConnectionId) -> CoreResult<()> {
        let tx = self.rw(&[CONNECTIONS, REQUESTS])?;
        let connections = tx.object_store(CONNECTIONS).map_err(idb_err)?;
        let requests = tx.object_store(REQUESTS).map_err(idb_err)?;
        let all_requests = requests
            .get_all(None, None)
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        for r_js in all_requests {
            let mut r: Request = from_js(r_js)?;
            if r.connection_id == Some(id) {
                r.connection_id = None;
                requests
                    .put(&to_js(&r)?, None)
                    .map_err(idb_err)?
                    .await
                    .map_err(idb_err)?;
            }
        }
        connections
            .delete(idb::Query::from(JsValue::from_f64(id as f64)))
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        commit(tx).await
    }

    async fn put_schema(&self, definition: &SchemaDefinition) -> CoreResult<()> {
        let tx = self.rw(&[SCHEMAS])?;
        let store = tx.object_store(SCHEMAS).map_err(idb_err)?;
        store
            .put(&to_js(definition)?, None)
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        commit(tx).await
    }

    async fn get_schema(&self, hash: &str) -> CoreResult<Option<SchemaDefinition>> {
        let tx = self.ro(&[SCHEMAS])?;
        let store = tx.object_store(SCHEMAS).map_err(idb_err)?;
        let value = store
            .get(JsValue::from_str(hash))
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        match value {
            Some(v) => Ok(Some(from_js(v)?)),
            None => Ok(None),
        }
    }

    async fn list_schemas(&self) -> CoreResult<Vec<SchemaDefinition>> {
        let tx = self.ro(&[SCHEMAS])?;
        let store = tx.object_store(SCHEMAS).map_err(idb_err)?;
        let all = store
            .get_all(None, None)
            .map_err(idb_err)?
            .await
            .map_err(idb_err)?;
        let mut out = Vec::with_capacity(all.len());
        for entry in all {
            out.push(from_js(entry)?);
        }
        Ok(out)
    }

    async fn clear_all(&self) -> CoreResult<()> {
        let tx = self.rw(&[REQUESTS, COLLECTIONS, CONNECTIONS, SCHEMAS])?;
        for store_name in [REQUESTS, COLLECTIONS, CONNECTIONS, SCHEMAS] {
            tx.object_store(store_name)
                .map_err(idb_err)?
                .clear()
                .map_err(idb_err)?
                .await
                .map_err(idb_err)?;
        }
        commit(tx).await
    }
}

use wasm_bindgen::JsCast as _;
