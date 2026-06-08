use std::collections::BTreeMap;
use std::time::Duration;

use rw_canonical::CanonicalValue;
use rw_transport::Transport;
use rw_transport_foxglove_ws::{FoxgloveConfig, FoxgloveTransport};
use tokio::time::timeout;

const URL_KILTED: &str = "ws://127.0.0.1:9091";
const URL_HUMBLE: &str = "ws://127.0.0.1:9093";
const SERVICE: &str = "/add_two_ints";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn foxglove_add_two_ints_round_trips_kilted() {
    round_trip(URL_KILTED).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn foxglove_add_two_ints_round_trips_humble() {
    round_trip(URL_HUMBLE).await;
}

async fn round_trip(url: &str) {
    if std::env::var("RW_INTEGRATION").ok().as_deref() != Some("1") {
        eprintln!("skipping live test: RW_INTEGRATION=1 not set");
        return;
    }

    let transport = FoxgloveTransport::new(FoxgloveConfig::new(url));
    transport.connect().await.expect("foxglove connect");

    let mut discovery = transport.discovery();
    let mut tries = 30;
    loop {
        let snap = discovery.borrow_and_update().clone();
        if snap.services.iter().any(|s| s.name == SERVICE) {
            break;
        }
        if tries == 0 {
            panic!("/add_two_ints never appeared in discovery; service running?");
        }
        tries -= 1;
        let _ = timeout(Duration::from_millis(500), discovery.changed()).await;
    }

    let request = CanonicalValue::Struct(BTreeMap::from([
        ("a".into(), CanonicalValue::Int(7)),
        ("b".into(), CanonicalValue::Int(35)),
    ]));

    let response = timeout(
        Duration::from_secs(10),
        transport.call_service(SERVICE, request),
    )
    .await
    .expect("service call within 10s")
    .expect("service call succeeded");

    let CanonicalValue::Struct(fields) = response else {
        panic!("expected struct response, got {response:?}");
    };
    let sum = fields.get("sum").expect("response has no `sum` field");
    let CanonicalValue::Int(n) = sum else {
        panic!("`sum` is not int: {sum:?}");
    };
    assert_eq!(*n, 42, "7 + 35 = 42");

    transport.disconnect().await.ok();
}
