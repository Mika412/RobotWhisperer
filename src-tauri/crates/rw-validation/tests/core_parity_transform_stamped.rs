mod support;

use rw_canonical::CanonicalValue;
use rw_validation::require_integration_env;
use support::{capture_via_foxglove, capture_via_rosbridge, strip_volatile, PublisherGuard};

const TOPIC: &str = "/rw_parity_tf";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn geometry_msgs_transform_stamped_decodes_identically_across_transports() {
    require_integration_env!();

    let _publisher = PublisherGuard::spawn(
        TOPIC,
        "geometry_msgs/msg/TransformStamped",
        "{ header: { stamp: { sec: 0, nanosec: 0 }, frame_id: world }, child_frame_id: robot, transform: { translation: { x: 1.0, y: 2.0, z: 3.0 }, rotation: { x: 0.0, y: 0.0, z: 0.0, w: 1.0 } } }",
    );

    let fox = capture_via_foxglove(TOPIC).await;
    let ros = capture_via_rosbridge(TOPIC).await;
    assert_eq!(strip_volatile(&fox), strip_volatile(&ros));

    for value in [&fox, &ros] {
        let fields = match value {
            CanonicalValue::Struct(f) => f,
            other => panic!("expected struct, got {other:?}"),
        };
        assert_eq!(
            fields.get("child_frame_id"),
            Some(&CanonicalValue::String("robot".into()))
        );
        let transform = match fields.get("transform") {
            Some(CanonicalValue::Struct(t)) => t,
            other => panic!("expected transform struct, got {other:?}"),
        };
        let translation = match transform.get("translation") {
            Some(CanonicalValue::Struct(t)) => t,
            other => panic!("expected translation struct, got {other:?}"),
        };
        assert_eq!(translation.get("x"), Some(&CanonicalValue::F64(1.0)));
        let rotation = match transform.get("rotation") {
            Some(CanonicalValue::Struct(r)) => r,
            other => panic!("expected rotation struct, got {other:?}"),
        };
        assert_eq!(rotation.get("w"), Some(&CanonicalValue::F64(1.0)));
    }
}
