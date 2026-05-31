use std::collections::BTreeMap;
use std::time::Duration;

use rw_canonical::CanonicalValue;
use rw_transport::Transport;
use rw_transport_foxglove_ws::{FoxgloveConfig, FoxgloveTransport};
use tokio::time::{timeout, Instant};

const URL: &str = "ws://127.0.0.1:9092";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ros1_foxglove_call_service_rosapi_topics() {
    if std::env::var("RW_INTEGRATION").as_deref() != Ok("1") {
        eprintln!("skipping: RW_INTEGRATION != 1");
        return;
    }
    let transport = FoxgloveTransport::new(FoxgloveConfig::new(URL));
    transport.connect().await.expect("connect");

    let deadline = Instant::now() + Duration::from_secs(10);
    let response = loop {
        let req = CanonicalValue::Struct(BTreeMap::new());
        match transport.call_service("/rosapi/topics", req).await {
            Ok(r) => break r,
            Err(err) if Instant::now() < deadline => {
                eprintln!("waiting for service advertise: {err}");
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Err(err) => panic!("call_service failed: {err}"),
        }
    };

    let _ = timeout(Duration::from_secs(0), async {}).await;

    match response {
        CanonicalValue::Struct(fields) => {
            assert!(
                matches!(fields.get("topics"), Some(CanonicalValue::Array(_))),
                "expected topics array, got {:?}",
                fields.get("topics")
            );
            assert!(
                matches!(fields.get("types"), Some(CanonicalValue::Array(_))),
                "expected types array, got {:?}",
                fields.get("types")
            );
        }
        other => panic!("expected struct, got {other:?}"),
    }

    transport.disconnect().await.ok();
}
