#![cfg(target_family = "wasm")]

use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;

use rw_core::schema::{SchemaKind, SchemaRegistry};
use rw_core::storage::{IdbStorage, NewCollection, NewConnection, NewRequest, Storage};
use rw_core::visualization::image::{image_value_to_rgba, ImageRgba};
use rw_pipeline::CanonicalPipeline;
use rw_transport::ConnectionId;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Default, Deserialize)]
struct SubscribeOptionsJs {
    #[serde(default)]
    target_hz: Option<f32>,
    #[serde(default)]
    queue_length: Option<u32>,
}

#[wasm_bindgen(start)]
pub fn __init() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub struct WasmRobotWhisperer {
    pipeline: CanonicalPipeline,
    storage: std::sync::Arc<IdbStorage>,
    schemas: std::sync::Arc<SchemaRegistry>,
}

#[wasm_bindgen]
impl WasmRobotWhisperer {
    #[allow(clippy::arc_with_non_send_sync)]
    pub async fn create() -> Result<WasmRobotWhisperer, JsError> {
        use std::sync::Arc;
        let storage: Arc<IdbStorage> = Arc::new(
            IdbStorage::open()
                .await
                .map_err(|err| JsError::new(&err.to_string()))?,
        );
        let storage_dyn: Arc<dyn Storage> = storage.clone();
        let registry: Arc<SchemaRegistry> = Arc::new(
            SchemaRegistry::new(storage_dyn)
                .await
                .map_err(|err| JsError::new(&err.to_string()))?,
        );
        let pipeline = CanonicalPipeline::with_schema_registry(registry.clone());
        Ok(WasmRobotWhisperer {
            pipeline,
            storage,
            schemas: registry,
        })
    }

    #[wasm_bindgen(js_name = "pipelineOpenFoxglove")]
    pub async fn pipeline_open_foxglove(&self, url: String) -> Result<String, JsError> {
        let id = self
            .pipeline
            .open_foxglove(url)
            .await
            .map_err(|err| JsError::new(&err.to_string()))?;
        Ok(id.to_string())
    }

    #[wasm_bindgen(js_name = "pipelineOpenRosbridge")]
    pub async fn pipeline_open_rosbridge(&self, url: String) -> Result<String, JsError> {
        let id = self
            .pipeline
            .open_rosbridge(url)
            .await
            .map_err(|err| JsError::new(&err.to_string()))?;
        Ok(id.to_string())
    }

    #[wasm_bindgen(js_name = "pipelineOpenDummy")]
    pub async fn pipeline_open_dummy(&self) -> Result<String, JsError> {
        let id = self
            .pipeline
            .open_dummy()
            .await
            .map_err(|err| JsError::new(&err.to_string()))?;
        Ok(id.to_string())
    }

    #[wasm_bindgen(js_name = "pipelineGetDiscovery")]
    pub async fn pipeline_get_discovery(&self, connection_id: String) -> Result<String, JsError> {
        let id = parse_connection_id(&connection_id)?;
        let transport = match self.pipeline.transport(id).await {
            Ok(t) => t,
            Err(_) => return Ok("null".into()),
        };
        let snapshot = transport.discovery().borrow().clone();
        let legacy = rw_transport::discovery_to_json(&snapshot);
        serde_json::to_string(&legacy).map_err(|err| JsError::new(&err.to_string()))
    }

    #[wasm_bindgen(js_name = "pipelineOnStatus")]
    pub async fn pipeline_on_status(
        &self,
        connection_id: String,
        on_status: js_sys::Function,
    ) -> Result<(), JsError> {
        let id = parse_connection_id(&connection_id)?;
        let transport = self
            .pipeline
            .transport(id)
            .await
            .map_err(|err| JsError::new(&err.to_string()))?;
        let mut rx = transport.status();
        let cb = Rc::new(RefCell::new(on_status));
        wasm_bindgen_futures::spawn_local(async move {
            emit_status(&cb, &rx.borrow().clone());
            while rx.changed().await.is_ok() {
                emit_status(&cb, &rx.borrow().clone());
            }
        });
        Ok(())
    }

    #[wasm_bindgen(js_name = "pipelineOnDiscovery")]
    pub async fn pipeline_on_discovery(
        &self,
        connection_id: String,
        on_discovery: js_sys::Function,
    ) -> Result<(), JsError> {
        let id = parse_connection_id(&connection_id)?;
        let transport = self
            .pipeline
            .transport(id)
            .await
            .map_err(|err| JsError::new(&err.to_string()))?;
        let mut rx = transport.discovery();
        let cb = Rc::new(RefCell::new(on_discovery));
        wasm_bindgen_futures::spawn_local(async move {
            emit_discovery(&cb, &rx.borrow().clone());
            while rx.changed().await.is_ok() {
                emit_discovery(&cb, &rx.borrow().clone());
            }
        });
        Ok(())
    }

    #[wasm_bindgen(js_name = "pipelineClose")]
    pub async fn pipeline_close(&self, connection_id: String) -> Result<(), JsError> {
        let id = parse_connection_id(&connection_id)?;
        self.pipeline
            .close(id)
            .await
            .map_err(|err| JsError::new(&err.to_string()))?;
        Ok(())
    }

    #[wasm_bindgen(js_name = "setPerfTraceEnabled")]
    pub fn set_perf_trace_enabled(&self, enabled: bool) {
        rw_transport::perf::set_perf_trace_enabled(enabled);
    }

    #[wasm_bindgen(js_name = "pipelineSubscribeTopic")]
    pub async fn pipeline_subscribe_topic(
        &self,
        connection_id: String,
        topic: String,
        on_frame: js_sys::Function,
        options_json: Option<String>,
    ) -> Result<JsValue, JsError> {
        let id = parse_connection_id(&connection_id)?;
        let options = match options_json.as_deref() {
            Some(json) if !json.is_empty() => {
                let parsed: SubscribeOptionsJs = serde_json::from_str(json)
                    .map_err(|err| JsError::new(&format!("subscribe options: {err}")))?;
                rw_transport::SubscribeOptions {
                    target_hz: parsed.target_hz,
                    queue_length: parsed.queue_length,
                }
            }
            _ => rw_transport::SubscribeOptions::default(),
        };
        let on_frame = Rc::new(RefCell::new(on_frame));
        let on_frame_clone = on_frame.clone();
        let result = self
            .pipeline
            .subscribe_topic_with_options(id, &topic, options, move |handle, frame, is_replay| {
                let cb = on_frame_clone.borrow();
                let payload = build_frame_js(handle, frame, is_replay);
                let this = JsValue::NULL;
                let _ = cb.call1(&this, &payload);
            })
            .await
            .map_err(|err| JsError::new(&err.to_string()))?;
        drop(on_frame);
        let response = SubscribeResponse {
            subscription_id: result.subscription_id,
            schema_id: result.schema_id,
            schema_name: result.schema_name,
            viz_role: result.viz_role,
        };
        serde_wasm_bindgen::to_value(&response).map_err(|err| JsError::new(&err.to_string()))
    }

    #[wasm_bindgen(js_name = "pipelineUnsubscribe")]
    pub async fn pipeline_unsubscribe(&self, subscription_id: String) -> Result<(), JsError> {
        self.pipeline
            .unsubscribe(&subscription_id)
            .await
            .map_err(|err| JsError::new(&err.to_string()))?;
        Ok(())
    }

    #[wasm_bindgen(js_name = "pipelineCallService")]
    pub async fn pipeline_call_service(
        &self,
        connection_id: String,
        service: String,
        request_json: String,
    ) -> Result<String, JsError> {
        let id = parse_connection_id(&connection_id)?;
        let request: rw_canonical::CanonicalValue = serde_json::from_str(&request_json)
            .map_err(|err| JsError::new(&format!("request_json: {err}")))?;
        let response = self
            .pipeline
            .call_service(id, &service, request)
            .await
            .map_err(|err| JsError::new(&err.to_string()))?;
        serde_json::to_string(&response)
            .map_err(|err| JsError::new(&format!("response_json: {err}")))
    }

    #[wasm_bindgen(js_name = "pipelineSendActionGoal")]
    pub async fn pipeline_send_action_goal(
        &self,
        connection_id: String,
        action: String,
        goal_json: String,
        on_envelope: js_sys::Function,
    ) -> Result<String, JsError> {
        let id = parse_connection_id(&connection_id)?;
        let goal: rw_canonical::CanonicalValue = serde_json::from_str(&goal_json)
            .map_err(|err| JsError::new(&format!("goal_json: {err}")))?;
        let stream = self
            .pipeline
            .send_action_goal(id, &action, goal)
            .await
            .map_err(|err| JsError::new(&err.to_string()))?;
        let goal_id = stream.cancel_token.goal_id.clone();

        let on_envelope = Rc::new(RefCell::new(on_envelope));
        let mut feedback_rx = stream.feedback;
        let result_rx = stream.result;
        let pipeline = self.pipeline.clone();
        let goal_id_clone = goal_id.clone();
        let cb_feedback = on_envelope.clone();
        let cb_result = on_envelope.clone();

        wasm_bindgen_futures::spawn_local(async move {
            while let Some(value) = feedback_rx.recv().await {
                let envelope = serde_json::json!({
                    "kind": "feedback",
                    "value": value,
                });
                emit_envelope(&cb_feedback, &envelope);
            }
        });

        wasm_bindgen_futures::spawn_local(async move {
            let envelope = match result_rx.await {
                Ok(Ok(value)) => serde_json::json!({ "kind": "result", "value": value }),
                Ok(Err(err)) => {
                    serde_json::json!({ "kind": "error", "message": err.to_string() })
                }
                Err(_) => {
                    serde_json::json!({ "kind": "error", "message": "action result channel closed" })
                }
            };
            emit_envelope(&cb_result, &envelope);
            emit_envelope(&cb_result, &serde_json::json!({ "kind": "closed" }));
            pipeline.forget_action_goal(&goal_id_clone).await;
        });

        Ok(goal_id)
    }

    #[wasm_bindgen(js_name = "pipelineCancelActionGoal")]
    pub async fn pipeline_cancel_action_goal(&self, goal_id: String) -> Result<(), JsError> {
        self.pipeline
            .cancel_action_goal(&goal_id)
            .await
            .map_err(|err| JsError::new(&err.to_string()))?;
        Ok(())
    }

    #[wasm_bindgen(js_name = "listRequests")]
    pub async fn list_requests(&self) -> Result<String, JsError> {
        let rows = self.storage.list_requests().await.map_err(storage_err)?;
        serde_json::to_string(&rows).map_err(|err| JsError::new(&err.to_string()))
    }

    #[wasm_bindgen(js_name = "getRequest")]
    pub async fn get_request(&self, id: f64) -> Result<JsValue, JsError> {
        let value = self
            .storage
            .get_request(id as i64)
            .await
            .map_err(storage_err)?;
        match value {
            Some(req) => serde_json::to_string(&req)
                .map(|s| JsValue::from_str(&s))
                .map_err(|err| JsError::new(&err.to_string())),
            None => Ok(JsValue::NULL),
        }
    }

    #[wasm_bindgen(js_name = "createRequest")]
    pub async fn create_request(&self, draft_json: String) -> Result<String, JsError> {
        let draft: NewRequest = serde_json::from_str(&draft_json)
            .map_err(|err| JsError::new(&format!("draft: {err}")))?;
        let created = self
            .storage
            .create_request(draft)
            .await
            .map_err(storage_err)?;
        serde_json::to_string(&created).map_err(|err| JsError::new(&err.to_string()))
    }

    #[wasm_bindgen(js_name = "updateRequest")]
    pub async fn update_request(&self, request_json: String) -> Result<(), JsError> {
        let request: rw_core::domain::Request = serde_json::from_str(&request_json)
            .map_err(|err| JsError::new(&format!("request: {err}")))?;
        self.storage
            .update_request(&request)
            .await
            .map_err(storage_err)
    }

    #[wasm_bindgen(js_name = "deleteRequest")]
    pub async fn delete_request(&self, id: f64) -> Result<(), JsError> {
        self.storage
            .delete_request(id as i64)
            .await
            .map_err(storage_err)
    }

    #[wasm_bindgen(js_name = "listCollections")]
    pub async fn list_collections(&self) -> Result<String, JsError> {
        let rows = self.storage.list_collections().await.map_err(storage_err)?;
        serde_json::to_string(&rows).map_err(|err| JsError::new(&err.to_string()))
    }

    #[wasm_bindgen(js_name = "createCollection")]
    pub async fn create_collection(&self, draft_json: String) -> Result<String, JsError> {
        let draft: NewCollection = serde_json::from_str(&draft_json)
            .map_err(|err| JsError::new(&format!("draft: {err}")))?;
        let created = self
            .storage
            .create_collection(draft)
            .await
            .map_err(storage_err)?;
        serde_json::to_string(&created).map_err(|err| JsError::new(&err.to_string()))
    }

    #[wasm_bindgen(js_name = "updateCollection")]
    pub async fn update_collection(&self, collection_json: String) -> Result<(), JsError> {
        let collection: rw_core::domain::Collection = serde_json::from_str(&collection_json)
            .map_err(|err| JsError::new(&format!("collection: {err}")))?;
        self.storage
            .update_collection(&collection)
            .await
            .map_err(storage_err)
    }

    #[wasm_bindgen(js_name = "deleteCollection")]
    pub async fn delete_collection(&self, id: f64) -> Result<(), JsError> {
        self.storage
            .delete_collection(id as i64)
            .await
            .map_err(storage_err)
    }

    #[wasm_bindgen(js_name = "listConnections")]
    pub async fn list_connections(&self) -> Result<String, JsError> {
        let rows = self.storage.list_connections().await.map_err(storage_err)?;
        serde_json::to_string(&rows).map_err(|err| JsError::new(&err.to_string()))
    }

    #[wasm_bindgen(js_name = "getConnection")]
    pub async fn get_connection(&self, id: f64) -> Result<JsValue, JsError> {
        let value = self
            .storage
            .get_connection(id as i64)
            .await
            .map_err(storage_err)?;
        match value {
            Some(conn) => serde_json::to_string(&conn)
                .map(|s| JsValue::from_str(&s))
                .map_err(|err| JsError::new(&err.to_string())),
            None => Ok(JsValue::NULL),
        }
    }

    #[wasm_bindgen(js_name = "createConnection")]
    pub async fn create_connection(&self, draft_json: String) -> Result<String, JsError> {
        let draft: NewConnection = serde_json::from_str(&draft_json)
            .map_err(|err| JsError::new(&format!("draft: {err}")))?;
        let created = self
            .storage
            .create_connection(draft)
            .await
            .map_err(storage_err)?;
        serde_json::to_string(&created).map_err(|err| JsError::new(&err.to_string()))
    }

    #[wasm_bindgen(js_name = "updateConnection")]
    pub async fn update_connection(&self, connection_json: String) -> Result<(), JsError> {
        let connection: rw_core::domain::Connection = serde_json::from_str(&connection_json)
            .map_err(|err| JsError::new(&format!("connection: {err}")))?;
        self.storage
            .update_connection(&connection)
            .await
            .map_err(storage_err)
    }

    #[wasm_bindgen(js_name = "deleteConnection")]
    pub async fn delete_connection(&self, id: f64) -> Result<(), JsError> {
        self.storage
            .delete_connection(id as i64)
            .await
            .map_err(storage_err)
    }

    #[wasm_bindgen(js_name = "clearWorkspaceStorage")]
    pub async fn clear_workspace_storage(&self) -> Result<(), JsError> {
        self.storage.clear_all().await.map_err(storage_err)
    }

    #[wasm_bindgen(js_name = "exportWorkspace")]
    pub async fn export_workspace(&self) -> Result<String, JsError> {
        let clock: std::sync::Arc<dyn rw_core::util::Clock> =
            std::sync::Arc::new(rw_core::util::SystemClock::new());
        let file = rw_core::storage::export_workspace(self.storage.as_ref(), clock)
            .await
            .map_err(storage_err)?;
        serde_json::to_string_pretty(&file).map_err(|err| JsError::new(&err.to_string()))
    }

    #[wasm_bindgen(js_name = "importWorkspace")]
    pub async fn import_workspace(
        &self,
        file_json: String,
        mode: String,
    ) -> Result<String, JsError> {
        let file: rw_core::domain::WorkspaceFile = serde_json::from_str(&file_json)
            .map_err(|err| JsError::new(&format!("file: {err}")))?;
        let mode = if mode == "replace" {
            rw_core::storage::ImportMode::Replace
        } else {
            rw_core::storage::ImportMode::Merge
        };
        let report = rw_core::storage::import_workspace(self.storage.as_ref(), file, mode)
            .await
            .map_err(storage_err)?;
        serde_json::to_string(&report).map_err(|err| JsError::new(&err.to_string()))
    }

    #[wasm_bindgen(js_name = "listSchemasSummary")]
    pub fn list_schemas_summary(&self) -> Result<String, JsError> {
        let rows = self.schemas.list_summaries();
        serde_json::to_string(&rows).map_err(|err| JsError::new(&err.to_string()))
    }

    #[wasm_bindgen(js_name = "listSchemasByName")]
    pub fn list_schemas_by_name(&self, name: String) -> Result<String, JsError> {
        let rows = self.schemas.get_by_name(&name);
        serde_json::to_string(&rows).map_err(|err| JsError::new(&err.to_string()))
    }

    #[wasm_bindgen(js_name = "getSchemaByHash")]
    pub fn get_schema_by_hash(&self, hash: String) -> Result<JsValue, JsError> {
        match self.schemas.get_by_hash(&hash) {
            Some(def) => serde_json::to_string(&def)
                .map(|s| JsValue::from_str(&s))
                .map_err(|err| JsError::new(&err.to_string())),
            None => Ok(JsValue::NULL),
        }
    }

    #[wasm_bindgen(js_name = "registerSchema")]
    pub async fn register_schema(
        &self,
        name: String,
        kind: String,
        definition: String,
    ) -> Result<String, JsError> {
        let parsed_kind = match kind.as_str() {
            "message" => SchemaKind::Message,
            "service" => SchemaKind::Service,
            "action" => SchemaKind::Action,
            other => return Err(JsError::new(&format!("unknown schema kind: {other}"))),
        };
        let schema_ref = self
            .schemas
            .register(&name, parsed_kind, &definition)
            .await
            .map_err(|err| JsError::new(&err.to_string()))?;
        serde_json::to_string(&schema_ref).map_err(|err| JsError::new(&err.to_string()))
    }
}

fn storage_err(err: rw_core::CoreError) -> JsError {
    JsError::new(&err.to_string())
}

fn emit_status(cb: &Rc<RefCell<js_sys::Function>>, status: &rw_transport::ConnectionStatus) {
    let json = match serde_json::to_string(&status_to_legacy_json(status)) {
        Ok(s) => JsValue::from_str(&s),
        Err(_) => return,
    };
    let this = JsValue::NULL;
    let _ = cb.borrow().call1(&this, &json);
}

fn status_to_legacy_json(status: &rw_transport::ConnectionStatus) -> serde_json::Value {
    use rw_transport::ConnectionStatus::*;
    match status {
        Disconnected => serde_json::Value::String("disconnected".into()),
        Connecting => serde_json::Value::String("connecting".into()),
        Connected => serde_json::Value::String("connected".into()),
        Reconnecting => serde_json::Value::String("reconnecting".into()),
        Failed(reason) => serde_json::json!({ "failed": reason }),
    }
}

fn emit_discovery(cb: &Rc<RefCell<js_sys::Function>>, snapshot: &rw_transport::Discovery) {
    let payload = rw_transport::discovery_to_json(snapshot);
    let json = match serde_json::to_string(&payload) {
        Ok(s) => JsValue::from_str(&s),
        Err(_) => return,
    };
    let this = JsValue::NULL;
    let _ = cb.borrow().call1(&this, &json);
}

fn emit_envelope(cb: &Rc<RefCell<js_sys::Function>>, value: &serde_json::Value) {
    let payload = match serde_json::to_string(value) {
        Ok(s) => JsValue::from_str(&s),
        Err(_) => return,
    };
    let this = JsValue::NULL;
    let _ = cb.borrow().call1(&this, &payload);
}

fn parse_connection_id(connection_id: &str) -> Result<ConnectionId, JsError> {
    let uuid = uuid::Uuid::from_str(connection_id)
        .map_err(|err| JsError::new(&format!("invalid connection id: {err}")))?;
    Ok(ConnectionId(uuid))
}

#[derive(serde::Serialize)]
struct SubscribeResponse {
    subscription_id: String,
    schema_id: String,
    schema_name: String,
    viz_role: String,
}

fn build_frame_js(handle: &str, frame: &rw_transport::Frame, is_replay: bool) -> JsValue {
    let viz_role = frame.schema.viz_role.wire_id();
    if viz_role == "image" {
        if let Some(image) = image_value_to_rgba(&frame.value) {
            return build_image_frame_js(handle, frame, is_replay, &viz_role, image);
        }
    }
    let obj = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("subscriptionId"),
        &JsValue::from_str(handle),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("schemaId"),
        &JsValue::from_str(frame.schema.id.as_str()),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("schemaName"),
        &JsValue::from_str(&frame.schema.name),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("vizRole"),
        &JsValue::from_str(&viz_role),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("timestampNs"),
        &JsValue::from_f64(frame.timestamp_ns as f64),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("isReplay"),
        &JsValue::from_bool(is_replay),
    );
    let pack_start_ns = frame.perf.as_ref().map(|_| rw_transport::perf::now_ns());
    let value_js = match serde_json::to_string(&frame.value) {
        Ok(json) => match js_sys::JSON::parse(&json) {
            Ok(v) => v,
            Err(err) => {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "[rw-wasm] JSON.parse of frame value failed for {}: {:?}",
                    frame.schema.name, err
                )));
                JsValue::NULL
            }
        },
        Err(err) => {
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "[rw-wasm] serde_json::to_string failed for {}: {}",
                frame.schema.name, err
            )));
            JsValue::NULL
        }
    };
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("value"), &value_js);
    if let (Some(perf), Some(pack_start_ns)) = (frame.perf.as_ref(), pack_start_ns) {
        attach_perf(&obj, perf, pack_start_ns);
    }
    obj.into()
}

fn attach_perf(obj: &js_sys::Object, perf: &rw_transport::perf::PerfTrace, pack_start_ns: u64) {
    let channel_send_ns = rw_transport::perf::now_ns();
    let perf_obj = js_sys::Object::new();
    let set = |name: &str, value: u64| {
        let _ = js_sys::Reflect::set(
            &perf_obj,
            &JsValue::from_str(name),
            &JsValue::from_f64(value as f64),
        );
    };
    set("wsRecvNs", perf.ws_recv_ns);
    set("decodeStartNs", perf.decode_start_ns);
    set("decodeEndNs", perf.decode_end_ns);
    set("packStartNs", pack_start_ns);
    set("channelSendNs", channel_send_ns);
    set("workerRecvNs", 0);
    set("workerDecodedNs", 0);
    let _ = js_sys::Reflect::set(obj, &JsValue::from_str("perf"), &perf_obj);
}

fn build_image_frame_js(
    handle: &str,
    frame: &rw_transport::Frame,
    is_replay: bool,
    viz_role: &str,
    image: ImageRgba,
) -> JsValue {
    let obj = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("subscriptionId"),
        &JsValue::from_str(handle),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("schemaId"),
        &JsValue::from_str(frame.schema.id.as_str()),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("schemaName"),
        &JsValue::from_str(&frame.schema.name),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("vizRole"),
        &JsValue::from_str(viz_role),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("timestampNs"),
        &JsValue::from_f64(frame.timestamp_ns as f64),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("isReplay"),
        &JsValue::from_bool(is_replay),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("width"),
        &JsValue::from_f64(image.width as f64),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("height"),
        &JsValue::from_f64(image.height as f64),
    );

    let view = unsafe { js_sys::Uint8ClampedArray::view(&image.rgba) };
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("rgba"), &view);
    obj.into()
}
