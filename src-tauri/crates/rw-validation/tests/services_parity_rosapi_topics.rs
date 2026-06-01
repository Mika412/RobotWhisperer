mod support;

use std::collections::BTreeMap;
use std::time::Duration;

use rw_canonical::CanonicalValue;
use rw_transport::Transport;
use rw_transport_foxglove_ws::{FoxgloveConfig, FoxgloveTransport};
use rw_transport_rosbridge::{RosbridgeConfig, RosbridgeTransport};
use rw_validation::require_integration_env;
use tokio::time::Instant;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rosapi_topics_response_shape_matches_across_foxglove_and_rosbridge() {
    require_integration_env!();

    let fox_response = call_via_foxglove().await;
    let ros_response = call_via_rosbridge().await;

    assert_topics_shape(&fox_response, "foxglove");
    assert_topics_shape(&ros_response, "rosbridge");
}

async fn call_via_foxglove() -> CanonicalValue {
    let transport = FoxgloveTransport::new(FoxgloveConfig::new("ws://127.0.0.1:9092"));
    transport.connect().await.expect("foxglove connect");

    let deadline = Instant::now() + Duration::from_secs(10);
    let response = loop {
        match transport
            .call_service("/rosapi/topics", CanonicalValue::Struct(BTreeMap::new()))
            .await
        {
            Ok(r) => break r,
            Err(_) if Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
            Err(err) => panic!("foxglove call_service failed: {err}"),
        }
    };
    transport.disconnect().await.ok();
    response
}

async fn call_via_rosbridge() -> CanonicalValue {
    let transport = RosbridgeTransport::new(RosbridgeConfig::new("ws://127.0.0.1:9090"));
    transport.connect().await.expect("rosbridge connect");
    let response = transport
        .call_service("/rosapi/topics", CanonicalValue::Struct(BTreeMap::new()))
        .await
        .expect("rosbridge call_service");
    transport.disconnect().await.ok();
    response
}

fn assert_topics_shape(value: &CanonicalValue, label: &str) {
    let fields = match value {
        CanonicalValue::Struct(f) => f,
        other => panic!("[{label}] expected struct, got {other:?}"),
    };
    let topics = fields
        .get("topics")
        .unwrap_or_else(|| panic!("[{label}] missing 'topics' field"));
    let types = fields
        .get("types")
        .unwrap_or_else(|| panic!("[{label}] missing 'types' field"));
    let topic_items = match topics {
        CanonicalValue::Array(items) => items,
        other => panic!("[{label}] topics not Array: {other:?}"),
    };
    let type_items = match types {
        CanonicalValue::Array(items) => items,
        other => panic!("[{label}] types not Array: {other:?}"),
    };
    assert_eq!(
        topic_items.len(),
        type_items.len(),
        "[{label}] topics/types arrays must match length"
    );
    if let Some(first) = topic_items.first() {
        assert!(
            matches!(first, CanonicalValue::String(_)),
            "[{label}] expected String entries, got {first:?}",
        );
    }
}
