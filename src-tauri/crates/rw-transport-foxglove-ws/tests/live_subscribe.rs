use std::process::Stdio;
use std::time::Duration;

use rw_canonical::CanonicalValue;
use rw_transport::{Discovery, Transport};
use rw_transport_foxglove_ws::{FoxgloveConfig, FoxgloveTransport};
use tokio::time::{timeout, Instant};

const FOXGLOVE_URL: &str = "ws://127.0.0.1:9091";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ros2_foxglove_publisher_decodes_to_canonical() {
    if std::env::var("RW_INTEGRATION").as_deref() != Ok("1") {
        eprintln!("skipping: RW_INTEGRATION != 1");
        return;
    }

    let transport = FoxgloveTransport::new(FoxgloveConfig::new(FOXGLOVE_URL));
    transport.connect().await.expect("connect");

    let topic = "/rw_live_test_topic";

    let mut publisher = std::process::Command::new("limactl")
        .args([
            "shell",
            "ros2",
            "bash",
            "-lc",
            "source /opt/ros/kilted/setup.bash && ros2 topic pub --rate 10 /rw_live_test_topic std_msgs/msg/String 'data: hello'",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn publisher");

    struct Killer<'a>(&'a mut std::process::Child);
    impl<'a> Drop for Killer<'a> {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut _killer = Killer(&mut publisher);

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut discovery_rx = transport.discovery();
    loop {
        let snapshot: Discovery = discovery_rx.borrow().clone();
        if snapshot
            .topics
            .iter()
            .any(|topic_info| topic_info.name == topic)
        {
            break;
        }
        if Instant::now() > deadline {
            panic!(
                "topic {topic} did not appear in discovery within 20s; saw {:?}",
                snapshot.topics.iter().map(|t| &t.name).collect::<Vec<_>>()
            );
        }
        let _ = timeout(Duration::from_millis(500), discovery_rx.changed()).await;
    }

    let mut subscription = transport.subscribe_topic(topic).await.expect("subscribe");
    let frame = timeout(Duration::from_secs(10), subscription.frames.recv())
        .await
        .expect("frame within 10s")
        .expect("subscription channel closed");

    assert!(
        !frame.schema.id.as_str().is_empty(),
        "schema id should be populated"
    );
    assert_eq!(frame.schema.name, "std_msgs/String");

    match frame.value {
        CanonicalValue::Struct(fields) => {
            assert_eq!(
                fields.get("data"),
                Some(&CanonicalValue::String("hello".into())),
                "decoded std_msgs/String payload"
            );
        }
        other => panic!("expected struct, got {other:?}"),
    }

    transport.disconnect().await.expect("disconnect");
}
