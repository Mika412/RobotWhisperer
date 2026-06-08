#![deny(missing_debug_implementations)]

mod wire;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use rw_canonical::{CanonicalSchema, CanonicalValue, Dialect, MessageDef, SchemaId, SchemaKind};
use rw_codec_cdr::decode_message as decode_cdr_message;
use rw_codec_rosmsg::decode_message as decode_rosmsg_message;
use rw_schema_foxglove::{
    parse_concatenated_with_resolver, split_concatenated, ActionDefinitionParts,
};
use rw_transport::action::{self, ActionGoalId, ActionTargets};
use rw_transport::{
    ActionCancelToken, ActionGoalStream, ConnectionStatus, Discovery, Frame, SubscribeOptions,
    Subscription, TargetDescriptor, TopicDescriptor, Transport, TransportError, TransportResult,
};
use rw_ws::{self as ws, WsMsg};
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use tracing::{debug, error, warn};

use rw_transport::task::{spawn_detached, spawn_task, SpawnedTask};
use rw_transport::time::sleep;

use wire::{
    parse_binary_frame, AdvertisedChannel, ClientMessage, ServerMessage, SubscriptionRequest,
    SUBPROTOCOLS,
};

#[derive(Debug, Clone)]
pub struct FoxgloveConfig {
    pub url: String,
    pub connect_timeout: Duration,
}

impl FoxgloveConfig {
    pub fn new(url: impl Into<String>) -> Self {
        FoxgloveConfig {
            url: url.into(),
            connect_timeout: Duration::from_secs(15),
        }
    }
}

fn dialect_from_subprotocol(subprotocol: Option<&str>) -> Dialect {
    match subprotocol {
        Some(wire::SUBPROTOCOL_ROS1) => Dialect::Ros1,
        _ => Dialect::Ros2,
    }
}

fn reconnect_backoff_ms(attempt: u32) -> u64 {
    const BASE: u64 = 500;
    const CAP: u64 = 30_000;
    BASE.saturating_mul(1u64 << attempt.min(20)).min(CAP)
}

fn member_or_whole(value: CanonicalValue, member: &str) -> CanonicalValue {
    match value {
        CanonicalValue::Struct(mut fields) => match fields.remove(member) {
            Some(inner) => inner,
            None => CanonicalValue::Struct(fields),
        },
        other => other,
    }
}

enum RunOutcome {
    Shutdown,
    Disconnected,
}

#[derive(Debug, Clone)]
pub struct FoxgloveTransport {
    inner: Arc<TransportInner>,
}

#[derive(Debug)]
struct TransportInner {
    config: FoxgloveConfig,
    status_tx: watch::Sender<ConnectionStatus>,
    status_rx: watch::Receiver<ConnectionStatus>,
    discovery_tx: watch::Sender<Discovery>,
    discovery_rx: watch::Receiver<Discovery>,
    command_tx: Mutex<Option<mpsc::Sender<Command>>>,
    actor_handle: Mutex<Option<SpawnedTask>>,
}

#[derive(Debug)]
enum Command {
    Subscribe {
        topic: String,
        options: SubscribeOptions,
        reply: oneshot::Sender<TransportResult<Subscription>>,
    },
    Unsubscribe {
        subscription_id: u32,
    },
    CallService {
        service: String,
        request: CanonicalValue,
        reply: oneshot::Sender<TransportResult<CanonicalValue>>,
    },
    SendActionGoal {
        action: String,
        goal: CanonicalValue,
        feedback_tx: mpsc::Sender<CanonicalValue>,
        result_tx: oneshot::Sender<TransportResult<CanonicalValue>>,
        token_reply: oneshot::Sender<ActionCancelToken>,
    },
    CancelActionGoal {
        token: ActionCancelToken,
        reply: oneshot::Sender<TransportResult<CanonicalValue>>,
    },
    Shutdown,
}

impl FoxgloveTransport {
    pub fn new(config: FoxgloveConfig) -> Self {
        let (status_tx, status_rx) = watch::channel(ConnectionStatus::Disconnected);
        let (discovery_tx, discovery_rx) = watch::channel(Discovery::default());
        FoxgloveTransport {
            inner: Arc::new(TransportInner {
                config,
                status_tx,
                status_rx,
                discovery_tx,
                discovery_rx,
                command_tx: Mutex::new(None),
                actor_handle: Mutex::new(None),
            }),
        }
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl Transport for FoxgloveTransport {
    async fn connect(&self) -> TransportResult<()> {
        let mut command_slot = self.inner.command_tx.lock().await;
        if command_slot.is_some() {
            return Ok(());
        }
        let (command_tx, command_rx) = mpsc::channel::<Command>(64);
        let inner = self.inner.clone();
        let _ = inner.status_tx.send(ConnectionStatus::Connecting);
        let socket = open_socket(&inner.config).await?;
        let _ = inner.status_tx.send(ConnectionStatus::Connected);
        let actor = Actor::spawn(inner.clone(), socket, command_rx, command_tx.clone());
        *command_slot = Some(command_tx);
        let mut handle_slot = inner.actor_handle.lock().await;
        *handle_slot = Some(actor);
        Ok(())
    }

    async fn disconnect(&self) -> TransportResult<()> {
        let command_tx = self.inner.command_tx.lock().await.take();
        if let Some(tx) = command_tx {
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
        let command_tx = {
            let guard = self.inner.command_tx.lock().await;
            guard.clone().ok_or(TransportError::NotConnected)?
        };
        let (reply_tx, reply_rx) = oneshot::channel();
        command_tx
            .send(Command::Subscribe {
                topic: topic.to_string(),
                options,
                reply: reply_tx,
            })
            .await
            .map_err(|_| TransportError::Closed)?;
        reply_rx.await.map_err(|_| TransportError::Closed)?
    }

    async fn publish(&self, _topic: &str, _value: CanonicalValue) -> TransportResult<()> {
        Err(TransportError::Other(
            "foxglove ws publish: not yet supported".into(),
        ))
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
        let command_tx = {
            let guard = self.inner.command_tx.lock().await;
            guard.clone().ok_or(TransportError::NotConnected)?
        };
        let (feedback_tx, feedback_rx) = mpsc::channel::<CanonicalValue>(64);
        let (result_tx, result_rx) = oneshot::channel();
        let (token_tx, token_rx) = oneshot::channel();
        command_tx
            .send(Command::SendActionGoal {
                action: action.to_string(),
                goal,
                feedback_tx,
                result_tx,
                token_reply: token_tx,
            })
            .await
            .map_err(|_| TransportError::Closed)?;
        let cancel_token = token_rx.await.map_err(|_| TransportError::Closed)?;
        Ok(ActionGoalStream {
            feedback: feedback_rx,
            result: result_rx,
            cancel_token,
        })
    }

    async fn cancel_action_goal(&self, token: &ActionCancelToken) -> TransportResult<()> {
        let command_tx = {
            let guard = self.inner.command_tx.lock().await;
            guard.clone().ok_or(TransportError::NotConnected)?
        };
        let (reply_tx, reply_rx) = oneshot::channel();
        command_tx
            .send(Command::CancelActionGoal {
                token: token.clone(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| TransportError::Closed)?;
        reply_rx
            .await
            .map_err(|_| TransportError::Closed)?
            .map(|_| ())
    }
}

type Socket = ws::WsConnection;

async fn open_socket(config: &FoxgloveConfig) -> TransportResult<Socket> {
    ws::connect(&config.url, config.connect_timeout, SUBPROTOCOLS)
        .await
        .map_err(|err| TransportError::Other(err.to_string()))
}

#[derive(Debug, Clone)]
struct ServiceEntry {
    id: u32,
    name: String,
    canonical_type: String,
    encoding: String,
    request_schema: Arc<CanonicalSchema>,
    response_schema: Arc<CanonicalSchema>,
    request_resolver: HashMap<String, MessageDef>,
    response_resolver: HashMap<String, MessageDef>,
}

#[derive(Debug)]
struct PendingServiceCall {
    response_schema: Arc<CanonicalSchema>,
    response_resolver: HashMap<String, MessageDef>,
    encoding: String,
    sink: ServiceResponseSink,
}

#[derive(Debug)]
enum ServiceResponseSink {
    User(oneshot::Sender<TransportResult<CanonicalValue>>),
    ActionAccepted(ActionGoalId),
    ActionResult(ActionGoalId),
}

#[derive(Debug)]
struct PendingActionGoal {
    action: String,
    goal_id: ActionGoalId,
    feedback_tx: mpsc::Sender<CanonicalValue>,
    result_tx: oneshot::Sender<TransportResult<CanonicalValue>>,
}

struct Actor {
    inner: Arc<TransportInner>,
    socket: Socket,
    dialect: Dialect,
    command_rx: mpsc::Receiver<Command>,
    self_tx: mpsc::Sender<Command>,
    channels_by_id: HashMap<u32, ChannelEntry>,
    channels_by_topic: HashMap<String, u32>,
    services_by_id: HashMap<u32, ServiceEntry>,
    services_by_name: HashMap<String, u32>,
    pending_service_calls: HashMap<u32, PendingServiceCall>,
    next_call_id: u32,
    schemas_by_id: HashMap<SchemaId, Arc<CanonicalSchema>>,
    resolvers: HashMap<SchemaId, HashMap<String, MessageDef>>,
    topic_schema_text: HashMap<String, String>,
    service_schema_text: HashMap<String, String>,
    dependency_schemas: HashMap<String, String>,
    subscribers: HashMap<u32, SubscriberSlot>,
    next_subscription_id: u32,
    awaiting_resubscribe: HashSet<u32>,
    action_definitions: HashMap<String, ActionDefinitionParts>,
    pending_action_goals: HashMap<ActionGoalId, PendingActionGoal>,
    action_feedback_subs: HashMap<String, u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChannelEncoding {
    Cdr,
    Ros1,
}

impl ChannelEncoding {
    fn parse(raw: &str) -> Self {
        match raw {
            "ros1" | "ros1msg" => ChannelEncoding::Ros1,
            _ => ChannelEncoding::Cdr,
        }
    }
}

#[derive(Debug, Clone)]
struct ChannelEntry {
    topic: String,
    schema_id: SchemaId,
    encoding: ChannelEncoding,
}

#[derive(Debug)]
enum SlotSink {
    Topic(mpsc::Sender<Frame>),
    ActionFeedback,
}

#[derive(Debug)]
struct SubscriberSlot {
    topic: String,
    sink: SlotSink,
    schema: Arc<CanonicalSchema>,
    resolver: HashMap<String, MessageDef>,
    encoding: ChannelEncoding,
    min_interval_ns: Option<u64>,
    last_emitted_ns: u64,
}

impl Actor {
    fn spawn(
        inner: Arc<TransportInner>,
        socket: Socket,
        command_rx: mpsc::Receiver<Command>,
        self_tx: mpsc::Sender<Command>,
    ) -> SpawnedTask {
        let dialect = dialect_from_subprotocol(socket.selected_subprotocol());
        let actor = Actor {
            inner,
            socket,
            dialect,
            command_rx,
            self_tx,
            channels_by_id: HashMap::new(),
            channels_by_topic: HashMap::new(),
            services_by_id: HashMap::new(),
            services_by_name: HashMap::new(),
            pending_service_calls: HashMap::new(),
            next_call_id: 1,
            schemas_by_id: HashMap::new(),
            resolvers: HashMap::new(),
            topic_schema_text: HashMap::new(),
            service_schema_text: HashMap::new(),
            dependency_schemas: HashMap::new(),
            subscribers: HashMap::new(),
            next_subscription_id: 1,
            awaiting_resubscribe: HashSet::new(),
            action_definitions: HashMap::new(),
            pending_action_goals: HashMap::new(),
            action_feedback_subs: HashMap::new(),
        };
        spawn_task(async move {
            if let Err(err) = actor.run().await {
                error!(?err, "foxglove transport actor exited with error");
            }
        })
    }

    fn fail_pending(&mut self, reason: &str) {
        let pending = std::mem::take(&mut self.pending_service_calls);
        for (_id, pending) in pending {
            if let ServiceResponseSink::User(reply) = pending.sink {
                let _ = reply.send(Err(TransportError::Other(reason.to_string())));
            }
        }
        let goals = std::mem::take(&mut self.pending_action_goals);
        for (_goal_id, goal) in goals {
            let _ = goal
                .result_tx
                .send(Err(TransportError::Other(reason.to_string())));
        }
        self.action_feedback_subs.clear();
        self.subscribers.clear();
    }

    async fn run(mut self) -> TransportResult<()> {
        loop {
            match self.run_connection().await {
                RunOutcome::Shutdown => return Ok(()),
                RunOutcome::Disconnected => {
                    if self.try_reconnect().await.is_err() {
                        return Ok(());
                    }
                }
            }
        }
    }

    async fn run_connection(&mut self) -> RunOutcome {
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    match command {
                        Command::Subscribe { topic, options, reply } => {
                            let result = self.handle_subscribe(&topic, options).await;
                            let _ = reply.send(result);
                        }
                        Command::Unsubscribe { subscription_id } => {
                            self.handle_unsubscribe(subscription_id).await;
                        }
                        Command::CallService { service, request, reply } => {
                            self.send_service_request(
                                &service,
                                request,
                                ServiceResponseSink::User(reply),
                            )
                            .await;
                        }
                        Command::SendActionGoal {
                            action,
                            goal,
                            feedback_tx,
                            result_tx,
                            token_reply,
                        } => {
                            self.start_action_goal(action, goal, feedback_tx, result_tx, token_reply)
                                .await;
                        }
                        Command::CancelActionGoal { token, reply } => {
                            self.cancel_action_goal(token, reply).await;
                        }
                        Command::Shutdown => {
                            self.fail_pending("transport disconnected");
                            self.socket.close().await;
                            return RunOutcome::Shutdown;
                        }
                    }
                }
                Some(message) = self.socket.next() => {
                    match message {
                        Ok(WsMsg::Text(text)) => self.handle_text(&text).await,
                        Ok(WsMsg::Binary(bytes)) => self.handle_binary(&bytes).await,
                        Ok(WsMsg::Close) => {
                            self.fail_pending("foxglove ws closed by server");
                            return RunOutcome::Disconnected;
                        }
                        Err(err) => {
                            warn!(?err, "foxglove ws stream error; will reconnect");
                            self.fail_pending(&format!("foxglove ws stream error: {err}"));
                            return RunOutcome::Disconnected;
                        }
                    }
                }
                else => return RunOutcome::Disconnected,
            }
        }
    }

    async fn try_reconnect(&mut self) -> Result<(), ()> {
        self.fail_pending("foxglove reconnecting");
        let _ = self.inner.status_tx.send(ConnectionStatus::Reconnecting);
        let mut attempt: u32 = 0;
        loop {
            let delay_ms = reconnect_backoff_ms(attempt);
            tokio::select! {
                command = self.command_rx.recv() => {
                    match command {
                        Some(Command::Shutdown) | None => return Err(()),
                        Some(_) => continue,
                    }
                }
                _ = sleep(Duration::from_millis(delay_ms)) => {}
            }
            match open_socket(&self.inner.config).await {
                Ok(socket) => {
                    self.dialect = dialect_from_subprotocol(socket.selected_subprotocol());
                    self.socket = socket;
                    self.channels_by_id.clear();
                    self.channels_by_topic.clear();
                    self.schemas_by_id.clear();
                    self.resolvers.clear();
                    self.services_by_id.clear();
                    self.services_by_name.clear();
                    self.action_definitions.clear();
                    self.awaiting_resubscribe = self.subscribers.keys().copied().collect();
                    let _ = self.inner.status_tx.send(ConnectionStatus::Connected);
                    return Ok(());
                }
                Err(err) => {
                    warn!(
                        ?err,
                        attempt, "foxglove reconnect socket open failed; will retry"
                    );
                    attempt = attempt.saturating_add(1);
                }
            }
        }
    }

    async fn handle_text(&mut self, text: &str) {
        let message: ServerMessage = match serde_json::from_str(text) {
            Ok(message) => message,
            Err(err) => {
                debug!(?err, "ignoring unparseable foxglove ws json");
                return;
            }
        };
        match message {
            ServerMessage::ServerInfo { name, .. } => {
                debug!(server = %name, "foxglove server info");
            }
            ServerMessage::Advertise { channels } => {
                for channel in channels {
                    self.register_channel(channel).await;
                }
                self.publish_discovery();
            }
            ServerMessage::Unadvertise { channel_ids } => {
                for id in channel_ids {
                    if let Some(entry) = self.channels_by_id.remove(&id) {
                        self.channels_by_topic.remove(&entry.topic);
                    }
                }
                self.publish_discovery();
            }
            ServerMessage::AdvertiseServices { services } => {
                for service in services {
                    self.register_service(service.normalised());
                }
                self.publish_discovery();
            }
            ServerMessage::UnadvertiseServices { service_ids } => {
                for id in service_ids {
                    if let Some(entry) = self.services_by_id.remove(&id) {
                        self.services_by_name.remove(&entry.name);
                    }
                }
                self.publish_discovery();
            }
            ServerMessage::Status { level, message } => {
                debug!(level, %message, "foxglove server status");
            }
            ServerMessage::Unknown => {}
        }
    }

    async fn handle_binary(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        match bytes[0] {
            wire::OPCODE_MESSAGE_DATA => self.handle_message_data(bytes),
            wire::OPCODE_SERVICE_CALL_RESPONSE => self.handle_service_response(bytes).await,
            wire::OPCODE_TIME => {}
            other => {
                debug!(opcode = other, "unhandled foxglove binary opcode");
            }
        }
    }

    fn handle_message_data(&mut self, bytes: &[u8]) {
        let perf_enabled = rw_transport::perf::perf_trace_enabled();
        let ws_recv_ns = if perf_enabled {
            rw_transport::perf::now_ns()
        } else {
            0
        };
        let frame = match parse_binary_frame(bytes) {
            Ok(frame) => frame,
            Err(err) => {
                warn!(?err, "foxglove binary frame parse failed");
                return;
            }
        };
        let slot = match self.subscribers.get_mut(&frame.subscription_id) {
            Some(slot) => slot,
            None => return,
        };
        if let Some(min_interval) = slot.min_interval_ns {
            let delta = frame.timestamp_ns.saturating_sub(slot.last_emitted_ns);
            if slot.last_emitted_ns != 0 && delta < min_interval {
                return;
            }
            slot.last_emitted_ns = frame.timestamp_ns;
        }
        let payload: &[u8] = frame.payload.as_ref();
        let decode_start_ns = if perf_enabled {
            rw_transport::perf::now_ns()
        } else {
            0
        };
        let value = match slot.encoding {
            ChannelEncoding::Cdr => {
                match decode_cdr_message(payload, slot.schema.primary(), &slot.resolver) {
                    Ok(value) => value,
                    Err(err) => {
                        warn!(?err, schema=%slot.schema.name, "cdr decode failed");
                        return;
                    }
                }
            }
            ChannelEncoding::Ros1 => {
                let rosmsg_resolver: rw_codec_rosmsg::Resolver = slot.resolver.clone();
                match decode_rosmsg_message(payload, slot.schema.primary(), &rosmsg_resolver) {
                    Ok(value) => value,
                    Err(err) => {
                        warn!(?err, schema=%slot.schema.name, "ros1 wire decode failed");
                        return;
                    }
                }
            }
        };
        let decode_end_ns = if perf_enabled {
            rw_transport::perf::now_ns()
        } else {
            0
        };
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
        let feedback_value = match &slot.sink {
            SlotSink::Topic(sender) => {
                let envelope = Frame {
                    timestamp_ns: frame.timestamp_ns,
                    schema: slot.schema.clone(),
                    value,
                    raw: Some(Arc::<[u8]>::from(payload)),
                    perf,
                };
                let _ = sender.try_send(envelope);
                None
            }
            SlotSink::ActionFeedback => Some(value),
        };
        if let Some(value) = feedback_value {
            self.route_action_feedback(value);
        }
    }

    fn route_action_feedback(&self, value: CanonicalValue) {
        for goal in self.pending_action_goals.values() {
            if action::goal_uuid_matches(&value, &goal.goal_id) {
                let _ = goal
                    .feedback_tx
                    .try_send(member_or_whole(value, "feedback"));
                return;
            }
        }
    }

    async fn handle_service_response(&mut self, bytes: &[u8]) {
        let response = match wire::parse_service_call_response(bytes) {
            Ok(r) => r,
            Err(err) => {
                warn!(?err, "foxglove service response parse failed");
                return;
            }
        };
        let pending = match self.pending_service_calls.remove(&response.call_id) {
            Some(p) => p,
            None => {
                debug!(call_id = response.call_id, "no pending call for response");
                return;
            }
        };
        let encoding = if response.encoding.is_empty() {
            pending.encoding.as_str()
        } else {
            response.encoding.as_str()
        };
        let outcome = match encoding {
            "" | "cdr" => decode_cdr_message(
                response.payload.as_ref(),
                pending.response_schema.primary(),
                &pending.response_resolver,
            )
            .map_err(|err| TransportError::Codec(format!("service response cdr decode: {err}"))),
            "ros1" | "ros1msg" => {
                let resolver: rw_codec_rosmsg::Resolver = pending.response_resolver.clone();
                decode_rosmsg_message(
                    response.payload.as_ref(),
                    pending.response_schema.primary(),
                    &resolver,
                )
                .map_err(|err| {
                    TransportError::Codec(format!("service response ros1 decode: {err}"))
                })
            }
            other => Err(TransportError::Codec(format!(
                "unsupported service response encoding '{other}'"
            ))),
        };
        self.deliver_service_outcome(pending.sink, outcome).await;
    }

    async fn deliver_service_outcome(
        &mut self,
        sink: ServiceResponseSink,
        outcome: TransportResult<CanonicalValue>,
    ) {
        match sink {
            ServiceResponseSink::User(reply) => {
                let _ = reply.send(outcome);
            }
            ServiceResponseSink::ActionAccepted(goal_id) => {
                self.on_send_goal_acknowledged(goal_id, outcome).await;
            }
            ServiceResponseSink::ActionResult(goal_id) => {
                self.finish_goal(goal_id, outcome).await;
            }
        }
    }

    async fn send_service_request(
        &mut self,
        service: &str,
        request: CanonicalValue,
        sink: ServiceResponseSink,
    ) {
        let entry = match self
            .services_by_name
            .get(service)
            .copied()
            .and_then(|service_id| self.services_by_id.get(&service_id).cloned())
        {
            Some(entry) => entry,
            None => {
                self.deliver_service_outcome(
                    sink,
                    Err(TransportError::UnknownService(service.into())),
                )
                .await;
                return;
            }
        };
        let payload = match rw_codec_cdr::encode_message(
            &request,
            entry.request_schema.primary(),
            &entry.request_resolver,
        ) {
            Ok(payload) => payload,
            Err(err) => {
                self.deliver_service_outcome(
                    sink,
                    Err(TransportError::Codec(format!(
                        "service request cdr encode: {err}"
                    ))),
                )
                .await;
                return;
            }
        };

        let call_id = self.next_call_id;
        self.next_call_id = self.next_call_id.wrapping_add(1).max(1);
        debug!(
            service = %service,
            call_id,
            encoding = %entry.encoding,
            payload_len = payload.len(),
            "foxglove service request dispatched",
        );
        let frame = wire::pack_service_call_request(entry.id, call_id, &entry.encoding, &payload);
        self.pending_service_calls.insert(
            call_id,
            PendingServiceCall {
                response_schema: entry.response_schema,
                response_resolver: entry.response_resolver,
                encoding: entry.encoding.clone(),
                sink,
            },
        );
        if let Err(err) = self.socket.send(WsMsg::Binary(frame)).await {
            if let Some(pending) = self.pending_service_calls.remove(&call_id) {
                self.deliver_service_outcome(
                    pending.sink,
                    Err(TransportError::Other(format!("ws send service: {err}"))),
                )
                .await;
            }
        }
    }

    async fn start_action_goal(
        &mut self,
        action: String,
        goal: CanonicalValue,
        feedback_tx: mpsc::Sender<CanonicalValue>,
        result_tx: oneshot::Sender<TransportResult<CanonicalValue>>,
        token_reply: oneshot::Sender<ActionCancelToken>,
    ) {
        let goal_id = ActionGoalId::new_v4();
        let _ = token_reply.send(ActionCancelToken {
            action: action.clone(),
            goal_id: goal_id.to_hex(),
        });
        self.pending_action_goals.insert(
            goal_id,
            PendingActionGoal {
                action: action.clone(),
                goal_id,
                feedback_tx,
                result_tx,
            },
        );
        let targets = ActionTargets::for_action(&action);
        self.ensure_feedback_subscription(&targets.feedback).await;
        let request = match self.shape_send_goal(&action, &targets.send_goal, &goal_id, goal) {
            Ok(request) => request,
            Err(err) => {
                self.finish_goal(goal_id, Err(err)).await;
                return;
            }
        };
        self.send_service_request(
            &targets.send_goal,
            request,
            ServiceResponseSink::ActionAccepted(goal_id),
        )
        .await;
    }

    fn shape_send_goal(
        &self,
        action: &str,
        send_goal_service: &str,
        goal_id: &ActionGoalId,
        goal: CanonicalValue,
    ) -> TransportResult<CanonicalValue> {
        let entry = self
            .services_by_name
            .get(send_goal_service)
            .copied()
            .and_then(|service_id| self.services_by_id.get(&service_id))
            .ok_or_else(|| TransportError::UnknownAction(action.to_string()))?;
        Ok(action::shape_send_goal_request(
            entry.request_schema.primary(),
            goal_id,
            goal,
        ))
    }

    async fn on_send_goal_acknowledged(
        &mut self,
        goal_id: ActionGoalId,
        outcome: TransportResult<CanonicalValue>,
    ) {
        let action = match self.pending_action_goals.get(&goal_id) {
            Some(goal) => goal.action.clone(),
            None => return,
        };
        let acknowledgement = match outcome {
            Ok(acknowledgement) => acknowledgement,
            Err(err) => {
                self.finish_goal(goal_id, Err(err)).await;
                return;
            }
        };
        if !action::goal_accepted(&acknowledgement) {
            self.finish_goal(
                goal_id,
                Err(TransportError::Other(format!(
                    "action goal rejected by server: {action}"
                ))),
            )
            .await;
            return;
        }
        let targets = ActionTargets::for_action(&action);
        let request = action::goal_result_request(&goal_id);
        Box::pin(self.send_service_request(
            &targets.get_result,
            request,
            ServiceResponseSink::ActionResult(goal_id),
        ))
        .await;
    }

    async fn finish_goal(
        &mut self,
        goal_id: ActionGoalId,
        outcome: TransportResult<CanonicalValue>,
    ) {
        let Some(goal) = self.pending_action_goals.remove(&goal_id) else {
            return;
        };
        let value = outcome.map(|response| member_or_whole(response, "result"));
        let _ = goal.result_tx.send(value);
        self.release_feedback_subscription(&goal.action).await;
    }

    async fn cancel_action_goal(
        &mut self,
        token: ActionCancelToken,
        reply: oneshot::Sender<TransportResult<CanonicalValue>>,
    ) {
        let goal_id = match ActionGoalId::from_hex(&token.goal_id) {
            Some(goal_id) => goal_id,
            None => {
                let _ = reply.send(Err(TransportError::Other(format!(
                    "malformed action goal token: {}",
                    token.goal_id
                ))));
                return;
            }
        };
        let targets = ActionTargets::for_action(&token.action);
        let request = action::cancel_request(&goal_id);
        self.send_service_request(
            &targets.cancel_goal,
            request,
            ServiceResponseSink::User(reply),
        )
        .await;
    }

    async fn ensure_feedback_subscription(&mut self, feedback_topic: &str) {
        if self.action_feedback_subs.contains_key(feedback_topic) {
            return;
        }
        let Some(channel_id) = self.channels_by_topic.get(feedback_topic).copied() else {
            debug!(
                topic = feedback_topic,
                "foxglove: action feedback topic not advertised yet"
            );
            return;
        };
        let Some(entry) = self.channels_by_id.get(&channel_id).cloned() else {
            return;
        };
        let Some(schema) = self.schemas_by_id.get(&entry.schema_id).cloned() else {
            return;
        };
        let resolver = self
            .resolvers
            .get(&entry.schema_id)
            .cloned()
            .unwrap_or_default();
        let subscription_id = self.next_subscription_id;
        self.next_subscription_id += 1;
        self.subscribers.insert(
            subscription_id,
            SubscriberSlot {
                topic: feedback_topic.to_string(),
                sink: SlotSink::ActionFeedback,
                schema,
                resolver,
                encoding: entry.encoding,
                min_interval_ns: None,
                last_emitted_ns: 0,
            },
        );
        self.action_feedback_subs
            .insert(feedback_topic.to_string(), subscription_id);
        let subscribe_msg = ClientMessage::Subscribe {
            subscriptions: vec![SubscriptionRequest {
                id: subscription_id,
                channel_id,
            }],
        };
        match serde_json::to_string(&subscribe_msg) {
            Ok(json) => {
                if let Err(err) = self.socket.send(WsMsg::Text(json)).await {
                    warn!(
                        ?err,
                        topic = feedback_topic,
                        "foxglove: feedback subscribe failed"
                    );
                }
            }
            Err(err) => warn!(?err, "serialise foxglove feedback subscribe failed"),
        }
    }

    async fn release_feedback_subscription(&mut self, action: &str) {
        let feedback_topic = ActionTargets::for_action(action).feedback;
        let still_active = self
            .pending_action_goals
            .values()
            .any(|goal| goal.action == action);
        if still_active {
            return;
        }
        if let Some(subscription_id) = self.action_feedback_subs.remove(&feedback_topic) {
            self.handle_unsubscribe(subscription_id).await;
        }
    }

    fn register_service(&mut self, service: wire::AdvertisedService) {
        if let Some(base) = service.name.strip_suffix("/_action/send_goal") {
            self.action_definitions
                .entry(base.to_string())
                .or_default()
                .set_goal_from_send_goal_request(&service.request_schema);
        } else if let Some(base) = service.name.strip_suffix("/_action/get_result") {
            self.action_definitions
                .entry(base.to_string())
                .or_default()
                .set_result_from_get_result_response(&service.response_schema);
        }
        let derive = |explicit: &str, suffix: &str| -> String {
            let trimmed = strip_msg_segment(explicit);
            if !trimmed.is_empty() {
                return trimmed;
            }
            let base = strip_msg_segment(&service.type_name);
            if base.is_empty() {
                String::new()
            } else {
                format!("{base}{suffix}")
            }
        };
        let req_name = derive(&service.request_schema_name, "_Request");
        let resp_name = derive(&service.response_schema_name, "_Response");
        let req_pair = match parse_concatenated_with_resolver(
            &req_name,
            SchemaKind::Message,
            &service.request_schema,
            self.dialect.clone(),
        ) {
            Ok(pair) => pair,
            Err(err) => {
                warn!(
                    ?err,
                    service = service.id,
                    "foxglove service request schema parse failed"
                );
                return;
            }
        };
        let resp_pair = match parse_concatenated_with_resolver(
            &resp_name,
            SchemaKind::Message,
            &service.response_schema,
            self.dialect.clone(),
        ) {
            Ok(pair) => pair,
            Err(err) => {
                warn!(
                    ?err,
                    service = service.id,
                    "foxglove service response schema parse failed"
                );
                return;
            }
        };
        let encoding = if service.encoding.is_empty() {
            match self.dialect {
                Dialect::Ros1 => "ros1".to_string(),
                _ => "cdr".to_string(),
            }
        } else {
            service.encoding.clone()
        };
        let canonical_type = strip_msg_segment(&service.type_name);
        let entry = ServiceEntry {
            id: service.id,
            name: service.name.clone(),
            canonical_type,
            encoding,
            request_schema: Arc::new(req_pair.0),
            response_schema: Arc::new(resp_pair.0),
            request_resolver: req_pair.1,
            response_resolver: resp_pair.1,
        };
        self.services_by_name
            .insert(service.name.clone(), service.id);
        self.services_by_id.insert(service.id, entry);
        let req_split = split_concatenated(&service.request_schema);
        let resp_split = split_concatenated(&service.response_schema);
        if !req_split.root_text.is_empty() || !resp_split.root_text.is_empty() {
            self.service_schema_text.insert(
                service.name.clone(),
                format!("{}\n---\n{}", req_split.root_text, resp_split.root_text),
            );
        }
        for (dep_name, dep_body) in req_split
            .dependencies
            .into_iter()
            .chain(resp_split.dependencies)
        {
            if !dep_name.is_empty() && !dep_body.is_empty() {
                self.dependency_schemas.insert(dep_name, dep_body);
            }
        }
    }

    async fn register_channel(&mut self, channel: AdvertisedChannel) {
        if let Some(base) = channel.topic.strip_suffix("/_action/feedback") {
            self.action_definitions
                .entry(base.to_string())
                .or_default()
                .set_feedback_from_feedback_topic(&channel.schema);
        }
        let canonical_name = strip_msg_segment(&channel.schema_name);
        let encoding = ChannelEncoding::parse(&channel.encoding);
        let (parsed_schema, resolver) = match parse_concatenated_with_resolver(
            &canonical_name,
            SchemaKind::Message,
            &channel.schema,
            self.dialect.clone(),
        ) {
            Ok(pair) => pair,
            Err(err) => {
                warn!(?err, channel = channel.id, topic = %channel.topic, "foxglove schema parse failed");
                return;
            }
        };
        let schema_id = parsed_schema.id.clone();
        let schema_arc = Arc::new(parsed_schema);
        self.schemas_by_id
            .insert(schema_id.clone(), schema_arc.clone());
        self.resolvers.insert(schema_id.clone(), resolver);
        if !channel.schema.is_empty() {
            let split = split_concatenated(&channel.schema);
            if !split.root_text.is_empty() {
                self.topic_schema_text
                    .insert(channel.topic.clone(), split.root_text);
            }
            for (dep_name, dep_body) in split.dependencies {
                if !dep_name.is_empty() && !dep_body.is_empty() {
                    self.dependency_schemas.insert(dep_name, dep_body);
                }
            }
        }
        let channel_id = channel.id;
        let topic = channel.topic;
        self.channels_by_id.insert(
            channel_id,
            ChannelEntry {
                topic: topic.clone(),
                schema_id,
                encoding,
            },
        );
        self.channels_by_topic.insert(topic.clone(), channel_id);
        self.resubscribe_topic(&topic, channel_id).await;
    }

    async fn resubscribe_topic(&mut self, topic: &str, channel_id: u32) {
        if self.awaiting_resubscribe.is_empty() {
            return;
        }
        let pending: Vec<u32> = self
            .awaiting_resubscribe
            .iter()
            .filter(|id| {
                self.subscribers
                    .get(id)
                    .is_some_and(|slot| slot.topic == topic)
            })
            .copied()
            .collect();
        if pending.is_empty() {
            return;
        }
        let Some(entry) = self.channels_by_id.get(&channel_id).cloned() else {
            return;
        };
        let Some(schema) = self.schemas_by_id.get(&entry.schema_id).cloned() else {
            return;
        };
        let resolver = self
            .resolvers
            .get(&entry.schema_id)
            .cloned()
            .unwrap_or_default();
        for id in pending {
            if let Some(slot) = self.subscribers.get_mut(&id) {
                slot.schema = schema.clone();
                slot.resolver = resolver.clone();
                slot.encoding = entry.encoding;
            }
            let subscribe_msg = ClientMessage::Subscribe {
                subscriptions: vec![SubscriptionRequest { id, channel_id }],
            };
            let json = match serde_json::to_string(&subscribe_msg) {
                Ok(json) => json,
                Err(err) => {
                    warn!(?err, "serialise foxglove re-subscribe failed");
                    continue;
                }
            };
            if let Err(err) = self.socket.send(WsMsg::Text(json)).await {
                warn!(?err, %topic, "foxglove: re-subscribe after reconnect failed");
            } else {
                self.awaiting_resubscribe.remove(&id);
            }
        }
    }

    fn publish_discovery(&self) {
        let mut services: Vec<TargetDescriptor> = Vec::new();
        for entry in self.services_by_id.values() {
            if entry.name.starts_with("/rosapi/") || entry.name.contains("/_action/") {
                continue;
            }
            services.push(TargetDescriptor {
                name: entry.name.clone(),
                schema_name: entry.canonical_type.clone(),
                schema_id: None,
                schema_definition: self.service_schema_text.get(&entry.name).cloned(),
            });
        }
        services.sort_by(|a, b| a.name.cmp(&b.name));

        let mut actions: Vec<TargetDescriptor> = Vec::new();
        for entry in self.services_by_id.values() {
            if let Some(base) = entry.name.strip_suffix("/_action/send_goal") {
                let action_type = entry
                    .canonical_type
                    .strip_suffix("_SendGoal")
                    .unwrap_or(&entry.canonical_type)
                    .to_string();
                actions.push(TargetDescriptor {
                    name: base.to_string(),
                    schema_name: action_type,
                    schema_id: None,
                    schema_definition: self
                        .action_definitions
                        .get(base)
                        .and_then(ActionDefinitionParts::to_action_definition),
                });
            }
        }
        actions.sort_by(|a, b| a.name.cmp(&b.name));

        let topics: Vec<TopicDescriptor> = self
            .channels_by_id
            .values()
            .map(|entry| {
                let schema_name = self
                    .schemas_by_id
                    .get(&entry.schema_id)
                    .map(|schema| schema.name.clone())
                    .unwrap_or_default();
                TopicDescriptor {
                    name: entry.topic.clone(),
                    schema_name,
                    schema_id: Some(entry.schema_id.clone()),
                    schema_definition: self.topic_schema_text.get(&entry.topic).cloned(),
                }
            })
            .collect();
        let mut topics = topics;
        topics.sort_by(|a, b| a.name.cmp(&b.name));

        let discovery = Discovery {
            topics,
            services,
            actions,
            dialect: Some(self.dialect.clone()),
            dependency_schemas: self.dependency_schemas.clone(),
        };
        let _ = self.inner.discovery_tx.send(discovery);
    }

    async fn handle_subscribe(
        &mut self,
        topic: &str,
        mut options: SubscribeOptions,
    ) -> TransportResult<Subscription> {
        let channel_id = *self
            .channels_by_topic
            .get(topic)
            .ok_or_else(|| TransportError::UnknownTopic(topic.into()))?;
        let entry = self
            .channels_by_id
            .get(&channel_id)
            .ok_or_else(|| TransportError::UnknownTopic(topic.into()))?
            .clone();
        let schema = self
            .schemas_by_id
            .get(&entry.schema_id)
            .ok_or_else(|| {
                TransportError::Schema(entry.schema_id.to_string(), "schema not in registry".into())
            })?
            .clone();
        let resolver = self
            .resolvers
            .get(&entry.schema_id)
            .cloned()
            .unwrap_or_default();
        let subscription_id = self.next_subscription_id;
        self.next_subscription_id += 1;
        let (sender, receiver) = mpsc::channel::<Frame>(256);
        let watcher_sender = sender.clone();
        let self_tx = self.self_tx.clone();
        spawn_detached(async move {
            watcher_sender.closed().await;
            let _ = self_tx.send(Command::Unsubscribe { subscription_id }).await;
        });
        options = options.with_default_for_schema(&schema.name);
        let min_interval_ns = options.min_interval_ns();
        self.subscribers.insert(
            subscription_id,
            SubscriberSlot {
                topic: topic.to_string(),
                sink: SlotSink::Topic(sender),
                schema: schema.clone(),
                resolver,
                encoding: entry.encoding,
                min_interval_ns,
                last_emitted_ns: 0,
            },
        );
        let _ = channel_id;

        let subscribe_msg = ClientMessage::Subscribe {
            subscriptions: vec![SubscriptionRequest {
                id: subscription_id,
                channel_id,
            }],
        };
        let json = serde_json::to_string(&subscribe_msg)
            .map_err(|err| TransportError::Other(format!("serialise subscribe: {err}")))?;
        self.socket
            .send(WsMsg::Text(json))
            .await
            .map_err(|err| TransportError::Other(format!("ws send subscribe: {err}")))?;
        Ok(Subscription {
            frames: receiver,
            schema,
        })
    }

    async fn handle_unsubscribe(&mut self, subscription_id: u32) {
        if self.subscribers.remove(&subscription_id).is_none() {
            return;
        }
        let msg = ClientMessage::Unsubscribe {
            subscription_ids: vec![subscription_id],
        };
        let json = match serde_json::to_string(&msg) {
            Ok(json) => json,
            Err(err) => {
                warn!(?err, "serialise foxglove unsubscribe failed");
                return;
            }
        };
        if let Err(err) = self.socket.send(WsMsg::Text(json)).await {
            warn!(?err, subscription_id, "foxglove unsubscribe send failed");
        }
    }
}

fn strip_msg_segment(name: &str) -> String {
    let segments: Vec<&str> = name.split('/').collect();
    if segments.len() == 3 && matches!(segments[1], "msg" | "srv" | "action") {
        format!("{}/{}", segments[0], segments[2])
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn strip_msg_segment_handles_three_part_names() {
        assert_eq!(strip_msg_segment("std_msgs/msg/Header"), "std_msgs/Header");
        assert_eq!(strip_msg_segment("std_msgs/Header"), "std_msgs/Header");
        assert_eq!(strip_msg_segment("example/srv/Foo"), "example/Foo");
        assert_eq!(strip_msg_segment("example/action/Bar"), "example/Bar");
    }

    #[test]
    fn config_has_sane_defaults() {
        let config = FoxgloveConfig::new("ws://127.0.0.1:9091");
        assert_eq!(config.connect_timeout, Duration::from_secs(15));
        assert_eq!(config.url, "ws://127.0.0.1:9091");
    }

    #[test]
    fn member_or_whole_returns_the_named_field_or_the_value() {
        let wrapped = CanonicalValue::Struct(BTreeMap::from([(
            "feedback".to_string(),
            CanonicalValue::Int(42),
        )]));
        assert_eq!(
            member_or_whole(wrapped, "feedback"),
            CanonicalValue::Int(42)
        );
        let without_member = CanonicalValue::Struct(BTreeMap::from([(
            "other".to_string(),
            CanonicalValue::Int(1),
        )]));
        assert_eq!(
            member_or_whole(without_member.clone(), "feedback"),
            without_member
        );
        let bare = CanonicalValue::Int(7);
        assert_eq!(member_or_whole(bare.clone(), "feedback"), bare);
    }

    #[test]
    fn shaped_goal_encodes_against_the_parsed_send_goal_schema() {
        let separator = "=".repeat(80);
        let send_goal_request = format!(
            "unique_identifier_msgs/UUID goal_id\nexample_interfaces/action/Fibonacci_Goal goal\n{separator}\nMSG: unique_identifier_msgs/UUID\nuint8[16] uuid\n{separator}\nMSG: example_interfaces/action/Fibonacci_Goal\nint32 order"
        );
        let (schema, resolver) = parse_concatenated_with_resolver(
            "example_interfaces/Fibonacci_SendGoal_Request",
            SchemaKind::Message,
            &send_goal_request,
            Dialect::Ros2,
        )
        .expect("parse send_goal request schema");

        let goal =
            CanonicalValue::Struct(BTreeMap::from([("order".into(), CanonicalValue::Int(5))]));
        let request = action::shape_send_goal_request(
            schema.primary(),
            &ActionGoalId::from_bytes([1u8; 16]),
            goal,
        );
        let encoded = rw_codec_cdr::encode_message(&request, schema.primary(), &resolver);
        assert!(
            encoded.is_ok(),
            "encode against the parsed send_goal schema failed: {encoded:?}"
        );
    }

    #[test]
    fn shaped_goal_encodes_against_a_flattened_send_goal_schema() {
        let separator = "=".repeat(80);
        let flattened = format!(
            "unique_identifier_msgs/UUID goal_id\nint32 order\n{separator}\nMSG: unique_identifier_msgs/UUID\nuint8[16] uuid"
        );
        let (schema, resolver) = parse_concatenated_with_resolver(
            "example_interfaces/Fibonacci_SendGoal_Request",
            SchemaKind::Message,
            &flattened,
            Dialect::Ros2,
        )
        .expect("parse flattened send_goal schema");
        let goal =
            CanonicalValue::Struct(BTreeMap::from([("order".into(), CanonicalValue::Int(5))]));
        let request = action::shape_send_goal_request(
            schema.primary(),
            &ActionGoalId::from_bytes([1u8; 16]),
            goal,
        );
        let encoded = rw_codec_cdr::encode_message(&request, schema.primary(), &resolver);
        assert!(
            encoded.is_ok(),
            "encode against a flattened send_goal schema failed: {encoded:?}"
        );
    }

    #[test]
    fn service_request_pack_and_response_decode_round_trip() {
        use rw_canonical::{ArrayLength, FieldDef, FieldType, MessageDef, PrimitiveType};
        let request_def = MessageDef {
            fields: vec![
                FieldDef {
                    name: "a".into(),
                    field_type: FieldType::Primitive(PrimitiveType::Int64),
                    default: None,
                    comment: None,
                },
                FieldDef {
                    name: "b".into(),
                    field_type: FieldType::Primitive(PrimitiveType::Int64),
                    default: None,
                    comment: None,
                },
            ],
            constants: vec![],
        };
        let request_value = CanonicalValue::Struct(std::collections::BTreeMap::from([
            ("a".into(), CanonicalValue::Int(5)),
            ("b".into(), CanonicalValue::Int(7)),
        ]));
        let resolver: HashMap<String, MessageDef> = HashMap::new();
        let payload =
            rw_codec_cdr::encode_message(&request_value, &request_def, &resolver).unwrap();

        let frame = wire::pack_service_call_request(42, 1, "cdr", &payload);
        assert_eq!(frame[0], wire::OPCODE_CLIENT_SERVICE_CALL_REQUEST);
        let mut as_response = frame.clone();
        as_response[0] = wire::OPCODE_SERVICE_CALL_RESPONSE;
        let parsed = wire::parse_service_call_response(&as_response).unwrap();
        assert_eq!(parsed.call_id, 1);
        let decoded =
            rw_codec_cdr::decode_message(parsed.payload.as_ref(), &request_def, &resolver).unwrap();
        assert_eq!(decoded, request_value);
        let _ = ArrayLength::Unbounded;
    }
}
