#![allow(dead_code)]

use std::process::{Child, Stdio};
use std::time::Duration;

use rw_canonical::CanonicalValue;
use rw_transport::Transport;
use rw_transport_foxglove_ws::{FoxgloveConfig, FoxgloveTransport};
use rw_transport_rosbridge::{RosbridgeConfig, RosbridgeTransport};
use tokio::time::{timeout, Instant};

pub const FOXGLOVE_URL: &str = "ws://127.0.0.1:9091";
pub const ROSBRIDGE_URL: &str = "ws://127.0.0.1:9089";

pub struct PublisherGuard(Child);

impl PublisherGuard {
    pub fn spawn(topic: &str, type_name: &str, yaml: &str) -> Self {
        let overlay = "if [ -f /home/mmarfeychuk.guest/rw_test_ws/install/setup.bash ]; then for _pkg in /home/mmarfeychuk.guest/rw_test_ws/install/*; do [ -d \"$_pkg/share\" ] && AMENT_PREFIX_PATH=\"$_pkg:${AMENT_PREFIX_PATH:-}\"; done; export AMENT_PREFIX_PATH; source /home/mmarfeychuk.guest/rw_test_ws/install/setup.bash || true; fi";
        let cmd = format!(
            "source /opt/ros/kilted/setup.bash; {overlay}; ros2 topic pub --rate 10 {topic} {type_name} '{yaml}'",
        );
        eprintln!("[support] spawning publisher: {cmd}");
        let child = std::process::Command::new("limactl")
            .args(["shell", "ros2", "bash", "-lc", &cmd])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn publisher");
        PublisherGuard(child)
    }
}

impl Drop for PublisherGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub async fn capture_via_foxglove(topic: &str) -> CanonicalValue {
    let transport = FoxgloveTransport::new(FoxgloveConfig::new(FOXGLOVE_URL));
    transport.connect().await.expect("foxglove connect");
    let mut sub = retry_subscribe(&transport, topic, Duration::from_secs(45)).await;
    let frame = timeout(Duration::from_secs(15), sub.frames.recv())
        .await
        .expect("foxglove frame within 15s")
        .expect("channel closed");
    transport.disconnect().await.ok();
    frame.value
}

pub async fn capture_via_rosbridge(topic: &str) -> CanonicalValue {
    let transport = RosbridgeTransport::new(RosbridgeConfig::new(ROSBRIDGE_URL));
    transport.connect().await.expect("rosbridge connect");
    let mut sub = retry_subscribe(&transport, topic, Duration::from_secs(45)).await;
    let frame = timeout(Duration::from_secs(15), sub.frames.recv())
        .await
        .expect("rosbridge frame within 15s")
        .expect("channel closed");
    transport.disconnect().await.ok();
    frame.value
}

async fn retry_subscribe(
    transport: &dyn Transport,
    topic: &str,
    deadline_after: Duration,
) -> rw_transport::Subscription {
    let deadline = Instant::now() + deadline_after;
    loop {
        match transport.subscribe_topic(topic).await {
            Ok(s) => return s,
            Err(err) => {
                if Instant::now() >= deadline {
                    panic!(
                        "subscribe failed for {topic} after {deadline_after:?}: last error: {err}",
                    );
                }
                tokio::time::sleep(Duration::from_millis(400)).await;
            }
        }
    }
}

pub fn strip_volatile(value: &CanonicalValue) -> CanonicalValue {
    match value {
        CanonicalValue::Struct(fields) => {
            let mut out = std::collections::BTreeMap::new();
            for (key, v) in fields {
                if is_volatile_field(key) {
                    out.insert(key.clone(), CanonicalValue::Null);
                } else {
                    out.insert(key.clone(), strip_volatile(v));
                }
            }
            CanonicalValue::Struct(out)
        }
        CanonicalValue::Array(items) => {
            CanonicalValue::Array(items.iter().map(strip_volatile).collect())
        }
        other => other.clone(),
    }
}

fn is_volatile_field(name: &str) -> bool {
    matches!(name, "stamp" | "seq")
}
