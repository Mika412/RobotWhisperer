#![deny(missing_debug_implementations)]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use rw_canonical::{
    canonical_schema_id, ArrayLength, CanonicalSchema, CanonicalValue, Dialect, FieldDef,
    FieldType, MessageDef, ParsedSchema, PrimitiveType, SchemaKind, VisualizationRole,
};
use rw_transport::{
    ActionCancelToken, ActionGoalStream, ConnectionStatus, Discovery, Frame, Subscription,
    TargetDescriptor, TopicDescriptor, Transport, TransportError, TransportResult,
};
use tokio::sync::{mpsc, oneshot, watch, Mutex};

const ADD_TWO_INTS: &str = "/dummy/add_two_ints";
const ADD_TWO_INTS_SCHEMA: &str = "example_interfaces/AddTwoInts";
const ADD_TWO_INTS_DEF: &str = "int64 a\nint64 b\n---\nint64 sum\n";

const FIBONACCI: &str = "/dummy/fibonacci";
const FIBONACCI_SCHEMA: &str = "example_interfaces/Fibonacci";
const FIBONACCI_DEF: &str = "int32 order\n---\nint32[] sequence\n---\nint32[] sequence\n";

#[cfg(not(target_family = "wasm"))]
use tokio::task::JoinHandle;
#[cfg(target_family = "wasm")]
use wasm_bindgen_futures as _;

#[cfg(not(target_family = "wasm"))]
type SpawnedTask = JoinHandle<()>;
#[cfg(target_family = "wasm")]
type SpawnedTask = ();

#[cfg(not(target_family = "wasm"))]
fn spawn_task<F>(future: F) -> SpawnedTask
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    tokio::spawn(future)
}

#[cfg(target_family = "wasm")]
fn spawn_task<F>(future: F) -> SpawnedTask
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

async fn sleep_ms(ms: u64) {
    #[cfg(not(target_family = "wasm"))]
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
    #[cfg(target_family = "wasm")]
    gloo_timers::future::TimeoutFuture::new(ms.min(i32::MAX as u64) as u32).await;
}

#[derive(Debug)]
pub struct DummyTransport {
    inner: Arc<Inner>,
}

struct Inner {
    status_tx: watch::Sender<ConnectionStatus>,
    status_rx: watch::Receiver<ConnectionStatus>,
    discovery_tx: watch::Sender<Discovery>,
    discovery_rx: watch::Receiver<Discovery>,
    schemas: HashMap<String, Arc<CanonicalSchema>>,
    subscribers: Mutex<HashMap<String, Vec<mpsc::Sender<Frame>>>>,
    publisher: Mutex<Option<SpawnedTask>>,
}

impl std::fmt::Debug for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DummyInner").finish_non_exhaustive()
    }
}

impl Default for DummyTransport {
    fn default() -> Self {
        DummyTransport::new()
    }
}

impl DummyTransport {
    pub fn new() -> Self {
        let counter_schema = build_schema("std_msgs/Int64", "int64 data\n", PrimitiveType::Int64);
        let string_schema = build_string_schema();
        let wave_schema =
            build_schema("std_msgs/Float64", "float64 data\n", PrimitiveType::Float64);
        let image_schema = build_image_schema();
        let markers_schema = build_markers_schema();

        let mut schemas = HashMap::new();
        schemas.insert("/dummy/counter".to_string(), counter_schema.clone());
        schemas.insert("/dummy/string".to_string(), string_schema.clone());
        schemas.insert("/dummy/wave".to_string(), wave_schema.clone());
        schemas.insert("/dummy/image".to_string(), image_schema.clone());
        schemas.insert("/dummy/markers".to_string(), markers_schema.clone());

        let discovery = Discovery {
            topics: vec![
                TopicDescriptor {
                    name: "/dummy/counter".into(),
                    schema_name: counter_schema.name.clone(),
                    schema_id: Some(counter_schema.id.clone()),
                    schema_definition: Some(counter_schema.definition.clone()),
                },
                TopicDescriptor {
                    name: "/dummy/string".into(),
                    schema_name: string_schema.name.clone(),
                    schema_id: Some(string_schema.id.clone()),
                    schema_definition: Some(string_schema.definition.clone()),
                },
                TopicDescriptor {
                    name: "/dummy/wave".into(),
                    schema_name: wave_schema.name.clone(),
                    schema_id: Some(wave_schema.id.clone()),
                    schema_definition: Some(wave_schema.definition.clone()),
                },
                TopicDescriptor {
                    name: "/dummy/image".into(),
                    schema_name: image_schema.name.clone(),
                    schema_id: Some(image_schema.id.clone()),
                    schema_definition: Some(image_schema.definition.clone()),
                },
                TopicDescriptor {
                    name: "/dummy/markers".into(),
                    schema_name: markers_schema.name.clone(),
                    schema_id: Some(markers_schema.id.clone()),
                    schema_definition: Some(markers_schema.definition.clone()),
                },
            ],
            services: vec![TargetDescriptor {
                name: ADD_TWO_INTS.into(),
                schema_name: ADD_TWO_INTS_SCHEMA.into(),
                schema_id: Some(canonical_schema_id(ADD_TWO_INTS_DEF)),
                schema_definition: Some(ADD_TWO_INTS_DEF.into()),
            }],
            actions: vec![TargetDescriptor {
                name: FIBONACCI.into(),
                schema_name: FIBONACCI_SCHEMA.into(),
                schema_id: Some(canonical_schema_id(FIBONACCI_DEF)),
                schema_definition: Some(FIBONACCI_DEF.into()),
            }],
            ..Default::default()
        };

        let (status_tx, status_rx) = watch::channel(ConnectionStatus::Disconnected);
        let (discovery_tx, discovery_rx) = watch::channel(discovery);

        DummyTransport {
            inner: Arc::new(Inner {
                status_tx,
                status_rx,
                discovery_tx,
                discovery_rx,
                schemas,
                subscribers: Mutex::new(HashMap::new()),
                publisher: Mutex::new(None),
            }),
        }
    }
}

fn build_schema(name: &str, definition: &str, prim: PrimitiveType) -> Arc<CanonicalSchema> {
    let parsed = ParsedSchema::Message(MessageDef {
        fields: vec![FieldDef {
            name: "data".into(),
            field_type: FieldType::Primitive(prim),
            default: None,
            comment: None,
        }],
        constants: vec![],
    });
    Arc::new(CanonicalSchema {
        id: canonical_schema_id(definition),
        name: name.into(),
        kind: SchemaKind::Message,
        dialect: Dialect::Custom("dummy".into()),
        definition: definition.into(),
        parsed,
        dependencies: vec![],
        viz_role: VisualizationRole::default(),
    })
}

fn build_string_schema() -> Arc<CanonicalSchema> {
    let parsed = ParsedSchema::Message(MessageDef {
        fields: vec![FieldDef {
            name: "data".into(),
            field_type: FieldType::String { bound: None },
            default: None,
            comment: None,
        }],
        constants: vec![],
    });
    Arc::new(CanonicalSchema {
        id: canonical_schema_id("string data\n"),
        name: "std_msgs/String".into(),
        kind: SchemaKind::Message,
        dialect: Dialect::Custom("dummy".into()),
        definition: "string data\n".into(),
        parsed,
        dependencies: vec![],
        viz_role: VisualizationRole::Text,
    })
}

const IMAGE_DEF: &str = "std_msgs/Header header\nuint32 height\nuint32 width\nstring encoding\nuint8 is_bigendian\nuint32 step\nuint8[] data\n";
const MARKERS_DEF: &str = "visualization_msgs/Marker[] markers\n";

fn build_image_schema() -> Arc<CanonicalSchema> {
    let fields = vec![
        primitive_field("height", PrimitiveType::Uint32),
        primitive_field("width", PrimitiveType::Uint32),
        FieldDef {
            name: "encoding".into(),
            field_type: FieldType::String { bound: None },
            default: None,
            comment: None,
        },
        primitive_field("is_bigendian", PrimitiveType::Uint8),
        primitive_field("step", PrimitiveType::Uint32),
        FieldDef {
            name: "data".into(),
            field_type: FieldType::Array {
                element: Box::new(FieldType::Primitive(PrimitiveType::Uint8)),
                length: ArrayLength::Unbounded,
            },
            default: None,
            comment: None,
        },
    ];
    build_viz_schema(
        "sensor_msgs/Image",
        IMAGE_DEF,
        fields,
        VisualizationRole::Image,
    )
}

fn build_markers_schema() -> Arc<CanonicalSchema> {
    let fields = vec![FieldDef {
        name: "markers".into(),
        field_type: FieldType::Array {
            element: Box::new(FieldType::Complex {
                type_name: "visualization_msgs/Marker".into(),
            }),
            length: ArrayLength::Unbounded,
        },
        default: None,
        comment: None,
    }];
    build_viz_schema(
        "visualization_msgs/MarkerArray",
        MARKERS_DEF,
        fields,
        VisualizationRole::MarkerArray,
    )
}

fn primitive_field(name: &str, prim: PrimitiveType) -> FieldDef {
    FieldDef {
        name: name.into(),
        field_type: FieldType::Primitive(prim),
        default: None,
        comment: None,
    }
}

fn build_viz_schema(
    name: &str,
    definition: &str,
    fields: Vec<FieldDef>,
    viz_role: VisualizationRole,
) -> Arc<CanonicalSchema> {
    let parsed = ParsedSchema::Message(MessageDef {
        fields,
        constants: vec![],
    });
    Arc::new(CanonicalSchema {
        id: canonical_schema_id(definition),
        name: name.into(),
        kind: SchemaKind::Message,
        dialect: Dialect::Custom("dummy".into()),
        definition: definition.into(),
        parsed,
        dependencies: vec![],
        viz_role,
    })
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl Transport for DummyTransport {
    async fn connect(&self) -> TransportResult<()> {
        let _ = self.inner.status_tx.send(ConnectionStatus::Connecting);
        let _ = self.inner.status_tx.send(ConnectionStatus::Connected);
        let mut publisher = self.inner.publisher.lock().await;
        if publisher.is_some() {
            return Ok(());
        }
        let inner = self.inner.clone();
        let task = spawn_task(async move {
            let mut tick: i64 = 0;
            loop {
                sleep_ms(100).await;
                tick = tick.wrapping_add(1);
                publish_tick(&inner, tick).await;
            }
        });
        *publisher = Some(task);
        Ok(())
    }

    async fn disconnect(&self) -> TransportResult<()> {
        let mut publisher = self.inner.publisher.lock().await;
        #[cfg(not(target_family = "wasm"))]
        if let Some(handle) = publisher.take() {
            handle.abort();
        }
        #[cfg(target_family = "wasm")]
        {
            *publisher = None;
        }
        self.inner.subscribers.lock().await.clear();
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
        let schema = self
            .inner
            .schemas
            .get(topic)
            .cloned()
            .ok_or_else(|| TransportError::Other(format!("unknown dummy topic {topic}")))?;
        let (sender, receiver) = mpsc::channel(64);
        self.inner
            .subscribers
            .lock()
            .await
            .entry(topic.to_string())
            .or_insert_with(Vec::new)
            .push(sender);
        Ok(Subscription {
            frames: receiver,
            schema,
        })
    }

    async fn publish(&self, _topic: &str, _value: CanonicalValue) -> TransportResult<()> {
        Err(TransportError::Other(
            "dummy transport: publish is not supported".into(),
        ))
    }

    async fn call_service(
        &self,
        service: &str,
        request: CanonicalValue,
    ) -> TransportResult<CanonicalValue> {
        if service != ADD_TWO_INTS {
            return Err(TransportError::Other(format!(
                "unknown dummy service {service}"
            )));
        }
        let a = int_field(&request, "a");
        let b = int_field(&request, "b");
        Ok(struct_one("sum", CanonicalValue::Int(a + b)))
    }

    async fn send_action_goal(
        &self,
        action: &str,
        goal: CanonicalValue,
    ) -> TransportResult<ActionGoalStream> {
        if action != FIBONACCI {
            return Err(TransportError::Other(format!(
                "unknown dummy action {action}"
            )));
        }
        let order = int_field(&goal, "order").clamp(0, 25);
        let (feedback_tx, feedback_rx) = mpsc::channel(16);
        let (result_tx, result_rx) = oneshot::channel();

        spawn_task(async move {
            let mut sequence: Vec<i64> = vec![0, 1];
            for _ in 0..order {
                let len = sequence.len();
                sequence.push(sequence[len - 1] + sequence[len - 2]);
                let feedback = struct_one("sequence", int_array(&sequence));
                if feedback_tx.send(feedback).await.is_err() {
                    return;
                }
                sleep_ms(150).await;
            }
            let _ = result_tx.send(Ok(struct_one("sequence", int_array(&sequence))));
        });

        Ok(ActionGoalStream {
            feedback: feedback_rx,
            result: result_rx,
            cancel_token: ActionCancelToken {
                action: action.to_string(),
                goal_id: format!("dummy-fibonacci-{order}"),
            },
        })
    }

    async fn cancel_action_goal(&self, _token: &ActionCancelToken) -> TransportResult<()> {
        Ok(())
    }
}

fn int_field(value: &CanonicalValue, key: &str) -> i64 {
    let CanonicalValue::Struct(map) = value else {
        return 0;
    };
    match map.get(key) {
        Some(CanonicalValue::Int(v)) => *v,
        Some(CanonicalValue::Uint(v)) => *v as i64,
        _ => 0,
    }
}

fn int_array(values: &[i64]) -> CanonicalValue {
    CanonicalValue::Array(values.iter().copied().map(CanonicalValue::Int).collect())
}

async fn publish_tick(inner: &Arc<Inner>, tick: i64) {
    publish_one(
        inner,
        "/dummy/counter",
        struct_one("data", CanonicalValue::Int(tick)),
    )
    .await;
    publish_one(
        inner,
        "/dummy/string",
        struct_one("data", CanonicalValue::String(format!("frame #{tick}"))),
    )
    .await;
    let v = (tick as f64 / 10.0).sin();
    publish_one(
        inner,
        "/dummy/wave",
        struct_one("data", CanonicalValue::F64(v)),
    )
    .await;
    publish_one(inner, "/dummy/image", make_image_value(tick, 96, 64)).await;
    publish_one(inner, "/dummy/markers", make_markers_value(tick)).await;
}

fn make_image_value(tick: i64, width: u32, height: u32) -> CanonicalValue {
    let shift = (tick.wrapping_mul(4) & 0xff) as u32;
    let mut data = Vec::with_capacity((width * height * 3) as usize);
    for y in 0..height {
        for x in 0..width {
            data.push(((x * 255 / width + shift) % 256) as u8);
            data.push((y * 255 / height % 256) as u8);
            data.push((shift % 256) as u8);
        }
    }
    let mut map = BTreeMap::new();
    map.insert("height".into(), CanonicalValue::Uint(height as u64));
    map.insert("width".into(), CanonicalValue::Uint(width as u64));
    map.insert("encoding".into(), CanonicalValue::String("rgb8".into()));
    map.insert("is_bigendian".into(), CanonicalValue::Uint(0));
    map.insert("step".into(), CanonicalValue::Uint((width * 3) as u64));
    map.insert("data".into(), CanonicalValue::Bytes(data));
    CanonicalValue::Struct(map)
}

fn make_markers_value(tick: i64) -> CanonicalValue {
    const COUNT: usize = 240;
    let phase = tick as f64 * 0.05;
    let mut points = Vec::with_capacity(COUNT);
    for i in 0..COUNT {
        let t = i as f64 / COUNT as f64 * std::f64::consts::TAU;
        let radius = 1.0 + 0.3 * (3.0 * t + phase).sin();
        let mut point = BTreeMap::new();
        point.insert("x".into(), CanonicalValue::F64(radius * t.cos()));
        point.insert("y".into(), CanonicalValue::F64(radius * t.sin()));
        point.insert(
            "z".into(),
            CanonicalValue::F64(0.3 * (2.0 * t + phase).sin()),
        );
        points.push(CanonicalValue::Struct(point));
    }
    let mut color = BTreeMap::new();
    color.insert("r".into(), CanonicalValue::F64(0.92));
    color.insert("g".into(), CanonicalValue::F64(0.28));
    color.insert("b".into(), CanonicalValue::F64(0.6));
    color.insert("a".into(), CanonicalValue::F64(1.0));
    let mut marker = BTreeMap::new();
    marker.insert("points".into(), CanonicalValue::Array(points));
    marker.insert("color".into(), CanonicalValue::Struct(color));
    struct_one(
        "markers",
        CanonicalValue::Array(vec![CanonicalValue::Struct(marker)]),
    )
}

fn struct_one(field: &str, value: CanonicalValue) -> CanonicalValue {
    let mut map = BTreeMap::new();
    map.insert(field.to_string(), value);
    CanonicalValue::Struct(map)
}

async fn publish_one(inner: &Arc<Inner>, topic: &str, value: CanonicalValue) {
    let schema = match inner.schemas.get(topic).cloned() {
        Some(s) => s,
        None => return,
    };
    let frame = Frame {
        timestamp_ns: 0,
        schema,
        value,
        raw: None,
        perf: None,
    };
    let mut subs = inner.subscribers.lock().await;
    let Some(slot) = subs.get_mut(topic) else {
        return;
    };
    slot.retain(|sender| sender.try_send(frame.clone()).is_ok() || !sender.is_closed());
    let snapshot = inner.discovery_rx.borrow().clone();
    let _ = inner.discovery_tx.send(snapshot);
}
