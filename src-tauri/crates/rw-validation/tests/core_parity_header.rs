mod support;

use rw_canonical::CanonicalValue;
use rw_validation::require_integration_env;
use support::{capture_via_foxglove, capture_via_rosbridge, strip_volatile, PublisherGuard};

const TOPIC: &str = "/rw_parity_header";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn std_msgs_header_decodes_identically_across_transports() {
    require_integration_env!();

    let _publisher = PublisherGuard::spawn(
        TOPIC,
        "std_msgs/msg/Header",
        "{ stamp: { sec: 0, nanosec: 0 }, frame_id: parity_frame }",
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
            fields.get("frame_id"),
            Some(&CanonicalValue::String("parity_frame".into())),
        );
        match fields.get("stamp") {
            Some(CanonicalValue::Time { .. }) => {}
            other => panic!("expected stamp: Time, got {other:?}"),
        }
    }
}
