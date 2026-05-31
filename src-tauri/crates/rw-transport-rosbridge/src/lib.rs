#![deny(missing_debug_implementations)]

mod wire;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use rw_canonical::{
    canonical_schema_id, CanonicalSchema, CanonicalValue, Dialect, MessageDef, ParsedSchema,
    SchemaId, SchemaKind, VisualizationRole,
};
use rw_codec_json::{json_to_canonical, json_to_canonical_with_schema, Resolver as JsonResolver};
use rw_transport::time::{sleep, timeout};
use rw_transport::{
    ActionCancelToken, ActionGoalStream, ConnectionStatus, Discovery, Frame, SubscribeOptions,
    Subscription, TargetDescriptor, TopicDescriptor, Transport, TransportError, TransportResult,
};
use rw_ws::{self as ws, WsMsg};
use serde_json::{json, Value as JsonValue};
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use tracing::{debug, warn};

use rw_transport::task::{spawn_detached, spawn_task, SpawnedTask};
use wire::{ClientOp, ServerOp};

#[derive(Debug, Clone)]
pub struct RosbridgeConfig {
    pub url: String,
    pub connect_timeout: Duration,
}

impl RosbridgeConfig {
    pub fn new(url: impl Into<String>) -> Self {
        RosbridgeConfig {
            url: url.into(),
            connect_timeout: Duration::from_secs(15),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RosbridgeTransport {
    inner: Arc<TransportInner>,
}

#[derive(Debug)]
struct TransportInner {
    config: RosbridgeConfig,
    detected_dialect: std::sync::RwLock<Dialect>,
    status_tx: watch::Sender<ConnectionStatus>,
    status_rx: watch::Receiver<ConnectionStatus>,
    discovery_tx: watch::Sender<Discovery>,
    discovery_rx: watch::Receiver<Discovery>,
    command_tx: Mutex<Option<mpsc::Sender<Command>>>,
    actor_handle: Mutex<Option<SpawnedTask>>,
}

impl TransportInner {
    fn dialect(&self) -> Dialect {
        self.detected_dialect
            .read()
            .map(|guard| guard.clone())
            .unwrap_or(Dialect::Ros2)
    }
}

#[derive(Debug)]
enum Command {
    Subscribe {
        topic: String,
        schema: Option<Arc<CanonicalSchema>>,
        options: SubscribeOptions,
        reply: oneshot::Sender<TransportResult<Subscription>>,
    },
    Unsubscribe {
        topic: String,
    },
    LookupTopic {
        topic: String,
        reply: oneshot::Sender<Option<TopicInfo>>,
    },
    CacheTopic {
        topic: String,
        info: TopicInfo,
    },
    CallService {
        service: String,
        request: CanonicalValue,
        reply: oneshot::Sender<TransportResult<CanonicalValue>>,
    },
    Publish {
        topic: String,
        payload: JsonValue,
        reply: oneshot::Sender<TransportResult<()>>,
    },
    SendActionGoal {
        action: String,
        action_type: String,
        args: JsonValue,
        feedback_tx: mpsc::Sender<CanonicalValue>,
        result_tx: oneshot::Sender<TransportResult<CanonicalValue>>,
        token_reply: oneshot::Sender<ActionCancelToken>,
    },
    CancelActionGoal {
        action: String,
        goal_id: String,
        reply: oneshot::Sender<TransportResult<()>>,
    },
    Shutdown,
}

impl RosbridgeTransport {
    async fn lookup_topic(&self, topic: &str) -> TransportResult<Option<TopicInfo>> {
        let tx = self
            .inner
            .command_tx
            .lock()
            .await
            .clone()
            .ok_or(TransportError::NotConnected)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(Command::LookupTopic {
            topic: topic.into(),
            reply: reply_tx,
        })
        .await
        .map_err(|_| TransportError::Closed)?;
        reply_rx.await.map_err(|_| TransportError::Closed)
    }

    async fn cache_topic(&self, topic: &str, info: TopicInfo) -> TransportResult<()> {
        let tx = self
            .inner
            .command_tx
            .lock()
            .await
            .clone()
            .ok_or(TransportError::NotConnected)?;
        tx.send(Command::CacheTopic {
            topic: topic.into(),
            info,
        })
        .await
        .map_err(|_| TransportError::Closed)?;
        Ok(())
    }

    async fn resolve_topic_via_rosapi(&self, topic: &str) -> TransportResult<TopicInfo> {
        let topic_arg = CanonicalValue::Struct(std::collections::BTreeMap::from([(
            "topic".into(),
            CanonicalValue::String(topic.into()),
        )]));
        let type_response = self.call_service("/rosapi/topic_type", topic_arg).await?;
        let raw_name = canonical_string_field(&type_response, "type").ok_or_else(|| {
            TransportError::Schema(topic.into(), "rosapi/topic_type missing 'type'".into())
        })?;
        let schema_name = normalise_schema_name(&raw_name);

        let details = self
            .request_message_details(&raw_name, &schema_name)
            .await?;
        let details_json = rw_codec_json::canonical_to_json(
            &details,
            matches!(self.inner.dialect(), Dialect::Ros1),
        );
        let schema = build_canonical_schema(&schema_name, &details_json, self.dialect());
        let resolver = build_resolver(&details_json);
        let info = TopicInfo {
            schema_name,
            schema: Arc::new(schema),
            resolver,
        };
        self.cache_topic(topic, info.clone()).await?;
        Ok(info)
    }

    async fn request_message_details(
        &self,
        raw_name: &str,
        normalised: &str,
    ) -> TransportResult<CanonicalValue> {
        let attempt = |name: String| {
            let value = CanonicalValue::Struct(std::collections::BTreeMap::from([(
                "type".into(),
                CanonicalValue::String(name),
            )]));
            async move { self.call_service("/rosapi/message_details", value).await }
        };
        let first = attempt(raw_name.into()).await?;
        if !is_empty_typedefs(&first) {
            return Ok(first);
        }
        let long_form = match normalised.split_once('/') {
            Some((pkg, ty)) if !pkg.is_empty() && !ty.is_empty() => {
                format!("{pkg}/msg/{ty}")
            }
            _ => return Ok(first),
        };
        if long_form == raw_name {
            return Ok(first);
        }
        let second = attempt(long_form).await?;
        if is_empty_typedefs(&second) {
            return Ok(first);
        }
        Ok(second)
    }

    async fn resolve_action_type(&self, action: &str) -> TransportResult<String> {
        let action_arg = CanonicalValue::Struct(std::collections::BTreeMap::from([(
            "action".into(),
            CanonicalValue::String(action.into()),
        )]));
        if let Ok(response) = self.call_service("/rosapi/action_type", action_arg).await {
            if let Some(name) = canonical_string_field(&response, "type") {
                return Ok(normalise_schema_name(&name));
            }
        }

        let send_goal_service = format!("{action}/_action/send_goal");
        let service_arg = CanonicalValue::Struct(std::collections::BTreeMap::from([(
            "service".into(),
            CanonicalValue::String(send_goal_service.clone()),
        )]));
        let response = self
            .call_service("/rosapi/service_type", service_arg)
            .await
            .map_err(|err| {
                TransportError::Schema(
                    action.into(),
                    format!("could not resolve action type via {send_goal_service}: {err}"),
                )
            })?;
        let raw_service_type = canonical_string_field(&response, "type").ok_or_else(|| {
            TransportError::Schema(
                action.into(),
                "rosapi/service_type returned no 'type'".into(),
            )
        })?;
        let normalised = normalise_schema_name(&raw_service_type);
        let action_type = normalised
            .strip_suffix("_SendGoal")
            .map(|s| s.to_string())
            .unwrap_or(normalised);
        Ok(action_type)
    }

    pub fn new(config: RosbridgeConfig) -> Self {
        let (status_tx, status_rx) = watch::channel(ConnectionStatus::Disconnected);
        let (discovery_tx, discovery_rx) = watch::channel(Discovery::default());
        RosbridgeTransport {
            inner: Arc::new(TransportInner {
                config,
                detected_dialect: std::sync::RwLock::new(Dialect::Ros2),
                status_tx,
                status_rx,
                discovery_tx,
                discovery_rx,
                command_tx: Mutex::new(None),
                actor_handle: Mutex::new(None),
            }),
        }
    }

    fn dialect(&self) -> Dialect {
        self.inner.dialect()
    }

    async fn detect_ros_dialect(&self) -> Dialect {
        let response = self
            .call_service(
                "/rosapi/topics",
                CanonicalValue::Struct(std::collections::BTreeMap::new()),
            )
            .await;
        match response {
            Ok(value) => dialect_from_topic_types(&value),
            Err(_) => Dialect::Ros2,
        }
    }
}

fn dialect_from_topic_types(value: &CanonicalValue) -> Dialect {
    let CanonicalValue::Struct(fields) = value else {
        return Dialect::Ros2;
    };
    let Some(CanonicalValue::Array(types)) = fields.get("types") else {
        return Dialect::Ros2;
    };
    let mut saw_two_segment = false;
    for entry in types {
        let CanonicalValue::String(name) = entry else {
            continue;
        };
        let segments: Vec<&str> = name.split('/').collect();
        if segments.len() == 3 && matches!(segments[1], "msg" | "srv" | "action") {
            return Dialect::Ros2;
        }
        if segments.len() == 2 {
            saw_two_segment = true;
        }
    }
    if saw_two_segment {
        Dialect::Ros1
    } else {
        Dialect::Ros2
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl Transport for RosbridgeTransport {
    async fn connect(&self) -> TransportResult<()> {
        {
            let mut command_slot = self.inner.command_tx.lock().await;
            if command_slot.is_some() {
                return Ok(());
            }
            let (command_tx, command_rx) = mpsc::channel::<Command>(64);
            let _ = self.inner.status_tx.send(ConnectionStatus::Connecting);
            let socket = open_socket(&self.inner.config).await?;
            let _ = self.inner.status_tx.send(ConnectionStatus::Connected);
            let handle = Actor::spawn(self.inner.clone(), socket, command_rx, command_tx.clone());
            *command_slot = Some(command_tx);
            *self.inner.actor_handle.lock().await = Some(handle);
        }
        let dialect = self.detect_ros_dialect().await;
        if let Ok(mut guard) = self.inner.detected_dialect.write() {
            *guard = dialect;
        }
        Ok(())
    }

    async fn disconnect(&self) -> TransportResult<()> {
        let tx = self.inner.command_tx.lock().await.take();
        if let Some(tx) = tx {
            let _ = tx.send(Command::Shutdown).await;
        }
        if let Some(handle) = self.inner.actor_handle.lock().await.take() {
            #[cfg(not(target_family = "wasm"))]
            let _ = handle.await;
            #[cfg(target_family = "wasm")]
            let _: SpawnedTask = handle;
        }
        let _ = self.inner.status_tx.send(ConnectionStatus::Disconnected);
        Ok(())
    }

    fn status(&self) -> watch::Receiver<ConnectionStatus> {
        self.inner.status_rx.clone()
    }

    fn discovery(&self) -> watch::Receiver<Discovery> {
        self.inner.discovery_rx.clone()
    }

    async fn subscribe_topic(&self, topic: &str) -> TransportResult<Subscription> {
        self.subscribe_topic_with_options(topic, SubscribeOptions::default())
            .await
    }

    async fn subscribe_topic_with_options(
        &self,
        topic: &str,
        options: SubscribeOptions,
    ) -> TransportResult<Subscription> {
        let info = match self.lookup_topic(topic).await? {
            Some(info) => info,
            None => self.resolve_topic_via_rosapi(topic).await?,
        };

        let tx = self
            .inner
            .command_tx
            .lock()
            .await
            .clone()
            .ok_or(TransportError::NotConnected)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(Command::Subscribe {
            topic: topic.into(),
            schema: Some(info.schema.clone()),
            options,
            reply: reply_tx,
        })
        .await
        .map_err(|_| TransportError::Closed)?;
        reply_rx.await.map_err(|_| TransportError::Closed)?
    }

    async fn publish(&self, topic: &str, value: CanonicalValue) -> TransportResult<()> {
        let json =
            rw_codec_json::canonical_to_json(&value, matches!(self.inner.dialect(), Dialect::Ros1));
        let tx = self
            .inner
            .command_tx
            .lock()
            .await
            .clone()
            .ok_or(TransportError::NotConnected)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(Command::Publish {
            topic: topic.into(),
            payload: json,
            reply: reply_tx,
        })
        .await
        .map_err(|_| TransportError::Closed)?;
        reply_rx.await.map_err(|_| TransportError::Closed)?
    }

    async fn call_service(
        &self,
        service: &str,
        request: CanonicalValue,
    ) -> TransportResult<CanonicalValue> {
        let tx = self
            .inner
            .command_tx
            .lock()
            .await
            .clone()
            .ok_or(TransportError::NotConnected)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(Command::CallService {
            service: service.into(),
            request,
            reply: reply_tx,
        })
        .await
        .map_err(|_| TransportError::Closed)?;
        reply_rx.await.map_err(|_| TransportError::Closed)?
    }

    async fn send_action_goal(
        &self,
        action: &str,
        goal: CanonicalValue,
    ) -> TransportResult<ActionGoalStream> {
        let action_type = self.resolve_action_type(action).await?;

        let args =
            rw_codec_json::canonical_to_json(&goal, matches!(self.inner.dialect(), Dialect::Ros1));
        let tx = self
            .inner
            .command_tx
            .lock()
            .await
            .clone()
            .ok_or(TransportError::NotConnected)?;
        let (feedback_tx, feedback_rx) = mpsc::channel::<CanonicalValue>(64);
        let (result_tx, result_rx) = oneshot::channel();
        let (token_tx, token_rx) = oneshot::channel();
        let feedback_monitor = feedback_tx.clone();
        tx.send(Command::SendActionGoal {
            action: action.into(),
            action_type,
            args,
            feedback_tx,
            result_tx,
            token_reply: token_tx,
        })
        .await
        .map_err(|_| TransportError::Closed)?;
        let cancel_token = token_rx.await.map_err(|_| TransportError::Closed)?;
        {
            let tx_clone = tx.clone();
            let token_clone = cancel_token.clone();
            spawn_detached(async move {
                feedback_monitor.closed().await;
                let (reply_tx, _reply_rx) = oneshot::channel();
                let _ = tx_clone
                    .send(Command::CancelActionGoal {
                        action: token_clone.action,
                        goal_id: token_clone.goal_id,
                        reply: reply_tx,
                    })
                    .await;
            });
        }
        Ok(ActionGoalStream {
            feedback: feedback_rx,
            result: result_rx,
            cancel_token,
        })
    }

    async fn cancel_action_goal(&self, token: &ActionCancelToken) -> TransportResult<()> {
        let tx = self
            .inner
            .command_tx
            .lock()
            .await
            .clone()
            .ok_or(TransportError::NotConnected)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(Command::CancelActionGoal {
            action: token.action.clone(),
            goal_id: token.goal_id.clone(),
            reply: reply_tx,
        })
        .await
        .map_err(|_| TransportError::Closed)?;
        reply_rx.await.map_err(|_| TransportError::Closed)?
    }
}

type Socket = ws::WsConnection;

async fn open_socket(config: &RosbridgeConfig) -> TransportResult<Socket> {
    ws::connect(&config.url, config.connect_timeout, &[])
        .await
        .map_err(|err| TransportError::Other(err.to_string()))
}

struct Actor {
    inner: Arc<TransportInner>,
    socket: Socket,
    command_rx: mpsc::Receiver<Command>,
    self_tx: mpsc::Sender<Command>,
    subscribers: HashMap<String, SubscriberSlot>,
    pending_service_calls: HashMap<String, PendingCall>,
    pending_action_goals: HashMap<String, PendingActionGoal>,
    topic_types: HashMap<String, TopicInfo>,
    discovered: HashMap<String, String>,
    discovered_services: HashMap<String, String>,
    discovered_actions: HashMap<String, String>,
    next_call_id: u64,
}

#[derive(Debug)]
struct SubscriberSlot {
    sender: mpsc::Sender<Frame>,
    schema: Arc<CanonicalSchema>,
    resolver: JsonResolver,
    options: SubscribeOptions,
}

#[derive(Debug)]
struct PendingActionGoal {
    feedback_tx: mpsc::Sender<CanonicalValue>,
    result_tx: oneshot::Sender<TransportResult<CanonicalValue>>,
}

#[derive(Debug)]
enum PendingCall {
    Internal(oneshot::Sender<TransportResult<JsonValue>>),
    User(oneshot::Sender<TransportResult<CanonicalValue>>),
}

#[derive(Debug, Clone)]
struct TopicInfo {
    schema_name: String,
    schema: Arc<CanonicalSchema>,
    resolver: JsonResolver,
}

impl Actor {
    fn spawn(
        inner: Arc<TransportInner>,
        socket: Socket,
        command_rx: mpsc::Receiver<Command>,
        self_tx: mpsc::Sender<Command>,
    ) -> SpawnedTask {
        let actor = Actor {
            inner,
            socket,
            command_rx,
            self_tx,
            subscribers: HashMap::new(),
            pending_service_calls: HashMap::new(),
            pending_action_goals: HashMap::new(),
            topic_types: HashMap::new(),
            discovered: HashMap::new(),
            discovered_services: HashMap::new(),
            discovered_actions: HashMap::new(),
            next_call_id: 1,
        };
        spawn_task(async move {
            let _ = actor.run().await;
        })
    }

    async fn run(mut self) -> TransportResult<()> {
        let topics_call_id = self.next_request_id();
        self.send_op(ClientOp::CallService {
            id: topics_call_id.clone(),
            service: "/rosapi/topics".into(),
            args: json!({}),
        })
        .await?;
        let (topics_tx, mut topics_rx) = oneshot::channel();
        self.pending_service_calls
            .insert(topics_call_id, PendingCall::Internal(topics_tx));
        let mut topics_pending = true;

        let services_call_id = self.next_request_id();
        self.send_op(ClientOp::CallService {
            id: services_call_id.clone(),
            service: "/rosapi/services".into(),
            args: json!({}),
        })
        .await?;
        let (services_tx, mut services_rx) = oneshot::channel();
        self.pending_service_calls
            .insert(services_call_id, PendingCall::Internal(services_tx));
        let mut services_pending = true;

        let mut rediscover = Box::pin(sleep(std::time::Duration::from_secs(5)));

        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    match command {
                        Command::Subscribe { topic, schema, options, reply } => {
                            let result = self.handle_subscribe(&topic, schema, options).await;
                            let _ = reply.send(result);
                        }
                        Command::Unsubscribe { topic } => {
                            self.handle_unsubscribe(&topic).await;
                        }
                        Command::LookupTopic { topic, reply } => {
                            let _ = reply.send(self.topic_types.get(&topic).cloned());
                        }
                        Command::CacheTopic { topic, info } => {
                            self.topic_types.insert(topic, info);
                            self.refresh_discovery();
                        }
                        Command::CallService { service, request, reply } => {
                            self.dispatch_call_service(&service, request, reply).await;
                        }
                        Command::Publish { topic, payload, reply } => {
                            let result = self.send_op(ClientOp::Publish { topic, msg: payload }).await;
                            let _ = reply.send(result);
                        }
                        Command::SendActionGoal {
                            action,
                            action_type,
                            args,
                            feedback_tx,
                            result_tx,
                            token_reply,
                        } => {
                            let id_fragment = uuid_fragment();
                            let goal_id = format!("{action}::{id_fragment}");
                            self.pending_action_goals.insert(
                                goal_id.clone(),
                                PendingActionGoal { feedback_tx, result_tx },
                            );
                            let _ = token_reply.send(ActionCancelToken {
                                action: action.clone(),
                                goal_id: goal_id.clone(),
                            });
                            if let Err(err) = self
                                .send_op(ClientOp::SendActionGoal {
                                    id: goal_id.clone(),
                                    action: action.clone(),
                                    action_type,
                                    args,
                                    feedback: true,
                                })
                                .await
                            {
                                if let Some(slot) = self.pending_action_goals.remove(&goal_id) {
                                    let _ = slot.result_tx.send(Err(err));
                                }
                            }
                        }
                        Command::CancelActionGoal { action, goal_id, reply } => {
                            let composed = if goal_id.contains("::") {
                                goal_id.clone()
                            } else {
                                format!("{action}::{goal_id}")
                            };
                            let result = self
                                .send_op(ClientOp::CancelActionGoal {
                                    id: composed.clone(),
                                    action,
                                })
                                .await;
                            self.pending_action_goals.remove(&composed);
                            let _ = reply.send(result);
                        }
                        Command::Shutdown => {
                            self.fail_pending("transport disconnected");
                            self.socket.close().await;
                            return Ok(());
                        }
                    }
                }
                Some(msg) = self.socket.next() => {
                    match msg {
                        Ok(WsMsg::Text(text)) => {
                            if let Err(err) = self.handle_text(&text).await {
                                warn!(?err, "rosbridge message handling failed");
                            }
                        }
                        Ok(WsMsg::Close) => {
                            if self.try_reconnect().await.is_err() {
                                return Ok(());
                            }
                        }
                        Ok(WsMsg::Binary(_)) => {
                        }
                        Err(err) => {
                            warn!(?err, "rosbridge ws stream error; will reconnect");
                            if self.try_reconnect().await.is_err() {
                                return Ok(());
                            }
                        }
                    }
                }
                response = &mut topics_rx, if topics_pending => {
                    topics_pending = false;
                    if let Ok(value) = response {
                        self.populate_initial_discovery(value).await;
                    }
                }
                response = &mut services_rx, if services_pending => {
                    services_pending = false;
                    if let Ok(value) = response {
                        self.populate_initial_services(value).await;
                    }
                }
                _ = &mut rediscover => {
                    let topics_id = self.next_request_id();
                    if self
                        .send_op(ClientOp::CallService {
                            id: topics_id.clone(),
                            service: "/rosapi/topics".into(),
                            args: json!({}),
                        })
                        .await
                        .is_ok()
                    {
                        let (tx, rx) = oneshot::channel();
                        self.pending_service_calls.insert(topics_id, PendingCall::Internal(tx));
                        topics_rx = rx;
                        topics_pending = true;
                    }
                    let services_id = self.next_request_id();
                    if self
                        .send_op(ClientOp::CallService {
                            id: services_id.clone(),
                            service: "/rosapi/services".into(),
                            args: json!({}),
                        })
                        .await
                        .is_ok()
                    {
                        let (tx, rx) = oneshot::channel();
                        self.pending_service_calls.insert(services_id, PendingCall::Internal(tx));
                        services_rx = rx;
                        services_pending = true;
                    }
                    rediscover = Box::pin(sleep(std::time::Duration::from_secs(5)));
                }
                else => return Ok(()),
            }
        }
    }

    fn next_request_id(&mut self) -> String {
        let id = format!("rw-call-{}", self.next_call_id);
        self.next_call_id += 1;
        id
    }

    async fn send_op(&mut self, op: ClientOp) -> TransportResult<()> {
        let text = serde_json::to_string(&op)
            .map_err(|err| TransportError::Other(format!("serialise op: {err}")))?;
        self.socket
            .send(WsMsg::Text(text))
            .await
            .map_err(|err| TransportError::Other(format!("ws send: {err}")))
    }

    async fn handle_text(&mut self, text: &str) -> TransportResult<()> {
        let perf_enabled = rw_transport::perf::perf_trace_enabled();
        let ws_recv_ns = if perf_enabled {
            rw_transport::perf::now_ns()
        } else {
            0
        };
        let parsed: ServerOp = match serde_json::from_str(text) {
            Ok(parsed) => parsed,
            Err(err) => {
                debug!(?err, %text, "ignoring unparseable rosbridge json");
                return Ok(());
            }
        };
        match parsed {
            ServerOp::Publish { topic, msg } => {
                let slot = match self.subscribers.get(&topic) {
                    Some(slot) => slot,
                    None => return Ok(()),
                };
                let decode_start_ns = if perf_enabled {
                    rw_transport::perf::now_ns()
                } else {
                    0
                };
                let value = match json_to_canonical_with_schema(
                    &msg,
                    slot.schema.primary(),
                    &slot.resolver,
                ) {
                    Ok(v) => v,
                    Err(err) => {
                        warn!(?err, %topic, "rosbridge schema decode failed; falling back to schema-blind");
                        match json_to_canonical(&msg) {
                            Ok(v) => v,
                            Err(err) => {
                                warn!(?err, %topic, "rosbridge json -> canonical failed");
                                return Ok(());
                            }
                        }
                    }
                };
                let decode_end_ns = if perf_enabled {
                    rw_transport::perf::now_ns()
                } else {
                    0
                };
                let timestamp_ns = current_timestamp_ns();
                let perf = if perf_enabled {
                    Some(rw_transport::perf::PerfTrace {
                        ws_recv_ns,
                        decode_start_ns,
                        decode_end_ns,
                        pack_start_ns: 0,
                        channel_send_ns: 0,
                    })
                } else {
                    None
                };
                let frame = Frame {
                    timestamp_ns,
                    schema: slot.schema.clone(),
                    value,
                    raw: None,
                    perf,
                };
                let _ = slot.sender.try_send(frame);
            }
            ServerOp::ServiceResponse {
                id, values, result, ..
            } => {
                if let Some(pending) = self.pending_service_calls.remove(&id) {
                    match pending {
                        PendingCall::Internal(tx) => {
                            let outcome = match result {
                                Some(false) => Err(TransportError::Other(format!(
                                    "service call failed: {values}"
                                ))),
                                _ => Ok(values),
                            };
                            let _ = tx.send(outcome);
                        }
                        PendingCall::User(tx) => {
                            let outcome = match result {
                                Some(false) => Err(TransportError::Other(format!(
                                    "service call failed: {values}"
                                ))),
                                _ => json_to_canonical(&values).map_err(|err| {
                                    TransportError::Codec(format!("service response decode: {err}"))
                                }),
                            };
                            let _ = tx.send(outcome);
                        }
                    }
                }
            }
            ServerOp::ActionFeedback { id, values } => {
                if let Some(slot) = self.pending_action_goals.get(&id) {
                    match json_to_canonical(&values) {
                        Ok(value) => {
                            let _ = slot.feedback_tx.try_send(value);
                        }
                        Err(err) => warn!(?err, %id, "action feedback decode failed"),
                    }
                }
            }
            ServerOp::ActionResult {
                id, values, result, ..
            } => {
                if let Some(slot) = self.pending_action_goals.remove(&id) {
                    let outcome = match result {
                        Some(false) => Err(TransportError::Other(format!(
                            "action goal failed: {values}"
                        ))),
                        _ => json_to_canonical(&values).map_err(|err| {
                            TransportError::Codec(format!("action result decode: {err}"))
                        }),
                    };
                    let _ = slot.result_tx.send(outcome);
                }
            }
            ServerOp::Other => {}
        }
        Ok(())
    }

    async fn handle_subscribe(
        &mut self,
        topic: &str,
        schema: Option<Arc<CanonicalSchema>>,
        mut options: SubscribeOptions,
    ) -> TransportResult<Subscription> {
        let info = if let Some(schema) = schema {
            let resolver = self
                .topic_types
                .get(topic)
                .map(|t| t.resolver.clone())
                .unwrap_or_default();
            TopicInfo {
                schema_name: schema.name.clone(),
                schema,
                resolver,
            }
        } else if let Some(info) = self.topic_types.get(topic) {
            info.clone()
        } else {
            self.resolve_topic_type(topic).await?
        };

        let (sender, receiver) = mpsc::channel::<Frame>(256);
        let watcher_sender = sender.clone();
        let self_tx = self.self_tx.clone();
        let topic_owned = topic.to_string();
        spawn_detached(async move {
            watcher_sender.closed().await;
            let _ = self_tx
                .send(Command::Unsubscribe { topic: topic_owned })
                .await;
        });
        options = options.with_default_for_schema(&info.schema_name);
        self.subscribers.insert(
            topic.to_string(),
            SubscriberSlot {
                sender,
                schema: info.schema.clone(),
                resolver: info.resolver.clone(),
                options,
            },
        );
        self.send_op(ClientOp::Subscribe {
            topic: topic.into(),
            type_name: Some(info.schema_name.clone()),
            throttle_rate: options.rosbridge_throttle_ms(),
            queue_length: options.queue_length,
        })
        .await?;
        Ok(Subscription {
            frames: receiver,
            schema: info.schema,
        })
    }

    fn fail_pending(&mut self, reason: &str) {
        self.fail_pending_calls(reason);
        self.subscribers.clear();
    }

    fn fail_pending_calls(&mut self, reason: &str) {
        let calls = std::mem::take(&mut self.pending_service_calls);
        for (_id, pending) in calls {
            match pending {
                PendingCall::User(tx) => {
                    let _ = tx.send(Err(TransportError::Other(reason.to_string())));
                }
                PendingCall::Internal(tx) => {
                    let _ = tx.send(Err(TransportError::Other(reason.to_string())));
                }
            }
        }
        let goals = std::mem::take(&mut self.pending_action_goals);
        for (_goal_id, goal) in goals {
            let _ = goal
                .result_tx
                .send(Err(TransportError::Other(reason.to_string())));
            drop(goal.feedback_tx);
        }
    }

    fn reconnect_backoff_ms(attempt: u32) -> u64 {
        const BASE: u64 = 500;
        const CAP: u64 = 30_000;
        BASE.saturating_mul(1u64 << attempt.min(20)).min(CAP)
    }

    async fn try_reconnect(&mut self) -> Result<(), ()> {
        self.fail_pending_calls("rosbridge reconnecting");
        let _ = self.inner.status_tx.send(ConnectionStatus::Reconnecting);
        let mut attempt: u32 = 0;
        loop {
            let delay_ms = Self::reconnect_backoff_ms(attempt);
            tokio::select! {
                cmd = self.command_rx.recv() => {
                    match cmd {
                        Some(Command::Shutdown) | None => return Err(()),
                        Some(_) => continue,
                    }
                }
                _ = sleep(std::time::Duration::from_millis(delay_ms)) => {}
            }
            match open_socket(&self.inner.config).await {
                Ok(socket) => {
                    self.socket = socket;
                    let _ = self.inner.status_tx.send(ConnectionStatus::Connected);
                    let topics: Vec<(String, String, SubscribeOptions)> = self
                        .subscribers
                        .iter()
                        .map(|(t, s)| (t.clone(), s.schema.name.clone(), s.options))
                        .collect();
                    for (topic, type_name, options) in topics {
                        if let Err(err) = self
                            .send_op(ClientOp::Subscribe {
                                topic: topic.clone(),
                                type_name: Some(type_name),
                                throttle_rate: options.rosbridge_throttle_ms(),
                                queue_length: options.queue_length,
                            })
                            .await
                        {
                            warn!(?err, %topic, "rosbridge: re-subscribe after reconnect failed");
                        }
                    }
                    return Ok(());
                }
                Err(err) => {
                    warn!(
                        ?err,
                        attempt, "rosbridge: reconnect socket open failed; will retry"
                    );
                    attempt = attempt.saturating_add(1);
                }
            }
        }
    }

    async fn handle_unsubscribe(&mut self, topic: &str) {
        if self.subscribers.remove(topic).is_none() {
            return;
        }
        if let Err(err) = self
            .send_op(ClientOp::Unsubscribe {
                topic: topic.to_string(),
            })
            .await
        {
            warn!(?err, topic, "rosbridge unsubscribe send failed");
        }
    }

    async fn dispatch_call_service(
        &mut self,
        service: &str,
        request: CanonicalValue,
        reply: oneshot::Sender<TransportResult<CanonicalValue>>,
    ) {
        let id = self.next_request_id();
        let args = rw_codec_json::canonical_to_json(
            &request,
            matches!(self.inner.dialect(), Dialect::Ros1),
        );
        self.pending_service_calls
            .insert(id.clone(), PendingCall::User(reply));
        if let Err(err) = self
            .send_op(ClientOp::CallService {
                id: id.clone(),
                service: service.into(),
                args,
            })
            .await
        {
            if let Some(PendingCall::User(tx)) = self.pending_service_calls.remove(&id) {
                let _ = tx.send(Err(err));
            }
        }
    }

    async fn resolve_topic_type(&mut self, topic: &str) -> TransportResult<TopicInfo> {
        let request_id = self.next_request_id();
        let (tx, rx) = oneshot::channel();
        self.pending_service_calls
            .insert(request_id.clone(), PendingCall::Internal(tx));
        self.send_op(ClientOp::CallService {
            id: request_id,
            service: "/rosapi/topic_type".into(),
            args: json!({ "topic": topic }),
        })
        .await?;
        let response = timeout(Duration::from_secs(5), rx)
            .await
            .map_err(|_| TransportError::Other(format!("rosapi/topic_type {topic} timed out")))?
            .map_err(|_| TransportError::Closed)??;
        let raw_name = response
            .get("type")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                TransportError::Schema(topic.into(), "rosapi/topic_type missing 'type'".into())
            })?;
        let schema_name = normalise_schema_name(raw_name);

        let detail_id = self.next_request_id();
        let (detail_tx, detail_rx) = oneshot::channel();
        self.pending_service_calls
            .insert(detail_id.clone(), PendingCall::Internal(detail_tx));
        self.send_op(ClientOp::CallService {
            id: detail_id,
            service: "/rosapi/message_details".into(),
            args: json!({ "type": raw_name }),
        })
        .await?;
        let details = timeout(Duration::from_secs(5), detail_rx)
            .await
            .map_err(|_| TransportError::Other("rosapi/message_details timed out".into()))?
            .map_err(|_| TransportError::Closed)??;

        let schema = build_canonical_schema(&schema_name, &details, self.inner.dialect());
        let schema_arc = Arc::new(schema);
        let resolver = build_resolver(&details);
        let info = TopicInfo {
            schema_name,
            schema: schema_arc.clone(),
            resolver,
        };
        self.topic_types.insert(topic.to_string(), info.clone());
        self.refresh_discovery();
        Ok(info)
    }

    async fn populate_initial_services(&mut self, response: TransportResult<JsonValue>) {
        let response = match response {
            Ok(value) => value,
            Err(_) => return,
        };
        let services = response
            .get("services")
            .and_then(JsonValue::as_array)
            .cloned()
            .unwrap_or_default();
        for value in &services {
            let Some(name) = value.as_str() else { continue };
            if let Some(action) = name.strip_suffix("/_action/send_goal") {
                self.discovered_actions
                    .insert(action.to_string(), String::new());
            } else {
                self.discovered_services
                    .insert(name.to_string(), String::new());
            }
        }
        self.refresh_discovery();
    }

    async fn populate_initial_discovery(&mut self, response: TransportResult<JsonValue>) {
        let response = match response {
            Ok(value) => value,
            Err(_) => return,
        };
        let topics = response
            .get("topics")
            .and_then(JsonValue::as_array)
            .cloned()
            .unwrap_or_default();
        let types = response
            .get("types")
            .and_then(JsonValue::as_array)
            .cloned()
            .unwrap_or_default();
        for (topic_value, type_value) in topics.iter().zip(types.iter()) {
            if let (Some(topic), Some(type_name)) = (topic_value.as_str(), type_value.as_str()) {
                self.discovered
                    .insert(topic.to_string(), normalise_schema_name(type_name));
            }
        }
        self.refresh_discovery();
    }

    fn refresh_discovery(&self) {
        let mut by_topic: std::collections::BTreeMap<String, TopicDescriptor> =
            std::collections::BTreeMap::new();
        for (topic, schema_name) in &self.discovered {
            by_topic.insert(
                topic.clone(),
                TopicDescriptor {
                    name: topic.clone(),
                    schema_name: schema_name.clone(),
                    schema_id: None,
                    schema_definition: None,
                },
            );
        }
        for (topic, info) in &self.topic_types {
            by_topic.insert(
                topic.clone(),
                TopicDescriptor {
                    name: topic.clone(),
                    schema_name: info.schema_name.clone(),
                    schema_id: Some(info.schema.id.clone()),
                    schema_definition: None,
                },
            );
        }
        let topics: Vec<TopicDescriptor> = by_topic.into_values().collect();

        let mut by_service: std::collections::BTreeMap<String, TargetDescriptor> =
            std::collections::BTreeMap::new();
        for (name, schema_name) in &self.discovered_services {
            if name.starts_with("/rosapi/") {
                continue;
            }
            by_service.insert(
                name.clone(),
                TargetDescriptor {
                    name: name.clone(),
                    schema_name: schema_name.clone(),
                    schema_id: None,
                    schema_definition: None,
                },
            );
        }
        let services: Vec<TargetDescriptor> = by_service.into_values().collect();

        let mut by_action: std::collections::BTreeMap<String, TargetDescriptor> =
            std::collections::BTreeMap::new();
        for (name, schema_name) in &self.discovered_actions {
            by_action.insert(
                name.clone(),
                TargetDescriptor {
                    name: name.clone(),
                    schema_name: schema_name.clone(),
                    schema_id: None,
                    schema_definition: None,
                },
            );
        }
        let actions: Vec<TargetDescriptor> = by_action.into_values().collect();

        let discovery = Discovery {
            topics,
            services,
            actions,
            dialect: Some(self.inner.dialect()),
            dependency_schemas: std::collections::HashMap::new(),
        };
        let _ = self.inner.discovery_tx.send(discovery);
    }
}

fn uuid_fragment() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = rw_transport::perf::now_ns();
    format!("{n:x}-{nanos:x}")
}

fn is_empty_typedefs(value: &CanonicalValue) -> bool {
    if let CanonicalValue::Struct(fields) = value {
        if let Some(CanonicalValue::Array(items)) = fields.get("typedefs") {
            return items.is_empty();
        }
    }
    true
}

fn normalise_schema_name(name: &str) -> String {
    let parts: Vec<&str> = name.split('/').collect();
    if parts.len() == 3 && matches!(parts[1], "msg" | "srv" | "action") {
        format!("{}/{}", parts[0], parts[2])
    } else {
        name.to_string()
    }
}

fn canonical_string_field(value: &CanonicalValue, field: &str) -> Option<String> {
    if let CanonicalValue::Struct(fields) = value {
        if let Some(CanonicalValue::String(s)) = fields.get(field) {
            if !s.is_empty() {
                return Some(s.clone());
            }
        }
    }
    None
}

fn current_timestamp_ns() -> u64 {
    rw_transport::perf::now_ns()
}

fn build_resolver(details: &JsonValue) -> JsonResolver {
    let mut resolver = JsonResolver::new();
    let typedefs = details
        .get("typedefs")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default();
    for typedef in &typedefs {
        let entry_type = typedef
            .get("type")
            .and_then(JsonValue::as_str)
            .unwrap_or("");
        if entry_type.is_empty() {
            continue;
        }
        let fieldnames = typedef
            .get("fieldnames")
            .and_then(JsonValue::as_array)
            .cloned()
            .unwrap_or_default();
        let fieldtypes = typedef
            .get("fieldtypes")
            .and_then(JsonValue::as_array)
            .cloned()
            .unwrap_or_default();
        let arraylen = typedef
            .get("fieldarraylen")
            .and_then(JsonValue::as_array)
            .cloned()
            .unwrap_or_default();
        let mut def = rw_canonical::MessageDef::default();
        for ((name, ty), arr) in fieldnames
            .iter()
            .zip(fieldtypes.iter())
            .zip(arraylen.iter())
        {
            let (Some(name), Some(ty)) = (name.as_str(), ty.as_str()) else {
                continue;
            };
            let len = arr.as_i64().unwrap_or(-1);
            let field_type = rosapi_type_to_canonical(ty, len);
            def.fields.push(rw_canonical::FieldDef {
                name: name.into(),
                field_type,
                default: None,
                comment: None,
            });
        }
        resolver.insert(normalise_schema_name(entry_type), def);
    }
    resolver
}

#[cfg(test)]
fn stub_schema(name: &str, dialect: Dialect) -> CanonicalSchema {
    let id = canonical_schema_id(name);
    let viz_role = rw_canonical::viz_role_for_schema(name);
    CanonicalSchema {
        id,
        name: name.into(),
        kind: SchemaKind::Message,
        dialect,
        definition: String::new(),
        parsed: ParsedSchema::Message(MessageDef::default()),
        dependencies: Vec::new(),
        viz_role,
    }
}

fn build_canonical_schema(
    schema_name: &str,
    details: &JsonValue,
    dialect: Dialect,
) -> CanonicalSchema {
    let viz_role = rw_canonical::viz_role_for_schema(schema_name);
    let typedefs = details
        .get("typedefs")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default();
    let mut dependencies: Vec<SchemaId> = Vec::new();
    let mut root_def: Option<MessageDef> = None;
    let mut canonical_text = String::new();

    for typedef in &typedefs {
        let entry_type_raw = typedef
            .get("type")
            .and_then(JsonValue::as_str)
            .unwrap_or("");
        let entry_type = normalise_schema_name(entry_type_raw);
        let fieldnames = typedef
            .get("fieldnames")
            .and_then(JsonValue::as_array)
            .cloned()
            .unwrap_or_default();
        let fieldtypes = typedef
            .get("fieldtypes")
            .and_then(JsonValue::as_array)
            .cloned()
            .unwrap_or_default();
        let arraylen = typedef
            .get("fieldarraylen")
            .and_then(JsonValue::as_array)
            .cloned()
            .unwrap_or_default();

        let mut def = MessageDef::default();
        let mut text = String::new();
        for ((name, ty), arr) in fieldnames
            .iter()
            .zip(fieldtypes.iter())
            .zip(arraylen.iter())
        {
            let (Some(name), Some(ty)) = (name.as_str(), ty.as_str()) else {
                continue;
            };
            let len = arr.as_i64().unwrap_or(-1);
            let field_type = rosapi_type_to_canonical(ty, len);
            def.fields.push(rw_canonical::FieldDef {
                name: name.into(),
                field_type,
                default: None,
                comment: None,
            });
            text.push_str(&format!("{} {}\n", ty, name));
        }
        if entry_type == schema_name {
            root_def = Some(def);
            canonical_text = text;
        } else {
            dependencies.push(canonical_schema_id(&text));
        }
    }
    let id = canonical_schema_id(&canonical_text);
    CanonicalSchema {
        id,
        name: schema_name.into(),
        kind: SchemaKind::Message,
        dialect,
        definition: canonical_text,
        parsed: ParsedSchema::Message(root_def.unwrap_or_default()),
        dependencies,
        viz_role,
    }
}

fn rosapi_type_to_canonical(ty: &str, array_len: i64) -> rw_canonical::FieldType {
    let (base, suffix) = match ty.rfind('[') {
        Some(open) if ty.ends_with(']') => (&ty[..open], &ty[open + 1..ty.len() - 1]),
        _ => (ty, ""),
    };
    let inner = parse_base_to_field_type(base);
    let is_dynamic = ty.ends_with("[]") || array_len == 0;
    let fixed = if let Ok(n) = suffix.parse::<usize>() {
        Some(n)
    } else if array_len > 0 {
        Some(array_len as usize)
    } else {
        None
    };
    if let Some(n) = fixed {
        rw_canonical::FieldType::Array {
            element: Box::new(inner),
            length: rw_canonical::ArrayLength::Fixed(n),
        }
    } else if is_dynamic {
        rw_canonical::FieldType::Array {
            element: Box::new(inner),
            length: rw_canonical::ArrayLength::Unbounded,
        }
    } else {
        inner
    }
}

fn parse_base_to_field_type(token: &str) -> rw_canonical::FieldType {
    if let Some(primitive) = rw_canonical::PrimitiveType::parse(token) {
        return rw_canonical::FieldType::Primitive(primitive);
    }
    let dds = match token {
        "float" => Some(rw_canonical::PrimitiveType::Float32),
        "double" => Some(rw_canonical::PrimitiveType::Float64),
        "boolean" => Some(rw_canonical::PrimitiveType::Bool),
        "octet" => Some(rw_canonical::PrimitiveType::Uint8),
        "long" => Some(rw_canonical::PrimitiveType::Int32),
        "unsigned long" => Some(rw_canonical::PrimitiveType::Uint32),
        "long long" => Some(rw_canonical::PrimitiveType::Int64),
        "unsigned long long" => Some(rw_canonical::PrimitiveType::Uint64),
        "short" => Some(rw_canonical::PrimitiveType::Int16),
        "unsigned short" => Some(rw_canonical::PrimitiveType::Uint16),
        _ => None,
    };
    if let Some(primitive) = dds {
        return rw_canonical::FieldType::Primitive(primitive);
    }
    if token == "string" {
        return rw_canonical::FieldType::String { bound: None };
    }
    if token == "time" {
        return rw_canonical::FieldType::Time;
    }
    if token == "duration" {
        return rw_canonical::FieldType::Duration;
    }
    if token.contains('/') {
        let parts: Vec<&str> = token.split('/').collect();
        let normalized = if parts.len() == 3 && matches!(parts[1], "msg" | "srv" | "action") {
            format!("{}/{}", parts[0], parts[2])
        } else {
            token.to_string()
        };
        if normalized == "builtin_interfaces/Time" {
            return rw_canonical::FieldType::Time;
        }
        if normalized == "builtin_interfaces/Duration" {
            return rw_canonical::FieldType::Duration;
        }
        return rw_canonical::FieldType::Complex {
            type_name: normalized,
        };
    }
    rw_canonical::FieldType::Complex {
        type_name: token.into(),
    }
}

#[doc(hidden)]
#[allow(dead_code)]
fn _link_anchor(_role: VisualizationRole) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn topics_response(types: &[&str]) -> CanonicalValue {
        CanonicalValue::Struct(std::collections::BTreeMap::from([(
            "types".to_string(),
            CanonicalValue::Array(
                types
                    .iter()
                    .map(|name| CanonicalValue::String(name.to_string()))
                    .collect(),
            ),
        )]))
    }

    #[test]
    fn dialect_detects_ros1_from_two_segment_types() {
        assert_eq!(
            dialect_from_topic_types(&topics_response(&["std_msgs/Int32", "rosgraph_msgs/Log"])),
            Dialect::Ros1
        );
    }

    #[test]
    fn dialect_detects_ros2_from_three_segment_types() {
        assert_eq!(
            dialect_from_topic_types(&topics_response(&["std_msgs/msg/Int32"])),
            Dialect::Ros2
        );
    }

    #[test]
    fn dialect_defaults_to_ros2_when_no_types() {
        assert_eq!(
            dialect_from_topic_types(&CanonicalValue::Struct(Default::default())),
            Dialect::Ros2
        );
    }

    #[test]
    fn rosapi_type_to_canonical_handles_primitives() {
        match rosapi_type_to_canonical("int32", -1) {
            rw_canonical::FieldType::Primitive(rw_canonical::PrimitiveType::Int32) => {}
            other => panic!("expected int32, got {other:?}"),
        }
    }

    #[test]
    fn rosapi_type_to_canonical_handles_unbounded_array() {
        match rosapi_type_to_canonical("float32[]", -1) {
            rw_canonical::FieldType::Array {
                length: rw_canonical::ArrayLength::Unbounded,
                ..
            } => {}
            other => panic!("expected unbounded array, got {other:?}"),
        }
    }

    #[test]
    fn rosapi_type_to_canonical_handles_fixed_array() {
        match rosapi_type_to_canonical("float64[16]", -1) {
            rw_canonical::FieldType::Array {
                length: rw_canonical::ArrayLength::Fixed(16),
                ..
            } => {}
            other => panic!("expected fixed array, got {other:?}"),
        }
    }

    #[test]
    fn rosapi_type_to_canonical_handles_complex() {
        match rosapi_type_to_canonical("std_msgs/Header", -1) {
            rw_canonical::FieldType::Complex { type_name } => {
                assert_eq!(type_name, "std_msgs/Header")
            }
            other => panic!("expected complex, got {other:?}"),
        }
    }

    #[test]
    fn ros2_msg_segment_is_canonicalised_to_short_form() {
        match rosapi_type_to_canonical("geometry_msgs/msg/Point", -1) {
            rw_canonical::FieldType::Complex { type_name } => {
                assert_eq!(type_name, "geometry_msgs/Point");
            }
            other => panic!("expected complex, got {other:?}"),
        }
    }

    #[test]
    fn stub_schema_is_deterministic() {
        let a = stub_schema("std_msgs/String", Dialect::Ros2);
        let b = stub_schema("std_msgs/String", Dialect::Ros2);
        assert_eq!(a.id, b.id);
        assert_eq!(a.viz_role, rw_canonical::VisualizationRole::Text);
    }

    #[test]
    fn build_resolver_indexes_typedefs_by_canonical_name() {
        let details = serde_json::json!({
            "typedefs": [
                {
                    "type": "geometry_msgs/Point",
                    "fieldnames": ["x", "y", "z"],
                    "fieldtypes": ["float64", "float64", "float64"],
                    "fieldarraylen": [-1, -1, -1],
                },
                {
                    "type": "geometry_msgs/msg/Quaternion",
                    "fieldnames": ["x", "y", "z", "w"],
                    "fieldtypes": ["float64", "float64", "float64", "float64"],
                    "fieldarraylen": [-1, -1, -1, -1],
                }
            ]
        });
        let resolver = build_resolver(&details);
        assert!(resolver.contains_key("geometry_msgs/Point"));
        assert!(resolver.contains_key("geometry_msgs/Quaternion"));
        let point = &resolver["geometry_msgs/Point"];
        assert_eq!(point.fields.len(), 3);
    }
}
