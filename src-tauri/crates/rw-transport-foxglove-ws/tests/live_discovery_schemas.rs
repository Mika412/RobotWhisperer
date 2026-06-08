use std::time::Duration;

use rw_transport::Transport;
use rw_transport_foxglove_ws::{FoxgloveConfig, FoxgloveTransport};
use tokio::time::timeout;

const URL: &str = "ws://127.0.0.1:9091";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discovery_emits_service_schema_text_for_add_two_ints() {
    if std::env::var("RW_INTEGRATION").ok().as_deref() != Some("1") {
        eprintln!("skipping live test: RW_INTEGRATION=1 not set");
        return;
    }

    let transport = FoxgloveTransport::new(FoxgloveConfig::new(URL));
    transport.connect().await.expect("connect");

    let mut rx = transport.discovery();
    let mut tries = 30;
    let svc = loop {
        let snap = rx.borrow_and_update().clone();
        if let Some(svc) = snap
            .services
            .iter()
            .find(|s| s.name == "/add_two_ints")
            .cloned()
        {
            break svc;
        }
        if tries == 0 {
            panic!("/add_two_ints never appeared in discovery");
        }
        tries -= 1;
        let _ = timeout(Duration::from_millis(500), rx.changed()).await;
    };

    let def = svc
        .schema_definition
        .as_deref()
        .expect("service descriptor must carry schema_definition");
    assert!(
        def.contains("int64 a") && def.contains("int64 b"),
        "expected request body in schema_definition, got: {def}"
    );
    assert!(
        def.contains("int64 sum"),
        "expected response body in schema_definition, got: {def}"
    );
    assert!(
        def.contains("\n---\n"),
        "expected `\\n---\\n` separator between request and response, got: {def}"
    );
    assert!(
        !def.contains("================"),
        "schema_definition still contains MSG separator, pre-splitting failed: {def}"
    );

    assert_eq!(svc.schema_name, "example_interfaces/AddTwoInts");

    transport.disconnect().await.ok();
}
