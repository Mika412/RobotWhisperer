mod support;

use rw_canonical::CanonicalValue;
use rw_validation::require_integration_env;
use support::{capture_via_foxglove, capture_via_rosbridge, strip_volatile, PublisherGuard};

const TOPIC: &str = "/rw_parity_pose";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn geometry_msgs_pose_decodes_identically_across_transports() {
    require_integration_env!();

    let _publisher = PublisherGuard::spawn(
        TOPIC,
        "geometry_msgs/msg/Pose",
        "{ position: { x: 1.0, y: 2.0, z: 3.0 }, orientation: { x: 0.0, y: 0.0, z: 0.0, w: 1.0 } }",
    );

    let fox = capture_via_foxglove(TOPIC).await;
    let ros = capture_via_rosbridge(TOPIC).await;

    assert_eq!(strip_volatile(&fox), strip_volatile(&ros));
    assert_pose_payload(&fox);
    assert_pose_payload(&ros);
}

fn assert_pose_payload(value: &CanonicalValue) {
    let fields = match value {
        CanonicalValue::Struct(f) => f,
        other => panic!("expected struct, got {other:?}"),
    };
    let position = match fields.get("position") {
        Some(CanonicalValue::Struct(p)) => p,
        other => panic!("expected position struct, got {other:?}"),
    };
    assert_eq!(position.get("x"), Some(&CanonicalValue::F64(1.0)));
    assert_eq!(position.get("y"), Some(&CanonicalValue::F64(2.0)));
    assert_eq!(position.get("z"), Some(&CanonicalValue::F64(3.0)));
    let orientation = match fields.get("orientation") {
        Some(CanonicalValue::Struct(o)) => o,
        other => panic!("expected orientation struct, got {other:?}"),
    };
    assert_eq!(orientation.get("w"), Some(&CanonicalValue::F64(1.0)));
}
