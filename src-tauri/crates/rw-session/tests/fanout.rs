use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use rw_canonical::{
    CanonicalSchema, CanonicalValue, Dialect, MessageDef, ParsedSchema, SchemaId, SchemaKind,
    VisualizationRole,
};
use rw_session::SubscriptionManager;
use rw_transport::{
    ActionCancelToken, ActionGoalStream, ConnectionId, ConnectionStatus, Discovery, Frame,
    Subscription, Transport, TransportError, TransportResult,
};
use tokio::sync::{mpsc, watch};
use tokio::time::{sleep, timeout};

#[derive(Debug)]
struct MockTransport {
    schema: Arc<CanonicalSchema>,
    upstream_opens: Arc<AtomicUsize>,
    senders: tokio::sync::Mutex<Vec<mpsc::Sender<Frame>>>,
    #[allow(dead_code)]
    status_tx: watch::Sender<ConnectionStatus>,
    status_rx: watch::Receiver<ConnectionStatus>,
    #[allow(dead_code)]
    discovery_tx: watch::Sender<Discovery>,
    discovery_rx: watch::Receiver<Discovery>,
}

impl MockTransport {
    fn new(schema: Arc<CanonicalSchema>) -> Arc<Self> {
        let (status_tx, status_rx) = watch::channel(ConnectionStatus::Connected);
        let (discovery_tx, discovery_rx) = watch::channel(Discovery::default());
        Arc::new(MockTransport {
            schema,
            upstream_opens: Arc::new(AtomicUsize::new(0)),
            senders: tokio::sync::Mutex::new(Vec::new()),
            status_tx,
            status_rx,
            discovery_tx,
            discovery_rx,
        })
    }

    async fn push(&self, value: CanonicalValue) {
        let frame = Frame {
            timestamp_ns: 0,
            schema: self.schema.clone(),
            value,
            raw: None,
            perf: None,
        };
        let mut senders = self.senders.lock().await;
        senders.retain(|tx| tx.try_send(frame.clone()).is_ok() || !tx.is_closed());
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn connect(&self) -> TransportResult<()> {
        Ok(())
    }
    async fn disconnect(&self) -> TransportResult<()> {
        Ok(())
    }
    fn status(&self) -> watch::Receiver<ConnectionStatus> {
        self.status_rx.clone()
    }
    fn discovery(&self) -> watch::Receiver<Discovery> {
        self.discovery_rx.clone()
    }
    async fn subscribe_topic(&self, _topic: &str) -> TransportResult<Subscription> {
        self.upstream_opens.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel(64);
        self.senders.lock().await.push(tx);
        Ok(Subscription {
            frames: rx,
            schema: self.schema.clone(),
        })
    }
    async fn publish(&self, _topic: &str, _value: CanonicalValue) -> TransportResult<()> {
        Ok(())
    }
    async fn call_service(
        &self,
        _service: &str,
        _request: CanonicalValue,
    ) -> TransportResult<CanonicalValue> {
        Err(TransportError::Other("not implemented in mock".into()))
    }
    async fn send_action_goal(
        &self,
        _action: &str,
        _goal: CanonicalValue,
    ) -> TransportResult<ActionGoalStream> {
        Err(TransportError::Other("not implemented in mock".into()))
    }
    async fn cancel_action_goal(&self, _token: &ActionCancelToken) -> TransportResult<()> {
        Err(TransportError::Other("not implemented in mock".into()))
    }
}

fn sample_schema() -> Arc<CanonicalSchema> {
    Arc::new(CanonicalSchema {
        id: SchemaId::new("test"),
        name: "test_pkg/Sample".into(),
        kind: SchemaKind::Message,
        dialect: Dialect::Foxglove,
        definition: "uint32 seq\n".into(),
        parsed: ParsedSchema::Message(MessageDef::default()),
        dependencies: vec![],
        viz_role: VisualizationRole::JsonTree,
    })
}

fn struct_value(seq: u64) -> CanonicalValue {
    CanonicalValue::Struct(BTreeMap::from([("seq".into(), CanonicalValue::Uint(seq))]))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_consumers_share_one_upstream() {
    let schema = sample_schema();
    let transport = MockTransport::new(schema);
    let manager = SubscriptionManager::new(16);
    let connection = ConnectionId::new();

    let mut handle_a = manager
        .subscribe(connection, "/topic", transport.as_ref())
        .await
        .expect("first subscribe");
    let mut handle_b = manager
        .subscribe(connection, "/topic", transport.as_ref())
        .await
        .expect("second subscribe");

    assert_eq!(transport.upstream_opens.load(Ordering::SeqCst), 1);
    assert_eq!(manager.refcount(connection, "/topic").await, Some(2));

    transport.push(struct_value(42)).await;
    let a = timeout(Duration::from_secs(2), handle_a.receiver.recv())
        .await
        .unwrap()
        .unwrap();
    let b = timeout(Duration::from_secs(2), handle_b.receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(a.value, struct_value(42));
    assert_eq!(b.value, struct_value(42));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn last_handle_drop_tears_down_upstream() {
    let schema = sample_schema();
    let transport = MockTransport::new(schema);
    let manager = SubscriptionManager::new(16);
    let connection = ConnectionId::new();

    let handle = manager
        .subscribe(connection, "/topic", transport.as_ref())
        .await
        .unwrap();
    assert_eq!(manager.shared_count().await, 1);
    drop(handle);

    sleep(Duration::from_millis(50)).await;
    assert_eq!(manager.shared_count().await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn late_joiner_sees_latest_frame() {
    let schema = sample_schema();
    let transport = MockTransport::new(schema);
    let manager = SubscriptionManager::new(16);
    let connection = ConnectionId::new();

    let mut early = manager
        .subscribe(connection, "/topic", transport.as_ref())
        .await
        .unwrap();
    transport.push(struct_value(1)).await;
    let _ = timeout(Duration::from_secs(1), early.receiver.recv()).await;

    let late = manager
        .subscribe(connection, "/topic", transport.as_ref())
        .await
        .unwrap();
    let cached = late.latest.expect("latest frame cached for late joiner");
    assert_eq!(cached.value, struct_value(1));
    assert_eq!(transport.upstream_opens.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn first_drop_keeps_upstream_alive_for_second_handle() {
    let schema = sample_schema();
    let transport = MockTransport::new(schema);
    let manager = SubscriptionManager::new(16);
    let connection = ConnectionId::new();

    let handle_a = manager
        .subscribe(connection, "/topic", transport.as_ref())
        .await
        .unwrap();
    let mut handle_b = manager
        .subscribe(connection, "/topic", transport.as_ref())
        .await
        .unwrap();
    drop(handle_a);
    sleep(Duration::from_millis(50)).await;
    assert_eq!(manager.refcount(connection, "/topic").await, Some(1));

    transport.push(struct_value(7)).await;
    let frame = timeout(Duration::from_secs(2), handle_b.receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(frame.value, struct_value(7));
}
