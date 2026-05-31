use std::collections::BTreeMap;
use std::time::Duration;

use rw_canonical::CanonicalValue;
use rw_transport::Transport;
use rw_transport_rosbridge::{RosbridgeConfig, RosbridgeTransport};
use tokio::time::timeout;

const URL: &str = "ws://127.0.0.1:9089";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rosbridge_rosapi_topics_via_call_service() {
    if std::env::var("RW_INTEGRATION").as_deref() != Ok("1") {
        eprintln!("skipping: RW_INTEGRATION != 1");
        return;
    }
    let transport = RosbridgeTransport::new(RosbridgeConfig::new(URL));
    transport.connect().await.expect("connect");

    let request = CanonicalValue::Struct(BTreeMap::new());
    let response = timeout(
        Duration::from_secs(10),
        transport.call_service("/rosapi/topics", request),
    )
    .await
    .expect("call_service within 10s")
    .expect("service call returned ok");

    match response {
        CanonicalValue::Struct(fields) => {
            assert!(
                matches!(fields.get("topics"), Some(CanonicalValue::Array(_))),
                "topics array present; got {:?}",
                fields.get("topics")
            );
            assert!(
                matches!(fields.get("types"), Some(CanonicalValue::Array(_))),
                "types array present; got {:?}",
                fields.get("types")
            );
        }
        other => panic!("expected struct, got {other:?}"),
    }

    transport.disconnect().await.ok();
}
