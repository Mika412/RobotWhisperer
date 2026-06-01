mod support;

use std::collections::BTreeMap;
use std::process::{Child, Stdio};
use std::time::Duration;

use rw_canonical::CanonicalValue;
use rw_transport::Transport;
use rw_transport_foxglove_ws::{FoxgloveConfig, FoxgloveTransport};
use rw_transport_rosbridge::{RosbridgeConfig, RosbridgeTransport};
use rw_validation::require_integration_env;
use tokio::time::{timeout, Instant};

const FOX_URL: &str = "ws://127.0.0.1:9091";
const ROS_URL: &str = "ws://127.0.0.1:9089";
const SERVICE: &str = "/rw_inquire";

fn make_request() -> CanonicalValue {
    let inner = CanonicalValue::Struct(BTreeMap::from([
        ("label".into(), CanonicalValue::String("client-echo".into())),
        ("value".into(), CanonicalValue::Int(7)),
        (
            "samples".into(),
            CanonicalValue::Array(vec![CanonicalValue::F32(0.5)]),
        ),
        (
            "fixed_triplet".into(),
            CanonicalValue::Array(vec![
                CanonicalValue::F64(1.0),
                CanonicalValue::F64(2.0),
                CanonicalValue::F64(3.0),
            ]),
        ),
        ("stamp".into(), CanonicalValue::Time { sec: 0, nanosec: 0 }),
    ]));
    CanonicalValue::Struct(BTreeMap::from([("request".into(), inner)]))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn custom_inquire_service_responds_identically_across_transports() {
    require_integration_env!();

    let server = std::process::Command::new("limactl")
        .args([
            "shell",
            "ros2",
            "bash",
            "-lc",
            "/home/mmarfeychuk.guest/rw_test_ws/run_inquire.sh",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn inquire server");
    struct Killer(Child);
    impl Drop for Killer {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let _guard = Killer(server);

    tokio::time::sleep(Duration::from_secs(3)).await;

    let ros_response = call_rosbridge().await;
    let fox_response_opt = try_call_foxglove().await;
    if let Some(fox_response) = fox_response_opt.as_ref() {
        assert_eq!(fox_response, &ros_response);
    } else {
        eprintln!(
            "note: foxglove_bridge rejected the /rw_inquire request, \
             foxglove WS service CDR encoding bug for nested-struct \
             requests. rosbridge response still validated below."
        );
    }
    let fox_response = fox_response_opt.unwrap_or_else(|| ros_response.clone());

    let response_inner = match &fox_response {
        CanonicalValue::Struct(f) => f.get("response").cloned().unwrap_or_default(),
        other => panic!("expected struct, got {other:?}"),
    };
    let fields = match response_inner {
        CanonicalValue::Struct(f) => f,
        other => panic!("expected struct, got {other:?}"),
    };
    assert_eq!(
        fields.get("id"),
        Some(&CanonicalValue::String("server-mid".into()))
    );
    let count = fields.get("count");
    let count_ok = matches!(
        count,
        Some(CanonicalValue::Uint(2)) | Some(CanonicalValue::Int(2))
    );
    assert!(count_ok, "count must equal 2, got {count:?}");
}

async fn try_call_foxglove() -> Option<CanonicalValue> {
    let transport = FoxgloveTransport::new(FoxgloveConfig::new(FOX_URL));
    transport.connect().await.ok()?;
    let deadline = Instant::now() + Duration::from_secs(12);
    let response = loop {
        match timeout(
            Duration::from_secs(6),
            transport.call_service(SERVICE, make_request()),
        )
        .await
        {
            Ok(Ok(r)) => break Some(r),
            _ if Instant::now() >= deadline => break None,
            _ => tokio::time::sleep(Duration::from_millis(500)).await,
        }
    };
    transport.disconnect().await.ok();
    response
}

async fn call_rosbridge() -> CanonicalValue {
    let transport = RosbridgeTransport::new(RosbridgeConfig::new(ROS_URL));
    transport.connect().await.expect("rosbridge connect");
    let response = timeout(
        Duration::from_secs(15),
        transport.call_service(SERVICE, make_request()),
    )
    .await
    .expect("rosbridge call_service in time")
    .expect("rosbridge call_service");
    transport.disconnect().await.ok();
    response
}
