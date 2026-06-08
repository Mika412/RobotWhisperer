use std::collections::BTreeMap;
use std::process::{Child, Stdio};
use std::time::Duration;

use rw_canonical::CanonicalValue;
use rw_transport::Transport;
use rw_transport_rosbridge::{RosbridgeConfig, RosbridgeTransport};
use tokio::time::Instant;

const URL: &str = "ws://127.0.0.1:9089";
const ACTION: &str = "/fibonacci";

#[ignore = "rosbridge ROS 2 Kilted does not expose /rosapi/action_type or /rosapi/service_type"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ros2_fibonacci_action_returns_sequence() {
    if std::env::var("RW_INTEGRATION").as_deref() != Ok("1") {
        eprintln!("skipping: RW_INTEGRATION != 1");
        return;
    }
    if std::env::var("RW_ACTION_SERVER").as_deref() != Ok("1") {
        eprintln!(
            "skipping: RW_ACTION_SERVER != 1; set RW_ACTION_SERVER=1 to spawn the fibonacci server"
        );
        return;
    }

    let server = std::process::Command::new("limactl")
        .args([
            "shell",
            "ros2",
            "bash",
            "-lc",
            "source /opt/ros/kilted/setup.bash && ros2 run action_tutorials_py fibonacci_action_server",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn fibonacci server");
    let mut guard = Killer(server);

    let transport = RosbridgeTransport::new(RosbridgeConfig::new(URL));
    transport.connect().await.expect("connect");

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut stream = loop {
        let goal =
            CanonicalValue::Struct(BTreeMap::from([("order".into(), CanonicalValue::Int(5))]));
        match transport.send_action_goal(ACTION, goal).await {
            Ok(s) => break s,
            Err(err) if Instant::now() < deadline => {
                eprintln!("send_action_goal not ready yet: {err}");
                tokio::time::sleep(Duration::from_millis(750)).await;
            }
            Err(err) => panic!("send_action_goal failed: {err}"),
        }
    };

    let mut feedback_seen = 0u32;
    let result = loop {
        tokio::select! {
            Some(fb) = stream.feedback.recv() => {
                feedback_seen += 1;
                let _ = fb;
            }
            res = &mut stream.result => {
                break res.expect("result channel closed");
            }
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                panic!("action did not complete within 30s; feedback_seen={feedback_seen}");
            }
        }
    };

    let value = result.expect("action goal returned ok");
    match value {
        CanonicalValue::Struct(fields) => match fields.get("sequence") {
            Some(CanonicalValue::Array(seq)) => {
                assert_eq!(seq.len(), 6);
            }
            other => panic!("expected sequence array, got {other:?}"),
        },
        other => panic!("expected struct, got {other:?}"),
    }
    assert!(feedback_seen > 0, "expected at least one feedback frame");

    transport.disconnect().await.ok();
    drop(guard.0.kill());
    let _ = &mut guard;
}

struct Killer(Child);
impl Drop for Killer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
