use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection as SqliteConnection, OptionalExtension, Row};

use crate::domain::{
    Collection, Connection, Request, RequestKind, SchemaRef, TransportConfig, TransportKind, Value,
};
use crate::ids::{CollectionId, ConnectionId, RequestId};
use crate::schema::SchemaDefinition;
use crate::storage::{NewCollection, NewConnection, NewRequest, Storage};
use crate::util::Clock;
use crate::{CoreError, CoreResult};

mod migrations;

pub struct SqliteStorage {
    conn: Arc<Mutex<SqliteConnection>>,
    clock: Arc<dyn Clock>,
}

impl SqliteStorage {
    pub fn open(path: &Path, clock: Arc<dyn Clock>) -> CoreResult<Self> {
        let mut conn = SqliteConnection::open(path).map_err(map_sqlite)?;
        bootstrap(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            clock,
        })
    }

    pub fn open_in_memory(clock: Arc<dyn Clock>) -> CoreResult<Self> {
        let mut conn = SqliteConnection::open_in_memory().map_err(map_sqlite)?;
        bootstrap(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            clock,
        })
    }

    async fn run<F, T>(&self, work: F) -> CoreResult<T>
    where
        F: FnOnce(&mut SqliteConnection) -> CoreResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let mut guard = conn.lock().expect("sqlite connection mutex poisoned");
            work(&mut guard)
        })
        .await
        .map_err(|join_error| CoreError::Storage(format!("storage task panicked: {join_error}")))?
    }

    fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }
}

fn bootstrap(conn: &mut SqliteConnection) -> CoreResult<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(map_sqlite)?;
    migrations::run(conn)
}

fn map_sqlite(err: rusqlite::Error) -> CoreError {
    if let rusqlite::Error::SqliteFailure(ref code, _) = err {
        if code.code == rusqlite::ErrorCode::ConstraintViolation {
            return CoreError::Conflict(err.to_string());
        }
    }
    CoreError::Storage(err.to_string())
}

fn to_rfc3339(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}

fn from_rfc3339(text: &str) -> CoreResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| CoreError::Storage(format!("invalid timestamp '{text}': {err}")))
}

fn row_to_collection(row: &Row<'_>) -> rusqlite::Result<Collection> {
    let created_at: String = row.get("created_at")?;
    Ok(Collection {
        id: row.get("id")?,
        parent_id: row.get("parent_id")?,
        name: row.get("name")?,
        created_at: from_rfc3339(&created_at).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
        })?,
    })
}

fn row_to_request(row: &Row<'_>) -> rusqlite::Result<Request> {
    let kind_text: String = row.get("kind")?;
    let kind = RequestKind::parse(&kind_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(CoreError::Storage(format!(
                "unknown request kind '{kind_text}'"
            ))),
        )
    })?;
    let schema_name: Option<String> = row.get("schema_name")?;
    let schema_hash: Option<String> = row.get("schema_hash")?;
    let schema = match (schema_name, schema_hash) {
        (Some(name), Some(hash)) => Some(SchemaRef { name, hash }),
        _ => None,
    };
    let input_json: String = row.get("input_json")?;
    let input: Value = serde_json::from_str(&input_json).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })?;
    let visualization_json: Option<String> = row.get("visualization_json")?;
    let visualization = match visualization_json {
        Some(text) => serde_json::from_str(&text).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
        })?,
        None => None,
    };
    let created_at: String = row.get("created_at")?;
    let updated_at: String = row.get("updated_at")?;
    Ok(Request {
        id: row.get("id")?,
        collection_id: row.get("collection_id")?,
        connection_id: row.get("connection_id")?,
        name: row.get("name")?,
        kind,
        target: row.get("target")?,
        schema,
        input,
        visualization,
        created_at: from_rfc3339(&created_at).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
        })?,
        updated_at: from_rfc3339(&updated_at).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
        })?,
    })
}

fn row_to_connection(row: &Row<'_>) -> rusqlite::Result<Connection> {
    let kind_text: String = row.get("transport_kind")?;
    let _kind = TransportKind::parse(&kind_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(CoreError::Storage(format!(
                "unknown transport kind '{kind_text}'"
            ))),
        )
    })?;
    let config_json: String = row.get("config_json")?;
    let config: TransportConfig = serde_json::from_str(&config_json).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })?;
    let auto_connect_int: i64 = row.get("auto_connect")?;
    let created_at: String = row.get("created_at")?;
    let updated_at: String = row.get("updated_at")?;
    Ok(Connection {
        id: row.get("id")?,
        name: row.get("name")?,
        config,
        auto_connect: auto_connect_int != 0,
        color: row.get("color")?,
        created_at: from_rfc3339(&created_at).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
        })?,
        updated_at: from_rfc3339(&updated_at).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
        })?,
    })
}

fn row_to_schema(row: &Row<'_>) -> rusqlite::Result<SchemaDefinition> {
    let parsed_json: String = row.get("parsed_json")?;
    let extras: ParsedSchemaPayload = serde_json::from_str(&parsed_json).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })?;
    Ok(SchemaDefinition {
        hash: row.get("hash")?,
        name: row.get("name")?,
        definition: row.get("definition")?,
        kind: extras.kind,
        parsed: extras.parsed,
        dependencies: extras.dependencies,
    })
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ParsedSchemaPayload {
    kind: crate::schema::SchemaKind,
    parsed: crate::schema::ParsedSchema,
    #[serde(default)]
    dependencies: Vec<String>,
}

#[async_trait::async_trait]
impl Storage for SqliteStorage {
    async fn list_requests(&self) -> CoreResult<Vec<Request>> {
        self.run(|conn| {
            let mut stmt = conn
                .prepare("SELECT * FROM requests ORDER BY created_at, id")
                .map_err(map_sqlite)?;
            let mapped = stmt.query_map([], row_to_request).map_err(map_sqlite)?;
            let collected: rusqlite::Result<Vec<_>> = mapped.collect();
            collected.map_err(map_sqlite)
        })
        .await
    }

    async fn list_requests_by_connection(
        &self,
        connection_id: ConnectionId,
    ) -> CoreResult<Vec<Request>> {
        self.run(move |conn| {
            let mut stmt = conn
                .prepare("SELECT * FROM requests WHERE connection_id = ?1 ORDER BY created_at, id")
                .map_err(map_sqlite)?;
            let mapped = stmt
                .query_map(params![connection_id], row_to_request)
                .map_err(map_sqlite)?;
            let collected: rusqlite::Result<Vec<_>> = mapped.collect();
            collected.map_err(map_sqlite)
        })
        .await
    }

    async fn get_request(&self, id: RequestId) -> CoreResult<Option<Request>> {
        self.run(move |conn| {
            conn.query_row(
                "SELECT * FROM requests WHERE id = ?1",
                params![id],
                row_to_request,
            )
            .optional()
            .map_err(map_sqlite)
        })
        .await
    }

    async fn create_request(&self, draft: NewRequest) -> CoreResult<Request> {
        let now = self.now();
        self.run(move |conn| {
            let input_json = serde_json::to_string(&draft.input)?;
            let (schema_name, schema_hash) = match draft.schema.as_ref() {
                Some(reference) => (Some(reference.name.clone()), Some(reference.hash.clone())),
                None => (None, None),
            };
            let visualization_json = match draft.visualization.as_ref() {
                Some(viz) => Some(serde_json::to_string(viz)?),
                None => None,
            };
            let now_text = to_rfc3339(now);
            conn.execute(
                "INSERT INTO requests (collection_id, connection_id, name, kind, target, schema_name, schema_hash, input_json, visualization_json, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                params![
                    draft.collection_id,
                    draft.connection_id,
                    draft.name,
                    draft.kind.as_str(),
                    draft.target,
                    schema_name,
                    schema_hash,
                    input_json,
                    visualization_json,
                    now_text,
                ],
            ).map_err(map_sqlite)?;
            let id = conn.last_insert_rowid();
            conn.query_row("SELECT * FROM requests WHERE id = ?1", params![id], row_to_request)
                .map_err(map_sqlite)
        })
        .await
    }

    async fn update_request(&self, request: &Request) -> CoreResult<()> {
        let payload = request.clone();
        let now = self.now();
        self.run(move |conn| {
            let input_json = serde_json::to_string(&payload.input)?;
            let (schema_name, schema_hash) = match payload.schema.as_ref() {
                Some(reference) => (Some(reference.name.clone()), Some(reference.hash.clone())),
                None => (None, None),
            };
            let visualization_json = match payload.visualization.as_ref() {
                Some(viz) => Some(serde_json::to_string(viz)?),
                None => None,
            };
            let updated = conn.execute(
                "UPDATE requests SET collection_id = ?2, connection_id = ?3, name = ?4, kind = ?5, target = ?6, schema_name = ?7, schema_hash = ?8, input_json = ?9, visualization_json = ?10, updated_at = ?11 WHERE id = ?1",
                params![
                    payload.id,
                    payload.collection_id,
                    payload.connection_id,
                    payload.name,
                    payload.kind.as_str(),
                    payload.target,
                    schema_name,
                    schema_hash,
                    input_json,
                    visualization_json,
                    to_rfc3339(now),
                ],
            ).map_err(map_sqlite)?;
            if updated == 0 {
                return Err(CoreError::NotFound(format!("request {}", payload.id)));
            }
            Ok(())
        })
        .await
    }

    async fn delete_request(&self, id: RequestId) -> CoreResult<()> {
        self.run(move |conn| {
            let removed = conn
                .execute("DELETE FROM requests WHERE id = ?1", params![id])
                .map_err(map_sqlite)?;
            if removed == 0 {
                return Err(CoreError::NotFound(format!("request {id}")));
            }
            Ok(())
        })
        .await
    }

    async fn list_collections(&self) -> CoreResult<Vec<Collection>> {
        self.run(|conn| {
            let mut stmt = conn
                .prepare("SELECT * FROM collections ORDER BY created_at, id")
                .map_err(map_sqlite)?;
            let mapped = stmt.query_map([], row_to_collection).map_err(map_sqlite)?;
            let collected: rusqlite::Result<Vec<_>> = mapped.collect();
            collected.map_err(map_sqlite)
        })
        .await
    }

    async fn create_collection(&self, draft: NewCollection) -> CoreResult<Collection> {
        let now = self.now();
        self.run(move |conn| {
            conn.execute(
                "INSERT INTO collections (parent_id, name, created_at) VALUES (?1, ?2, ?3)",
                params![draft.parent_id, draft.name, to_rfc3339(now)],
            )
            .map_err(map_sqlite)?;
            let id = conn.last_insert_rowid();
            conn.query_row(
                "SELECT * FROM collections WHERE id = ?1",
                params![id],
                row_to_collection,
            )
            .map_err(map_sqlite)
        })
        .await
    }

    async fn update_collection(&self, collection: &Collection) -> CoreResult<()> {
        let payload = collection.clone();
        self.run(move |conn| {
            let updated = conn
                .execute(
                    "UPDATE collections SET parent_id = ?2, name = ?3 WHERE id = ?1",
                    params![payload.id, payload.parent_id, payload.name],
                )
                .map_err(map_sqlite)?;
            if updated == 0 {
                return Err(CoreError::NotFound(format!("collection {}", payload.id)));
            }
            Ok(())
        })
        .await
    }

    async fn delete_collection(&self, id: CollectionId) -> CoreResult<()> {
        self.run(move |conn| {
            let removed = conn
                .execute("DELETE FROM collections WHERE id = ?1", params![id])
                .map_err(map_sqlite)?;
            if removed == 0 {
                return Err(CoreError::NotFound(format!("collection {id}")));
            }
            Ok(())
        })
        .await
    }

    async fn list_connections(&self) -> CoreResult<Vec<Connection>> {
        self.run(|conn| {
            let mut stmt = conn
                .prepare("SELECT * FROM connections ORDER BY created_at, id")
                .map_err(map_sqlite)?;
            let mapped = stmt.query_map([], row_to_connection).map_err(map_sqlite)?;
            let collected: rusqlite::Result<Vec<_>> = mapped.collect();
            collected.map_err(map_sqlite)
        })
        .await
    }

    async fn get_connection(&self, id: ConnectionId) -> CoreResult<Option<Connection>> {
        self.run(move |conn| {
            conn.query_row(
                "SELECT * FROM connections WHERE id = ?1",
                params![id],
                row_to_connection,
            )
            .optional()
            .map_err(map_sqlite)
        })
        .await
    }

    async fn create_connection(&self, draft: NewConnection) -> CoreResult<Connection> {
        let now = self.now();
        self.run(move |conn| {
            let kind = draft.config.kind();
            let config_json = serde_json::to_string(&draft.config)?;
            let now_text = to_rfc3339(now);
            conn.execute(
                "INSERT INTO connections (name, transport_kind, config_json, auto_connect, color, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![
                    draft.name,
                    kind.as_str(),
                    config_json,
                    if draft.auto_connect { 1_i64 } else { 0_i64 },
                    draft.color,
                    now_text,
                ],
            ).map_err(map_sqlite)?;
            let id = conn.last_insert_rowid();
            conn.query_row(
                "SELECT * FROM connections WHERE id = ?1",
                params![id],
                row_to_connection,
            )
            .map_err(map_sqlite)
        })
        .await
    }

    async fn update_connection(&self, connection: &Connection) -> CoreResult<()> {
        let payload = connection.clone();
        let now = self.now();
        self.run(move |conn| {
            let kind = payload.config.kind();
            let config_json = serde_json::to_string(&payload.config)?;
            let updated = conn.execute(
                "UPDATE connections SET name = ?2, transport_kind = ?3, config_json = ?4, auto_connect = ?5, color = ?6, updated_at = ?7 WHERE id = ?1",
                params![
                    payload.id,
                    payload.name,
                    kind.as_str(),
                    config_json,
                    if payload.auto_connect { 1_i64 } else { 0_i64 },
                    payload.color,
                    to_rfc3339(now),
                ],
            ).map_err(map_sqlite)?;
            if updated == 0 {
                return Err(CoreError::NotFound(format!("connection {}", payload.id)));
            }
            Ok(())
        })
        .await
    }

    async fn delete_connection(&self, id: ConnectionId) -> CoreResult<()> {
        self.run(move |conn| {
            let tx = conn.transaction().map_err(map_sqlite)?;
            tx.execute(
                "UPDATE requests SET connection_id = NULL WHERE connection_id = ?1",
                params![id],
            )
            .map_err(map_sqlite)?;
            let removed = tx
                .execute("DELETE FROM connections WHERE id = ?1", params![id])
                .map_err(map_sqlite)?;
            if removed == 0 {
                return Err(CoreError::NotFound(format!("connection {id}")));
            }
            tx.commit().map_err(map_sqlite)?;
            Ok(())
        })
        .await
    }

    async fn put_schema(&self, definition: &SchemaDefinition) -> CoreResult<()> {
        let payload = definition.clone();
        let now = self.now();
        self.run(move |conn| {
            let parsed_json = serde_json::to_string(&ParsedSchemaPayload {
                kind: payload.kind,
                parsed: payload.parsed,
                dependencies: payload.dependencies,
            })?;
            conn.execute(
                "INSERT INTO schemas (hash, name, definition, parsed_json, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT(hash) DO UPDATE SET name = excluded.name, definition = excluded.definition, parsed_json = excluded.parsed_json",
                params![
                    payload.hash,
                    payload.name,
                    payload.definition,
                    parsed_json,
                    to_rfc3339(now),
                ],
            )
            .map_err(map_sqlite)?;
            Ok(())
        })
        .await
    }

    async fn get_schema(&self, hash: &str) -> CoreResult<Option<SchemaDefinition>> {
        let hash = hash.to_string();
        self.run(move |conn| {
            conn.query_row(
                "SELECT hash, name, definition, parsed_json FROM schemas WHERE hash = ?1",
                params![hash],
                row_to_schema,
            )
            .optional()
            .map_err(map_sqlite)
        })
        .await
    }

    async fn list_schemas(&self) -> CoreResult<Vec<SchemaDefinition>> {
        self.run(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT hash, name, definition, parsed_json FROM schemas ORDER BY name, hash",
                )
                .map_err(map_sqlite)?;
            let mapped = stmt.query_map([], row_to_schema).map_err(map_sqlite)?;
            let collected: rusqlite::Result<Vec<_>> = mapped.collect();
            collected.map_err(map_sqlite)
        })
        .await
    }

    async fn clear_all(&self) -> CoreResult<()> {
        self.run(|conn| {
            let tx = conn.transaction().map_err(map_sqlite)?;
            tx.execute("DELETE FROM requests", []).map_err(map_sqlite)?;
            tx.execute("DELETE FROM collections", [])
                .map_err(map_sqlite)?;
            tx.execute("DELETE FROM connections", [])
                .map_err(map_sqlite)?;
            tx.execute("DELETE FROM schemas", []).map_err(map_sqlite)?;
            tx.commit().map_err(map_sqlite)?;
            Ok(())
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Value;
    use crate::util::MockClock;
    use chrono::TimeZone;

    fn fixed_clock(year: i32, month: u32, day: u32) -> Arc<MockClock> {
        Arc::new(MockClock::new(
            Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap(),
        ))
    }

    fn make_storage(clock: Arc<MockClock>) -> SqliteStorage {
        SqliteStorage::open_in_memory(clock).expect("open in-memory storage")
    }

    fn ws_config(url: &str) -> TransportConfig {
        TransportConfig::FoxgloveWs {
            url: url.into(),
            headers: vec![],
        }
    }

    #[tokio::test]
    async fn requests_round_trip() {
        let clock = fixed_clock(2026, 1, 1);
        let storage = make_storage(clock);

        let request = storage
            .create_request(NewRequest {
                name: "Scan".into(),
                kind: RequestKind::Topic,
                target: "/scan".into(),
                collection_id: None,
                connection_id: None,
                schema: Some(SchemaRef {
                    name: "sensor_msgs/LaserScan".into(),
                    hash: "h".into(),
                }),
                input: Value::empty_struct(),
                visualization: None,
            })
            .await
            .expect("create");

        let listed = storage.list_requests().await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0], request);

        let fetched = storage.get_request(request.id).await.expect("get");
        assert_eq!(fetched, Some(request));
    }

    #[tokio::test]
    async fn request_visualization_round_trips() {
        let clock = fixed_clock(2026, 1, 1);
        let storage = make_storage(clock);

        let mut request = storage
            .create_request(NewRequest {
                name: "Camera".into(),
                kind: RequestKind::Topic,
                target: "/image".into(),
                collection_id: None,
                connection_id: None,
                schema: None,
                input: Value::empty_struct(),
                visualization: None,
            })
            .await
            .expect("create");
        assert_eq!(request.visualization, None);

        request.visualization = Some(serde_json::json!({
            "tab": "visualize",
            "visualizerId": "rw.viz.image",
            "configs": { "rw.viz.image": {} }
        }));
        storage.update_request(&request).await.expect("update");

        let fetched = storage
            .get_request(request.id)
            .await
            .expect("get")
            .expect("present");
        assert_eq!(fetched.visualization, request.visualization);
    }

    #[tokio::test]
    async fn update_request_bumps_updated_at() {
        let clock = fixed_clock(2026, 1, 1);
        let storage = make_storage(clock.clone());
        let mut request = storage
            .create_request(NewRequest {
                name: "Scan".into(),
                kind: RequestKind::Topic,
                target: "/scan".into(),
                collection_id: None,
                connection_id: None,
                schema: None,
                input: Value::empty_struct(),
                visualization: None,
            })
            .await
            .unwrap();

        clock.advance(chrono::Duration::seconds(10));
        request.name = "Renamed".into();
        storage.update_request(&request).await.expect("update");

        let after = storage.get_request(request.id).await.unwrap().unwrap();
        assert_eq!(after.name, "Renamed");
        assert!(after.updated_at > request.created_at);
    }

    #[tokio::test]
    async fn delete_connection_nulls_request_link() {
        let clock = fixed_clock(2026, 1, 1);
        let storage = make_storage(clock);

        let connection = storage
            .create_connection(NewConnection {
                name: "Lab".into(),
                config: ws_config("ws://localhost:8765"),
                auto_connect: false,
                color: None,
            })
            .await
            .unwrap();

        let request = storage
            .create_request(NewRequest {
                name: "Bound".into(),
                kind: RequestKind::Topic,
                target: "/topic".into(),
                collection_id: None,
                connection_id: Some(connection.id),
                schema: None,
                input: Value::empty_struct(),
                visualization: None,
            })
            .await
            .unwrap();

        storage
            .delete_connection(connection.id)
            .await
            .expect("delete connection");

        let after = storage.get_request(request.id).await.unwrap().unwrap();
        assert_eq!(after.connection_id, None);
        assert!(storage
            .get_connection(connection.id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn delete_collection_nulls_request_link_and_cascades_children() {
        let clock = fixed_clock(2026, 1, 1);
        let storage = make_storage(clock);

        let parent = storage
            .create_collection(NewCollection {
                name: "Parent".into(),
                parent_id: None,
            })
            .await
            .unwrap();
        let child = storage
            .create_collection(NewCollection {
                name: "Child".into(),
                parent_id: Some(parent.id),
            })
            .await
            .unwrap();
        let request = storage
            .create_request(NewRequest {
                name: "Inside".into(),
                kind: RequestKind::Topic,
                target: "/x".into(),
                collection_id: Some(child.id),
                connection_id: None,
                schema: None,
                input: Value::empty_struct(),
                visualization: None,
            })
            .await
            .unwrap();

        storage
            .delete_collection(parent.id)
            .await
            .expect("delete parent");

        let collections = storage.list_collections().await.unwrap();
        assert!(collections.is_empty());
        let after = storage.get_request(request.id).await.unwrap().unwrap();
        assert_eq!(after.collection_id, None);
    }

    #[tokio::test]
    async fn unique_connection_name_raises_conflict() {
        let clock = fixed_clock(2026, 1, 1);
        let storage = make_storage(clock);

        storage
            .create_connection(NewConnection {
                name: "Lab".into(),
                config: ws_config("ws://a"),
                auto_connect: false,
                color: None,
            })
            .await
            .unwrap();

        let result = storage
            .create_connection(NewConnection {
                name: "Lab".into(),
                config: ws_config("ws://b"),
                auto_connect: false,
                color: None,
            })
            .await;
        assert!(
            matches!(result, Err(CoreError::Conflict(_))),
            "expected Conflict, got {result:?}"
        );
    }

    #[tokio::test]
    async fn schema_round_trips_and_idempotent_put() {
        let clock = fixed_clock(2026, 1, 1);
        let storage = make_storage(clock);

        let definition = SchemaDefinition {
            hash: "h1".into(),
            name: "std_msgs/Header".into(),
            definition: "uint32 seq\n".into(),
            kind: crate::schema::SchemaKind::Message,
            parsed: crate::schema::ParsedSchema::Message(crate::schema::MessageDef::default()),
            dependencies: vec![],
        };
        storage.put_schema(&definition).await.unwrap();
        storage.put_schema(&definition).await.unwrap();

        let listed = storage.list_schemas().await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0], definition);

        let fetched = storage.get_schema("h1").await.unwrap();
        assert_eq!(fetched, Some(definition));
        assert!(storage.get_schema("missing").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn migration_replay_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ws.db");

        {
            let storage = SqliteStorage::open(&path, fixed_clock(2026, 1, 1)).unwrap();
            storage
                .create_collection(NewCollection {
                    name: "Persisted".into(),
                    parent_id: None,
                })
                .await
                .unwrap();
        }

        let storage = SqliteStorage::open(&path, fixed_clock(2026, 1, 1)).unwrap();
        let collections = storage.list_collections().await.unwrap();
        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0].name, "Persisted");
    }

    #[tokio::test]
    async fn clear_all_empties_every_table_and_preserves_migrations() {
        let clock = fixed_clock(2026, 1, 1);
        let storage = make_storage(clock);

        let coll = storage
            .create_collection(NewCollection {
                name: "C".into(),
                parent_id: None,
            })
            .await
            .unwrap();
        let conn = storage
            .create_connection(NewConnection {
                name: "Conn".into(),
                config: ws_config("ws://localhost:9091"),
                auto_connect: false,
                color: None,
            })
            .await
            .unwrap();
        storage
            .create_request(NewRequest {
                name: "R".into(),
                kind: crate::domain::RequestKind::Topic,
                target: "/scan".into(),
                schema: None,
                input: Value::Null,
                visualization: None,
                collection_id: Some(coll.id),
                connection_id: Some(conn.id),
            })
            .await
            .unwrap();
        let schema = SchemaDefinition {
            hash: "abc".into(),
            name: "test/Msg".into(),
            kind: crate::schema::SchemaKind::Message,
            definition: "uint8 x".into(),
            parsed: crate::schema::ParsedSchema::Message(crate::schema::MessageDef {
                fields: vec![],
                constants: vec![],
            }),
            dependencies: vec![],
        };
        storage.put_schema(&schema).await.unwrap();

        assert_eq!(storage.list_requests().await.unwrap().len(), 1);
        assert_eq!(storage.list_collections().await.unwrap().len(), 1);
        assert_eq!(storage.list_connections().await.unwrap().len(), 1);
        assert_eq!(storage.list_schemas().await.unwrap().len(), 1);

        storage.clear_all().await.unwrap();

        assert_eq!(storage.list_requests().await.unwrap().len(), 0);
        assert_eq!(storage.list_collections().await.unwrap().len(), 0);
        assert_eq!(storage.list_connections().await.unwrap().len(), 0);
        assert_eq!(storage.list_schemas().await.unwrap().len(), 0);

        let conn_guard = storage.conn.lock().unwrap();
        let mig_count: i64 = conn_guard
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("count migrations");
        assert!(
            mig_count > 0,
            "schema_migrations row must persist across clear_all"
        );
    }
}
