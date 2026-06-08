use std::process::Stdio;
use std::time::Duration;

use rw_canonical::CanonicalValue;
use rw_transport::Transport;
use rw_transport_foxglove_ws::{FoxgloveConfig, FoxgloveTransport};
use rw_transport_rosbridge::{RosbridgeConfig, RosbridgeTransport};
use rw_validation::require_integration_env;
use tokio::time::{timeout, Instant};

const TOPIC: &str = "/rw_parity_string";
const PAYLOAD: &str = "parity-canonical";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn std_msgs_string_decodes_identically_across_foxglove_and_rosbridge() {
    require_integration_env!();

    let mut publisher = std::process::Command::new("limactl")
        .args([
            "shell",
            "ros2",
            "bash",
            "-lc",
            &format!(
                "source /opt/ros/kilted/setup.bash && ros2 topic pub --rate 10 {TOPIC} std_msgs/msg/String 'data: {PAYLOAD}'"
            ),
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

    let fox_value = capture_one_via_foxglove().await;
    let ros_value = capture_one_via_rosbridge().await;

    assert_canonical_string_eq(&fox_value, PAYLOAD);
    assert_canonical_string_eq(&ros_value, PAYLOAD);
    assert_eq!(
        fox_value, ros_value,
        "canonical values should match across transports"
    );
}

async fn capture_one_via_foxglove() -> CanonicalValue {
    let transport = FoxgloveTransport::new(FoxgloveConfig::new("ws://127.0.0.1:9091"));
    transport.connect().await.expect("foxglove connect");

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut sub = loop {
        match transport.subscribe_topic(TOPIC).await {
            Ok(s) => break s,
            Err(_) if Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
            Err(err) => panic!("foxglove subscribe failed: {err}"),
        }
    };
    let frame = timeout(Duration::from_secs(15), sub.frames.recv())
        .await
        .expect("foxglove frame within 15s")
        .expect("channel closed");
    transport.disconnect().await.ok();
    frame.value
}

async fn capture_one_via_rosbridge() -> CanonicalValue {
    let transport = RosbridgeTransport::new(RosbridgeConfig::new("ws://127.0.0.1:9089"));
    transport.connect().await.expect("rosbridge connect");

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut sub = loop {
        match transport.subscribe_topic(TOPIC).await {
            Ok(s) => break s,
            Err(_) if Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Err(err) => panic!("rosbridge subscribe failed: {err}"),
        }
    };
    let frame = timeout(Duration::from_secs(15), sub.frames.recv())
        .await
        .expect("rosbridge frame within 15s")
        .expect("channel closed");
    transport.disconnect().await.ok();
    frame.value
}

fn assert_canonical_string_eq(value: &CanonicalValue, expected: &str) {
    match value {
        CanonicalValue::Struct(fields) => match fields.get("data") {
            Some(CanonicalValue::String(s)) => assert_eq!(s, expected),
            other => panic!("expected data: String, got {other:?}"),
        },
        other => panic!("expected struct, got {other:?}"),
    }
}
