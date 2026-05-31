use std::process::Stdio;
use std::time::Duration;

use rw_canonical::CanonicalValue;
use rw_session::SubscriptionManager;
use rw_transport::{ConnectionId, Transport};
use rw_transport_foxglove_ws::{FoxgloveConfig, FoxgloveTransport};
use tokio::time::{timeout, Instant};

const URL: &str = "ws://127.0.0.1:9091";
const TOPIC: &str = "/rw_session_live_topic";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_session_consumers_share_one_foxglove_subscription() {
    if std::env::var("RW_INTEGRATION").as_deref() != Ok("1") {
        eprintln!("skipping: RW_INTEGRATION != 1");
        return;
    }

    let transport = FoxgloveTransport::new(FoxgloveConfig::new(URL));
    transport.connect().await.expect("connect");

    let mut publisher = std::process::Command::new("limactl")
        .args([
            "shell",
            "ros2",
            "bash",
            "-lc",
            "source /opt/ros/kilted/setup.bash && ros2 topic pub --rate 10 /rw_session_live_topic std_msgs/msg/String 'data: shared'",
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

    let mut discovery = transport.discovery();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if discovery.borrow().topics.iter().any(|t| t.name == TOPIC) {
            break;
        }
        if Instant::now() > deadline {
            panic!("topic never advertised");
        }
        let _ = timeout(Duration::from_millis(500), discovery.changed()).await;
    }

    let manager = SubscriptionManager::new(32);
    let connection = ConnectionId::new();

    let mut handle_a = manager
        .subscribe(connection, TOPIC, &transport as &dyn Transport)
        .await
        .expect("subscribe a");
    let mut handle_b = manager
        .subscribe(connection, TOPIC, &transport as &dyn Transport)
        .await
        .expect("subscribe b");

    assert_eq!(
        manager.refcount(connection, TOPIC).await,
        Some(2),
        "both handles refcounted into a single upstream"
    );

    let a = timeout(Duration::from_secs(10), handle_a.receiver.recv())
        .await
        .expect("frame a within 10s")
        .expect("a channel closed");
    let b = timeout(Duration::from_secs(10), handle_b.receiver.recv())
        .await
        .expect("frame b within 10s")
        .expect("b channel closed");
    assert_eq!(a.schema.name, "std_msgs/String");
    assert_eq!(b.schema.name, "std_msgs/String");
    match &a.value {
        CanonicalValue::Struct(fields) => assert_eq!(
            fields.get("data"),
            Some(&CanonicalValue::String("shared".into()))
        ),
        _ => panic!("expected struct"),
    }

    drop(handle_a);
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(
        manager.refcount(connection, TOPIC).await,
        Some(1),
        "refcount drops to 1 after first handle release"
    );

    drop(handle_b);
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(
        manager.shared_count().await,
        0,
        "upstream torn down after last handle"
    );

    transport.disconnect().await.ok();
}
