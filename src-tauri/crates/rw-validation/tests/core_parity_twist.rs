mod support;

use rw_canonical::CanonicalValue;
use rw_validation::require_integration_env;
use support::{capture_via_foxglove, capture_via_rosbridge, strip_volatile, PublisherGuard};

const TOPIC: &str = "/rw_parity_twist";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn geometry_msgs_twist_decodes_identically_across_transports() {
    require_integration_env!();

    let _publisher = PublisherGuard::spawn(
        TOPIC,
        "geometry_msgs/msg/Twist",
        "{ linear: { x: 1.0, y: 2.0, z: 3.0 }, angular: { x: 0.1, y: 0.2, z: 0.3 } }",
    );

    let fox = capture_via_foxglove(TOPIC).await;
    let ros = capture_via_rosbridge(TOPIC).await;
    assert_eq!(strip_volatile(&fox), strip_volatile(&ros));

    for value in [&fox, &ros] {
        let fields = match value {
            CanonicalValue::Struct(f) => f,
            other => panic!("expected struct, got {other:?}"),
        };
        let linear = match fields.get("linear") {
            Some(CanonicalValue::Struct(l)) => l,
            other => panic!("expected linear struct, got {other:?}"),
        };
        assert_eq!(linear.get("x"), Some(&CanonicalValue::F64(1.0)));
        assert_eq!(linear.get("y"), Some(&CanonicalValue::F64(2.0)));
        assert_eq!(linear.get("z"), Some(&CanonicalValue::F64(3.0)));
    }
}
