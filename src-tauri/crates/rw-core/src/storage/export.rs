use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportMode {
    Replace,
    Merge,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ImportConflict {
    pub entity: String,
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub connections_added: u32,
    pub connections_skipped: u32,
    pub collections_added: u32,
    pub requests_added: u32,
    pub schemas_added: u32,
    pub conflicts: Vec<ImportConflict>,
}

use crate::domain::{Workspace, WorkspaceFile};
use crate::storage::Storage;
use crate::util::Clock;
use crate::{CoreError, CoreResult};
use std::sync::Arc;

pub async fn export_workspace(
    storage: &dyn Storage,
    clock: Arc<dyn Clock>,
) -> CoreResult<WorkspaceFile> {
    let connections = storage.list_connections().await?;
    let collections = storage.list_collections().await?;
    let requests = storage.list_requests().await?;
    let schemas = storage.list_schemas().await?;

    Ok(WorkspaceFile::new(
        Workspace {
            connections,
            collections,
            requests,
            schemas,
        },
        clock.now(),
    ))
}

pub async fn import_workspace(
    storage: &dyn Storage,
    file: WorkspaceFile,
    mode: ImportMode,
) -> CoreResult<ImportReport> {
    if file.format != crate::domain::WORKSPACE_FORMAT {
        return Err(CoreError::InvalidArgument(format!(
            "unrecognized workspace format '{}'",
            file.format
        )));
    }
    if file.version != crate::domain::WORKSPACE_VERSION {
        return Err(CoreError::InvalidArgument(format!(
            "unsupported workspace version {}",
            file.version
        )));
    }

    match mode {
        ImportMode::Replace => import_replace(storage, file.workspace).await,
        ImportMode::Merge => import_merge(storage, file.workspace).await,
    }
}

async fn import_replace(storage: &dyn Storage, workspace: Workspace) -> CoreResult<ImportReport> {
    for request in storage.list_requests().await? {
        storage.delete_request(request.id).await?;
    }
    for collection in storage.list_collections().await? {
        let _ = storage.delete_collection(collection.id).await;
    }
    for connection in storage.list_connections().await? {
        storage.delete_connection(connection.id).await?;
    }

    insert_workspace(storage, workspace, false).await
}

async fn import_merge(storage: &dyn Storage, workspace: Workspace) -> CoreResult<ImportReport> {
    insert_workspace(storage, workspace, true).await
}

async fn insert_workspace(
    storage: &dyn Storage,
    workspace: Workspace,
    report_conflicts: bool,
) -> CoreResult<ImportReport> {
    let mut report = ImportReport::default();

    let mut connection_id_remap: std::collections::HashMap<i64, i64> = Default::default();
    let existing_connections = storage.list_connections().await?;
    for incoming in workspace.connections {
        if let Some(existing) = existing_connections
            .iter()
            .find(|candidate| candidate.name == incoming.name)
        {
            connection_id_remap.insert(incoming.id, existing.id);
            if report_conflicts && existing.config != incoming.config {
                report.conflicts.push(ImportConflict {
                    entity: "connection".into(),
                    name: incoming.name.clone(),
                    reason: "existing connection has different transport config".into(),
                });
            }
            report.connections_skipped += 1;
            continue;
        }
        let created = storage
            .create_connection(crate::storage::NewConnection {
                name: incoming.name,
                config: incoming.config,
                auto_connect: incoming.auto_connect,
                color: incoming.color,
            })
            .await?;
        connection_id_remap.insert(incoming.id, created.id);
        report.connections_added += 1;
    }

    let mut collection_id_remap: std::collections::HashMap<i64, i64> = Default::default();
    let mut sorted_collections = workspace.collections;
    sorted_collections.sort_by_key(|collection| collection.parent_id.unwrap_or(0));
    for incoming in sorted_collections {
        let parent = match incoming.parent_id {
            Some(original) => collection_id_remap.get(&original).copied(),
            None => None,
        };
        let created = storage
            .create_collection(crate::storage::NewCollection {
                name: incoming.name,
                parent_id: parent,
            })
            .await?;
        collection_id_remap.insert(incoming.id, created.id);
        report.collections_added += 1;
    }

    for incoming in workspace.requests {
        let collection_id = incoming
            .collection_id
            .and_then(|original| collection_id_remap.get(&original).copied());
        let connection_id = incoming
            .connection_id
            .and_then(|original| connection_id_remap.get(&original).copied());
        storage
            .create_request(crate::storage::NewRequest {
                name: incoming.name,
                kind: incoming.kind,
                target: incoming.target,
                collection_id,
                connection_id,
                schema: incoming.schema,
                input: incoming.input,
                visualization: incoming.visualization,
            })
            .await?;
        report.requests_added += 1;
    }

    for definition in workspace.schemas {
        let already_present = storage.get_schema(&definition.hash).await?.is_some();
        storage.put_schema(&definition).await?;
        if !already_present {
            report.schemas_added += 1;
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        Collection, Connection, Request, RequestKind, SchemaRef, TransportConfig, Value,
    };
    use crate::schema::SchemaDefinition;
    use chrono::TimeZone;

    fn fixed_time() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    }

    fn sample_workspace() -> Workspace {
        Workspace {
            connections: vec![Connection {
                id: 1,
                name: "Lab".into(),
                config: TransportConfig::FoxgloveWs {
                    url: "ws://localhost:8765".into(),
                    headers: vec![],
                },
                auto_connect: true,
                color: Some("#00aaff".into()),
                created_at: fixed_time(),
                updated_at: fixed_time(),
            }],
            collections: vec![Collection {
                id: 1,
                parent_id: None,
                name: "Demo".into(),
                created_at: fixed_time(),
            }],
            requests: vec![Request {
                id: 1,
                collection_id: Some(1),
                connection_id: Some(1),
                name: "Scan".into(),
                kind: RequestKind::Topic,
                target: "/scan".into(),
                schema: Some(SchemaRef {
                    name: "sensor_msgs/LaserScan".into(),
                    hash: "h1".into(),
                }),
                input: Value::empty_struct(),
                visualization: None,
                created_at: fixed_time(),
                updated_at: fixed_time(),
            }],
            schemas: vec![SchemaDefinition {
                hash: "h1".into(),
                name: "sensor_msgs/LaserScan".into(),
                definition: "Header header\nfloat32 angle_min".into(),
                kind: crate::schema::SchemaKind::Message,
                parsed: crate::schema::ParsedSchema::Message(crate::schema::MessageDef::default()),
                dependencies: vec![],
            }],
        }
    }

    #[test]
    fn workspace_file_serializes_envelope_fields_at_top_level() {
        let file = WorkspaceFile::new(sample_workspace(), fixed_time());
        let json = serde_json::to_value(&file).unwrap();
        let object = json.as_object().unwrap();
        assert_eq!(object.get("format").unwrap(), "robot-whisperer/workspace");
        assert_eq!(object.get("version").unwrap(), 1);
        assert!(object.contains_key("connections"));
        assert!(object.contains_key("requests"));
    }

    #[test]
    fn workspace_round_trips_through_json() {
        let file = WorkspaceFile::new(sample_workspace(), fixed_time());
        let json = serde_json::to_string(&file).unwrap();
        let decoded: WorkspaceFile = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, file);
    }

    use proptest::prelude::*;

    fn arb_value() -> impl Strategy<Value = Value> {
        prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            "[a-z]{0,8}".prop_map(Value::String),
        ]
    }

    fn arb_request(
        connection_ids: Vec<i64>,
        collection_ids: Vec<i64>,
    ) -> impl Strategy<Value = Request> {
        let connection_pool = if connection_ids.is_empty() {
            vec![None]
        } else {
            let mut pool = vec![None];
            pool.extend(connection_ids.into_iter().map(Some));
            pool
        };
        let collection_pool = if collection_ids.is_empty() {
            vec![None]
        } else {
            let mut pool = vec![None];
            pool.extend(collection_ids.into_iter().map(Some));
            pool
        };

        (
            1i64..1000,
            "[a-z]{1,8}",
            prop_oneof![
                Just(RequestKind::Topic),
                Just(RequestKind::Service),
                Just(RequestKind::Action)
            ],
            "/[a-z]{1,8}",
            arb_value(),
            prop::sample::select(connection_pool),
            prop::sample::select(collection_pool),
        )
            .prop_map(
                |(id, name, kind, target, input, connection_id, collection_id)| Request {
                    id,
                    collection_id,
                    connection_id,
                    name,
                    kind,
                    target,
                    schema: None,
                    input,
                    visualization: None,
                    created_at: fixed_time(),
                    updated_at: fixed_time(),
                },
            )
    }

    fn arb_workspace() -> impl Strategy<Value = Workspace> {
        let connections = prop::collection::vec(
            (1i64..100, "[a-z]{1,8}").prop_map(|(id, name)| Connection {
                id,
                name,
                config: TransportConfig::FoxgloveWs {
                    url: "ws://x".into(),
                    headers: vec![],
                },
                auto_connect: false,
                color: None,
                created_at: fixed_time(),
                updated_at: fixed_time(),
            }),
            0..3,
        );
        let collections = prop::collection::vec(
            (1i64..100, "[a-z]{1,8}").prop_map(|(id, name)| Collection {
                id,
                parent_id: None,
                name,
                created_at: fixed_time(),
            }),
            0..3,
        );

        (connections, collections).prop_flat_map(|(connections, collections)| {
            let connection_ids: Vec<i64> = connections.iter().map(|c| c.id).collect();
            let collection_ids: Vec<i64> = collections.iter().map(|c| c.id).collect();
            (
                Just(connections),
                Just(collections),
                prop::collection::vec(arb_request(connection_ids, collection_ids), 0..3),
            )
                .prop_map(|(connections, collections, requests)| Workspace {
                    connections,
                    collections,
                    requests,
                    schemas: vec![],
                })
        })
    }

    proptest! {
        #[test]
        fn workspace_file_round_trip_preserves_structure(workspace in arb_workspace()) {
            let file = WorkspaceFile::new(workspace, fixed_time());
            let json = serde_json::to_string(&file).unwrap();
            let decoded: WorkspaceFile = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(decoded, file);
        }
    }
}
