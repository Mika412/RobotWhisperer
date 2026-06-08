use std::process::Stdio;
use std::time::Duration;

use rw_transport::{Discovery, Transport};
use rw_transport_foxglove_ws::{FoxgloveConfig, FoxgloveTransport};
use tokio::time::{timeout, Instant};

const URL: &str = "ws://127.0.0.1:9092";
const TOPIC: &str = "/rw_ros1_fox_topic";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ros1_foxglove_subscribe_rosout_advertises_and_decodes() {
    if std::env::var("RW_INTEGRATION").as_deref() != Ok("1") {
        eprintln!("skipping: RW_INTEGRATION != 1");
        return;
    }
    let publisher = std::process::Command::new("limactl")
        .args([
            "shell",
            "ros1",
            "bash",
            "-lc",
            &format!(
                "source /opt/ros/noetic/setup.bash && rostopic pub --rate 10 {TOPIC} std_msgs/String 'data: hello-ros1-fox'",
            ),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn ros1 publisher");
    struct Killer(std::process::Child);
    impl Drop for Killer {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let _guard = Killer(publisher);

    let transport = FoxgloveTransport::new(FoxgloveConfig::new(URL));
    transport
        .connect()
        .await
        .expect("connect ROS 1 foxglove bridge");

    let mut discovery = transport.discovery();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let snap: Discovery = discovery.borrow().clone();
        if snap.topics.iter().any(|t| t.name == TOPIC) {
            break;
        }
        if Instant::now() > deadline {
            panic!(
                "{TOPIC} never advertised; seen: {:?}",
                snap.topics.iter().map(|t| &t.name).collect::<Vec<_>>()
            );
        }
        let _ = timeout(Duration::from_millis(500), discovery.changed()).await;
    }

    let mut sub = transport.subscribe_topic(TOPIC).await.expect("subscribe");
    let frame = timeout(Duration::from_secs(30), sub.frames.recv())
        .await
        .expect("ros1 frame within 30s")
        .expect("ros1 subscription channel closed");
    assert_eq!(frame.schema.name, "std_msgs/String");
    match frame.value {
        rw_canonical::CanonicalValue::Struct(fields) => {
            assert_eq!(
                fields.get("data"),
                Some(&rw_canonical::CanonicalValue::String(
                    "hello-ros1-fox".into()
                )),
            );
        }
        other => panic!("expected struct, got {other:?}"),
    }

    transport.disconnect().await.ok();
}
