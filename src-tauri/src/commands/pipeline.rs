use rw_core::visualization::image::image_value_to_rgba;
use rw_transport::{ConnectionId, SubscribeOptions};
use rw_wire::{
    flags as wire_flags, now_ns, pack_frame_raw, pack_frame_with_cbor_perf, perf_trace_enabled,
    FrameKind,
};
use serde::Deserialize;
use serde::Serialize;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::State;

use crate::ingest_ws::IngestHub;
use crate::pipeline::CanonicalPipeline;

#[derive(Debug, thiserror::Error)]
pub enum PipelineCommandError {
    #[error("transport: {0}")]
    Transport(String),
    #[error("invalid argument: {0}")]
    Invalid(String),
}

impl serde::Serialize for PipelineCommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let (kind, message): (&str, &str) = match self {
            PipelineCommandError::Transport(message) => ("transport", message),
            PipelineCommandError::Invalid(message) => ("invalid", message),
        };
        let mut state = serializer.serialize_struct("PipelineCommandError", 2)?;
        state.serialize_field("kind", kind)?;
        state.serialize_field("message", message)?;
        state.end()
    }
}

#[tauri::command]
pub async fn pipeline_open_foxglove(
    pipeline: State<'_, CanonicalPipeline>,
    url: String,
) -> Result<String, PipelineCommandError> {
    let id = pipeline
        .open_foxglove(url)
        .await
        .map_err(|err| PipelineCommandError::Transport(err.to_string()))?;
    Ok(id.to_string())
}

#[tauri::command]
pub async fn pipeline_open_rosbridge(
    pipeline: State<'_, CanonicalPipeline>,
    url: String,
) -> Result<String, PipelineCommandError> {
    let id = pipeline
        .open_rosbridge(url)
        .await
        .map_err(|err| PipelineCommandError::Transport(err.to_string()))?;
    Ok(id.to_string())
}

#[tauri::command]
pub async fn pipeline_open_dummy(
    pipeline: State<'_, CanonicalPipeline>,
) -> Result<String, PipelineCommandError> {
    let id = pipeline
        .open_dummy()
        .await
        .map_err(|err| PipelineCommandError::Transport(err.to_string()))?;
    Ok(id.to_string())
}

#[tauri::command]
pub async fn pipeline_close(
    pipeline: State<'_, CanonicalPipeline>,
    connection_id: String,
) -> Result<(), PipelineCommandError> {
    let id = parse_connection_id(&connection_id)?;
    pipeline
        .close(id)
        .await
        .map_err(|err| PipelineCommandError::Transport(err.to_string()))
}

#[tauri::command]
pub async fn pipeline_status(
    pipeline: State<'_, CanonicalPipeline>,
    connection_id: String,
) -> Result<String, PipelineCommandError> {
    let id = parse_connection_id(&connection_id)?;
    let transport = pipeline
        .transport(id)
        .await
        .map_err(|err| PipelineCommandError::Transport(err.to_string()))?;
    let status = transport.status().borrow().clone();
    serde_json::to_string(&status).map_err(|err| PipelineCommandError::Invalid(err.to_string()))
}

#[tauri::command]
pub async fn pipeline_discovery(
    pipeline: State<'_, CanonicalPipeline>,
    connection_id: String,
) -> Result<String, PipelineCommandError> {
    let id = parse_connection_id(&connection_id)?;
    let transport = pipeline
        .transport(id)
        .await
        .map_err(|err| PipelineCommandError::Transport(err.to_string()))?;
    let discovery = transport.discovery().borrow().clone();
    let json = rw_transport::discovery_to_json(&discovery);
    serde_json::to_string(&json).map_err(|err| PipelineCommandError::Invalid(err.to_string()))
}

fn parse_connection_id(raw: &str) -> Result<ConnectionId, PipelineCommandError> {
    raw.parse::<uuid::Uuid>()
        .map(ConnectionId)
        .map_err(|err| PipelineCommandError::Invalid(format!("connection id: {err}")))
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineSubscribeResponse {
    pub subscription_id: String,
    pub schema_id: String,
    pub schema_name: String,
    pub viz_role: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PipelineSubscribeOptions {
    #[serde(default)]
    pub target_hz: Option<f32>,
    #[serde(default)]
    pub queue_length: Option<u32>,
    #[serde(default)]
    pub fields: Option<Vec<String>>,
}

impl From<PipelineSubscribeOptions> for SubscribeOptions {
    fn from(value: PipelineSubscribeOptions) -> Self {
        SubscribeOptions {
            target_hz: value.target_hz,
            queue_length: value.queue_length,
        }
    }
}

fn pack_value_frame(
    handle: &str,
    frame: &rw_transport::Frame,
    is_replay: bool,
    fields: &Option<Vec<String>>,
    payload_hint: &mut usize,
) -> Option<Vec<u8>> {
    let mut flags = 0u16;
    if is_replay {
        flags |= wire_flags::STALE_REPLAY;
    }
    let perf_enabled = perf_trace_enabled();
    let mut perf = frame.perf;
    if perf_enabled {
        if let Some(trace) = perf.as_mut() {
            trace.pack_start_ns = now_ns();
        }
    }

    let image = if frame.schema.viz_role.wire_id() == "image" {
        image_value_to_rgba(&frame.value)
    } else {
        None
    };

    let mut packed = match image {
        Some(image) => pack_frame_raw(
            handle,
            frame.timestamp_ns,
            FrameKind::Image,
            flags,
            &pack_image_payload(&image),
            perf,
        ),
        None => {
            let bytes = match pack_cbor_value(handle, frame, flags, fields, *payload_hint, perf) {
                Ok(bytes) => bytes,
                Err(err) => {
                    tracing::warn!(?err, "canonical value -> cbor envelope failed");
                    return None;
                }
            };
            *payload_hint = (*payload_hint * 7 / 8) + (bytes.len() / 8);
            bytes
        }
    };

    if perf_enabled && perf.is_some() {
        stamp_channel_send(&mut packed);
    }
    Some(packed)
}

fn pack_image_payload(image: &rw_core::visualization::image::ImageRgba) -> Vec<u8> {
    let mut payload = Vec::with_capacity(16 + image.rgba.len());
    payload.extend_from_slice(&image.width.to_le_bytes());
    payload.extend_from_slice(&image.height.to_le_bytes());
    payload.extend_from_slice(&[0u8; 8]);
    payload.extend_from_slice(&image.rgba);
    payload
}

fn pack_cbor_value(
    handle: &str,
    frame: &rw_transport::Frame,
    flags: u16,
    fields: &Option<Vec<String>>,
    payload_hint: usize,
    perf: Option<rw_wire::PerfTrace>,
) -> Result<Vec<u8>, rw_wire::CborPackError> {
    match fields {
        Some(selected) => pack_frame_with_cbor_perf(
            handle,
            frame.timestamp_ns,
            FrameKind::Value,
            flags,
            &rw_canonical::ProjectedValue::new(&frame.value, selected),
            payload_hint,
            perf,
        ),
        None => pack_frame_with_cbor_perf(
            handle,
            frame.timestamp_ns,
            FrameKind::Value,
            flags,
            &frame.value,
            payload_hint,
            perf,
        ),
    }
}

fn stamp_channel_send(packed: &mut [u8]) {
    let tail = packed.len().saturating_sub(8);
    packed[tail..tail + 8].copy_from_slice(&now_ns().to_le_bytes());
}

#[tauri::command]
pub async fn ingest_ws_port(hub: State<'_, IngestHub>) -> Result<u16, PipelineCommandError> {
    for _ in 0..200 {
        if let Some(port) = hub.port() {
            return Ok(port);
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    Err(PipelineCommandError::Transport(
        "ingest ws server not ready".into(),
    ))
}

#[tauri::command]
pub async fn pipeline_subscribe_topic(
    pipeline: State<'_, CanonicalPipeline>,
    hub: State<'_, IngestHub>,
    connection_id: String,
    topic: String,
    options: Option<PipelineSubscribeOptions>,
) -> Result<PipelineSubscribeResponse, PipelineCommandError> {
    let cid = parse_connection_id(&connection_id)?;
    let raw_opts = options.unwrap_or_default();
    let fields = raw_opts.fields.clone().filter(|f| !f.is_empty());
    let opts: SubscribeOptions = raw_opts.into();
    let hub = hub.inner().clone();
    let mut payload_hint: usize = 4096;
    let response = pipeline
        .subscribe_topic_with_options(cid, &topic, opts, move |handle, frame, is_replay| {
            if let Some(packed) =
                pack_value_frame(handle, frame, is_replay, &fields, &mut payload_hint)
            {
                hub.send(packed);
            }
        })
        .await
        .map_err(|err| PipelineCommandError::Transport(err.to_string()))?;
    Ok(PipelineSubscribeResponse {
        subscription_id: response.subscription_id,
        schema_id: response.schema_id,
        schema_name: response.schema_name,
        viz_role: response.viz_role,
    })
}

#[tauri::command]
pub async fn pipeline_unsubscribe(
    pipeline: State<'_, CanonicalPipeline>,
    subscription_id: String,
) -> Result<(), PipelineCommandError> {
    pipeline
        .unsubscribe(&subscription_id)
        .await
        .map_err(|err| PipelineCommandError::Transport(err.to_string()))
}

#[tauri::command]
pub async fn pipeline_call_service(
    pipeline: State<'_, CanonicalPipeline>,
    connection_id: String,
    service: String,
    request_json: String,
) -> Result<String, PipelineCommandError> {
    let cid = parse_connection_id(&connection_id)?;
    let request: rw_canonical::CanonicalValue = serde_json::from_str(&request_json)
        .map_err(|err| PipelineCommandError::Invalid(format!("request_json: {err}")))?;
    let response = pipeline
        .call_service(cid, &service, request)
        .await
        .map_err(|err| PipelineCommandError::Transport(err.to_string()))?;
    serde_json::to_string(&response)
        .map_err(|err| PipelineCommandError::Invalid(format!("response_json: {err}")))
}

#[tauri::command]
pub async fn pipeline_send_action_goal(
    pipeline: State<'_, CanonicalPipeline>,
    connection_id: String,
    action: String,
    goal_json: String,
    channel: Channel<InvokeResponseBody>,
) -> Result<String, PipelineCommandError> {
    let cid = parse_connection_id(&connection_id)?;
    let goal: rw_canonical::CanonicalValue = serde_json::from_str(&goal_json)
        .map_err(|err| PipelineCommandError::Invalid(format!("goal_json: {err}")))?;
    let stream = pipeline
        .send_action_goal(cid, &action, goal)
        .await
        .map_err(|err| PipelineCommandError::Transport(err.to_string()))?;
    let goal_id = stream.cancel_token.goal_id.clone();
    let fb_channel = channel.clone();
    let mut feedback_rx = stream.feedback;
    tokio::spawn(async move {
        while let Some(value) = feedback_rx.recv().await {
            let envelope = serde_json::json!({ "kind": "feedback", "value": value });
            if let Ok(bytes) = serde_json::to_vec(&envelope) {
                let _ = fb_channel.send(InvokeResponseBody::Raw(bytes));
            }
        }
    });
    let result_channel = channel;
    let result_rx = stream.result;
    let pipeline_clone: CanonicalPipeline = pipeline.inner().clone();
    let goal_id_clone = goal_id.clone();
    tokio::spawn(async move {
        let envelope = match result_rx.await {
            Ok(Ok(value)) => serde_json::json!({ "kind": "result", "value": value }),
            Ok(Err(err)) => serde_json::json!({ "kind": "error", "message": err.to_string() }),
            Err(_) => {
                serde_json::json!({ "kind": "error", "message": "action result channel closed" })
            }
        };
        if let Ok(bytes) = serde_json::to_vec(&envelope) {
            let _ = result_channel.send(InvokeResponseBody::Raw(bytes));
        }
        let closed = serde_json::json!({ "kind": "closed" });
        if let Ok(bytes) = serde_json::to_vec(&closed) {
            let _ = result_channel.send(InvokeResponseBody::Raw(bytes));
        }
        pipeline_clone.forget_action_goal(&goal_id_clone).await;
    });
    Ok(goal_id)
}

#[tauri::command]
pub async fn pipeline_cancel_action_goal(
    pipeline: State<'_, CanonicalPipeline>,
    goal_id: String,
) -> Result<(), PipelineCommandError> {
    pipeline
        .cancel_action_goal(&goal_id)
        .await
        .map_err(|err| PipelineCommandError::Transport(err.to_string()))
}
