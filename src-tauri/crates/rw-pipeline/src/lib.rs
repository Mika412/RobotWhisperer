#![deny(missing_debug_implementations)]

use std::collections::HashMap;
use std::sync::Arc;

use rw_canonical::CanonicalValue;
use rw_core::schema::{SchemaKind, SchemaRegistry};
use rw_session::{SubscriptionHandle, SubscriptionManager};
use rw_transport::{
    ActionCancelToken, ActionGoalStream, ConnectionId, Discovery, SubscribeOptions, Transport,
    TransportError, TransportResult,
};
use rw_transport_dummy::DummyTransport;
use rw_transport_foxglove_ws::{FoxgloveConfig, FoxgloveTransport};
use rw_transport_rosbridge::{RosbridgeConfig, RosbridgeTransport};
use tokio::sync::Mutex;
use uuid::Uuid;

use rw_transport::task::{spawn_task, SpawnedTask};

fn spawn_detached<F>(future: F)
where
    F: std::future::Future<Output = ()> + MaybeSend + 'static,
{
    #[cfg(not(target_family = "wasm"))]
    {
        tokio::spawn(future);
    }
    #[cfg(target_family = "wasm")]
    {
        wasm_bindgen_futures::spawn_local(future);
    }
}

#[derive(Clone)]
pub struct CanonicalPipeline {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for CanonicalPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CanonicalPipeline")
            .field("has_schema_registry", &self.inner.schema_registry.is_some())
            .finish_non_exhaustive()
    }
}

struct Inner {
    subscription_manager: SubscriptionManager,
    connections: Mutex<HashMap<ConnectionId, Arc<dyn Transport>>>,
    subscriptions: Mutex<HashMap<String, ActiveSubscription>>,
    action_goals: Mutex<HashMap<String, (ConnectionId, ActionCancelToken)>>,
    schema_registry: Option<Arc<SchemaRegistry>>,
}

#[derive(Debug)]
pub struct ActiveSubscription {
    #[allow(dead_code)]
    pub schema_id: String,
    #[allow(dead_code)]
    pub schema_name: String,
    #[allow(dead_code)]
    pub viz_role: String,
    #[allow(dead_code)]
    forwarder: SpawnedTask,
    _handle: SubscriptionHandle,
}

#[cfg(not(target_family = "wasm"))]
impl Drop for ActiveSubscription {
    fn drop(&mut self) {
        self.forwarder.abort();
    }
}

impl Default for CanonicalPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl CanonicalPipeline {
    pub fn new() -> Self {
        CanonicalPipeline::with_optional_registry(None)
    }

    pub fn with_schema_registry(registry: Arc<SchemaRegistry>) -> Self {
        CanonicalPipeline::with_optional_registry(Some(registry))
    }

    #[cfg_attr(target_family = "wasm", allow(clippy::arc_with_non_send_sync))]
    fn with_optional_registry(schema_registry: Option<Arc<SchemaRegistry>>) -> Self {
        CanonicalPipeline {
            inner: Arc::new(Inner {
                subscription_manager: SubscriptionManager::default(),
                connections: Mutex::new(HashMap::new()),
                subscriptions: Mutex::new(HashMap::new()),
                action_goals: Mutex::new(HashMap::new()),
                schema_registry,
            }),
        }
    }

    pub fn schema_registry(&self) -> Option<&Arc<SchemaRegistry>> {
        self.inner.schema_registry.as_ref()
    }

    #[allow(dead_code)]
    pub fn subscription_manager(&self) -> &SubscriptionManager {
        &self.inner.subscription_manager
    }

    pub async fn open_foxglove(&self, url: impl Into<String>) -> TransportResult<ConnectionId> {
        let transport = Arc::new(FoxgloveTransport::new(FoxgloveConfig::new(url)));
        transport.connect().await?;
        let id = ConnectionId::new();
        let dyn_transport: Arc<dyn Transport> = transport.clone() as Arc<dyn Transport>;
        self.inner
            .connections
            .lock()
            .await
            .insert(id, dyn_transport.clone());
        self.spawn_schema_watcher(dyn_transport);
        Ok(id)
    }

    pub async fn open_dummy(&self) -> TransportResult<ConnectionId> {
        let transport = Arc::new(DummyTransport::new());
        transport.connect().await?;
        let id = ConnectionId::new();
        let dyn_transport: Arc<dyn Transport> = transport.clone() as Arc<dyn Transport>;
        self.inner
            .connections
            .lock()
            .await
            .insert(id, dyn_transport.clone());
        self.spawn_schema_watcher(dyn_transport);
        Ok(id)
    }

    pub async fn open_rosbridge(&self, url: impl Into<String>) -> TransportResult<ConnectionId> {
        let transport = Arc::new(RosbridgeTransport::new(RosbridgeConfig::new(url)));
        transport.connect().await?;
        let id = ConnectionId::new();
        let dyn_transport: Arc<dyn Transport> = transport.clone() as Arc<dyn Transport>;
        self.inner
            .connections
            .lock()
            .await
            .insert(id, dyn_transport.clone());
        self.spawn_schema_watcher(dyn_transport);
        Ok(id)
    }

    fn spawn_schema_watcher(&self, transport: Arc<dyn Transport>) {
        let Some(registry) = self.inner.schema_registry.clone() else {
            return;
        };
        let mut discovery_rx = transport.discovery();
        spawn_detached(async move {
            let snapshot = discovery_rx.borrow().clone();
            register_discovery(&registry, &snapshot).await;
            while discovery_rx.changed().await.is_ok() {
                let snapshot = discovery_rx.borrow().clone();
                register_discovery(&registry, &snapshot).await;
            }
        });
    }

    pub async fn transport(
        &self,
        connection_id: ConnectionId,
    ) -> TransportResult<Arc<dyn Transport>> {
        self.inner
            .connections
            .lock()
            .await
            .get(&connection_id)
            .cloned()
            .ok_or_else(|| TransportError::Other(format!("unknown connection {connection_id}")))
    }

    pub async fn close(&self, connection_id: ConnectionId) -> TransportResult<()> {
        let removed = self.inner.connections.lock().await.remove(&connection_id);
        if let Some(transport) = removed {
            transport.disconnect().await?;
        }
        Ok(())
    }

    pub async fn subscribe_topic<F>(
        &self,
        connection_id: ConnectionId,
        topic: &str,
        pack_and_send: F,
    ) -> TransportResult<SubscribeResult>
    where
        F: FnMut(&str, &rw_transport::Frame, bool) + MaybeSend + 'static,
    {
        self.subscribe_topic_with_options(
            connection_id,
            topic,
            SubscribeOptions::default(),
            pack_and_send,
        )
        .await
    }

    pub async fn subscribe_topic_with_options<F>(
        &self,
        connection_id: ConnectionId,
        topic: &str,
        options: SubscribeOptions,
        mut pack_and_send: F,
    ) -> TransportResult<SubscribeResult>
    where
        F: FnMut(&str, &rw_transport::Frame, bool) + MaybeSend + 'static,
    {
        let transport = self.transport(connection_id).await?;
        let handle = self
            .inner
            .subscription_manager
            .subscribe_with_options(connection_id, topic, options, transport.as_ref())
            .await?;
        let schema_id = handle.schema.id.to_string();
        let schema_name = handle.schema.name.clone();
        let viz_role = handle.schema.viz_role.wire_id();
        let subscription_id = Uuid::new_v4().to_string();

        if let Some(latest) = &handle.latest {
            pack_and_send(&subscription_id, latest.as_ref(), true);
        }

        let mut receiver = handle.receiver.resubscribe();
        let forwarder_id = subscription_id.clone();
        let forwarder = spawn_task(async move {
            use tokio::sync::broadcast::error::RecvError;
            loop {
                match receiver.recv().await {
                    Ok(frame) => pack_and_send(&forwarder_id, frame.as_ref(), false),
                    Err(RecvError::Lagged(n)) => {
                        tracing::warn!(
                            subscription_id = %forwarder_id,
                            lagged = n,
                            "subscription consumer lagged; frames dropped, continuing",
                        );
                        continue;
                    }
                    Err(RecvError::Closed) => break,
                }
            }
        });

        self.inner.subscriptions.lock().await.insert(
            subscription_id.clone(),
            ActiveSubscription {
                schema_id: schema_id.clone(),
                schema_name: schema_name.clone(),
                viz_role: viz_role.clone(),
                forwarder,
                _handle: handle,
            },
        );
        Ok(SubscribeResult {
            subscription_id,
            schema_id,
            schema_name,
            viz_role,
        })
    }

    pub async fn call_service(
        &self,
        connection_id: ConnectionId,
        service: &str,
        request: CanonicalValue,
    ) -> TransportResult<CanonicalValue> {
        let transport = self.transport(connection_id).await?;
        transport.call_service(service, request).await
    }

    pub async fn send_action_goal(
        &self,
        connection_id: ConnectionId,
        action: &str,
        goal: CanonicalValue,
    ) -> TransportResult<ActionGoalStream> {
        let transport = self.transport(connection_id).await?;
        let stream = transport.send_action_goal(action, goal).await?;
        self.inner.action_goals.lock().await.insert(
            stream.cancel_token.goal_id.clone(),
            (connection_id, stream.cancel_token.clone()),
        );
        Ok(stream)
    }

    pub async fn cancel_action_goal(&self, goal_id: &str) -> TransportResult<()> {
        let entry = self.inner.action_goals.lock().await.remove(goal_id);
        let Some((connection_id, token)) = entry else {
            return Ok(());
        };
        let transport = self.transport(connection_id).await?;
        transport.cancel_action_goal(&token).await
    }

    pub async fn forget_action_goal(&self, goal_id: &str) {
        self.inner.action_goals.lock().await.remove(goal_id);
    }

    pub async fn unsubscribe(&self, subscription_id: &str) -> TransportResult<()> {
        let removed = self
            .inner
            .subscriptions
            .lock()
            .await
            .remove(subscription_id);
        if removed.is_none() {
            return Err(TransportError::Other(format!(
                "unknown subscription {subscription_id}"
            )));
        }
        Ok(())
    }
}

async fn register_discovery(registry: &Arc<SchemaRegistry>, discovery: &Discovery) {
    let mut pending: Vec<(&str, &str, SchemaKind)> = Vec::new();
    for (name, body) in &discovery.dependency_schemas {
        pending.push((name.as_str(), body.as_str(), SchemaKind::Message));
    }
    for topic in &discovery.topics {
        if let Some(def) = topic.schema_definition.as_ref() {
            pending.push((&topic.schema_name, def, SchemaKind::Message));
        }
    }
    for service in &discovery.services {
        if let Some(def) = service.schema_definition.as_ref() {
            pending.push((&service.schema_name, def, SchemaKind::Service));
        }
    }
    for action in &discovery.actions {
        if let Some(def) = action.schema_definition.as_ref() {
            pending.push((&action.schema_name, def, SchemaKind::Action));
        }
    }

    while !pending.is_empty() {
        let mut still = Vec::new();
        let mut progressed = false;
        for (name, body, kind) in pending {
            if register_one(registry, name, body, kind).await {
                progressed = true;
            } else {
                still.push((name, body, kind));
            }
        }
        pending = still;
        if !progressed {
            break;
        }
    }
    for (name, body, kind) in &pending {
        if let Err(err) = registry.register(name, *kind, body).await {
            tracing::warn!(?err, name, "schema failed to register during discovery");
        }
    }
}

async fn register_one(
    registry: &Arc<SchemaRegistry>,
    name: &str,
    definition: &str,
    kind: SchemaKind,
) -> bool {
    registry.register(name, kind, definition).await.is_ok()
}

#[derive(Debug, Clone)]
pub struct SubscribeResult {
    pub subscription_id: String,
    pub schema_id: String,
    pub schema_name: String,
    pub viz_role: String,
}

#[cfg(not(target_family = "wasm"))]
pub trait MaybeSend: Send {}
#[cfg(not(target_family = "wasm"))]
impl<T: Send> MaybeSend for T {}

#[cfg(target_family = "wasm")]
pub trait MaybeSend {}
#[cfg(target_family = "wasm")]
impl<T> MaybeSend for T {}
