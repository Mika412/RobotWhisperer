use std::process::Stdio;
use std::time::Duration;

use rw_canonical::CanonicalValue;
use rw_transport::{Discovery, Transport};
use rw_transport_rosbridge::{RosbridgeConfig, RosbridgeTransport};
use tokio::time::{timeout, Instant};

const URL: &str = "ws://127.0.0.1:9089";
const TOPIC: &str = "/rw_rosbridge_live_topic";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ros2_rosbridge_publisher_decodes_to_canonical() {
    if std::env::var("RW_INTEGRATION").as_deref() != Ok("1") {
        eprintln!("skipping: RW_INTEGRATION != 1");
        return;
    }

    let transport = RosbridgeTransport::new(RosbridgeConfig::new(URL));
    transport.connect().await.expect("connect");

    tokio::time::sleep(Duration::from_millis(300)).await;

    let mut publisher = std::process::Command::new("limactl")
        .args([
            "shell",
            "ros2",
            "bash",
            "-lc",
            "source /opt/ros/kilted/setup.bash && ros2 topic pub --rate 10 /rw_rosbridge_live_topic std_msgs/msg/String 'data: hello-rosbridge'",
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

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut sub = loop {
        match transport.subscribe_topic(TOPIC).await {
            Ok(sub) => break sub,
            Err(err) => {
                if Instant::now() > deadline {
                    panic!("subscribe_topic never succeeded: {err}");
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    };
    let _ = Discovery::default();
    let frame = timeout(Duration::from_secs(15), sub.frames.recv())
        .await
        .expect("frame within 15s")
        .expect("channel closed");
    assert_eq!(frame.schema.name, "std_msgs/String");
    match frame.value {
        CanonicalValue::Struct(fields) => {
            assert_eq!(
                fields.get("data"),
                Some(&CanonicalValue::String("hello-rosbridge".into())),
                "decoded String payload"
            );
        }
        other => panic!("expected struct, got {other:?}"),
    }

    transport.disconnect().await.ok();
}
