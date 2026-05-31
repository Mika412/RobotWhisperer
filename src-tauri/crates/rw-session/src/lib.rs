#![deny(missing_debug_implementations)]
#![deny(unused_must_use)]

use std::collections::HashMap;
use std::sync::Arc;

use rw_canonical::CanonicalSchema;
use rw_transport::task::{spawn_task, SpawnedTask};
use rw_transport::{
    ConnectionId, Frame, SubscribeOptions, Transport, TransportError, TransportResult,
};
use tokio::sync::{broadcast, Mutex};
use tracing::warn;

#[derive(Debug)]
pub struct SubscriptionHandle {
    pub schema: Arc<CanonicalSchema>,
    pub receiver: broadcast::Receiver<Arc<Frame>>,
    pub latest: Option<Arc<Frame>>,
    #[allow(dead_code)]
    refcount: Arc<RefcountGuard>,
}

struct RefcountGuard {
    key: SharedKey,
    manager: Arc<ManagerInner>,
}

impl std::fmt::Debug for RefcountGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RefcountGuard")
            .field("key", &self.key)
            .finish()
    }
}

impl Drop for RefcountGuard {
    fn drop(&mut self) {
        let key = self.key.clone();
        let manager = self.manager.clone();
        #[cfg(not(target_family = "wasm"))]
        {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    manager.release(&key).await;
                });
            } else {
                let mut guard = manager.shared.blocking_lock();
                if let Some(shared) = guard.get_mut(&key) {
                    shared.refcount = shared.refcount.saturating_sub(1);
                    if shared.refcount == 0 {
                        guard.remove(&key);
                    }
                }
            }
        }
        #[cfg(target_family = "wasm")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                manager.release(&key).await;
            });
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SharedKey {
    connection_id: ConnectionId,
    topic: String,
}

#[derive(Debug)]
struct SharedSubscription {
    refcount: usize,
    sender: broadcast::Sender<Arc<Frame>>,
    schema: Arc<CanonicalSchema>,
    latest: Arc<Mutex<Option<Arc<Frame>>>>,
    #[allow(dead_code)]
    forwarder: SpawnedTask,
}

#[cfg(not(target_family = "wasm"))]
impl Drop for SharedSubscription {
    fn drop(&mut self) {
        self.forwarder.abort();
    }
}

#[derive(Clone, Debug)]
pub struct SubscriptionManager {
    inner: Arc<ManagerInner>,
}

#[derive(Debug)]
struct ManagerInner {
    shared: Mutex<HashMap<SharedKey, SharedSubscription>>,
    capacity: usize,
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        SubscriptionManager::new(256)
    }
}

impl SubscriptionManager {
    pub fn new(capacity: usize) -> Self {
        SubscriptionManager {
            inner: Arc::new(ManagerInner {
                shared: Mutex::new(HashMap::new()),
                capacity: capacity.max(1),
            }),
        }
    }

    pub async fn subscribe(
        &self,
        connection_id: ConnectionId,
        topic: &str,
        transport: &dyn Transport,
    ) -> TransportResult<SubscriptionHandle> {
        self.subscribe_with_options(connection_id, topic, SubscribeOptions::default(), transport)
            .await
    }

    pub async fn subscribe_with_options(
        &self,
        connection_id: ConnectionId,
        topic: &str,
        options: SubscribeOptions,
        transport: &dyn Transport,
    ) -> TransportResult<SubscriptionHandle> {
        let key = SharedKey {
            connection_id,
            topic: topic.to_string(),
        };
        let mut shared = self.inner.shared.lock().await;
        if let Some(slot) = shared.get_mut(&key) {
            slot.refcount += 1;
            let receiver = slot.sender.subscribe();
            let schema = slot.schema.clone();
            let latest = slot.latest.lock().await.clone();
            return Ok(SubscriptionHandle {
                schema,
                receiver,
                latest,
                refcount: Arc::new(RefcountGuard {
                    key,
                    manager: self.inner.clone(),
                }),
            });
        }

        let mut subscription = transport
            .subscribe_topic_with_options(topic, options)
            .await?;
        let schema = subscription.schema.clone();
        let (sender, receiver) = broadcast::channel::<Arc<Frame>>(self.inner.capacity);
        let latest = Arc::new(Mutex::new(None));
        let forwarder = {
            let sender = sender.clone();
            let latest = latest.clone();
            let topic_for_log = topic.to_string();
            spawn_task(async move {
                while let Some(frame) = subscription.frames.recv().await {
                    let frame = Arc::new(frame);
                    {
                        let mut slot = latest.lock().await;
                        *slot = Some(frame.clone());
                    }
                    if let Err(err) = sender.send(frame) {
                        if sender.receiver_count() > 0 {
                            warn!(?err, topic = %topic_for_log, "fanout send failed");
                        }
                    }
                }
            })
        };
        let shared_sub = SharedSubscription {
            refcount: 1,
            sender,
            schema: schema.clone(),
            latest: latest.clone(),
            forwarder,
        };
        let cached_latest = latest.lock().await.clone();
        shared.insert(key.clone(), shared_sub);

        Ok(SubscriptionHandle {
            schema,
            receiver,
            latest: cached_latest,
            refcount: Arc::new(RefcountGuard {
                key,
                manager: self.inner.clone(),
            }),
        })
    }

    pub async fn shared_count(&self) -> usize {
        self.inner.shared.lock().await.len()
    }

    pub async fn refcount(&self, connection_id: ConnectionId, topic: &str) -> Option<usize> {
        let key = SharedKey {
            connection_id,
            topic: topic.to_string(),
        };
        self.inner.shared.lock().await.get(&key).map(|s| s.refcount)
    }
}

impl ManagerInner {
    async fn release(&self, key: &SharedKey) {
        let mut guard = self.shared.lock().await;
        if let Some(shared) = guard.get_mut(key) {
            shared.refcount = shared.refcount.saturating_sub(1);
            if shared.refcount == 0 {
                guard.remove(key);
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("transport: {0}")]
    Transport(#[from] TransportError),
}

pub type SessionResult<T> = Result<T, SessionError>;
