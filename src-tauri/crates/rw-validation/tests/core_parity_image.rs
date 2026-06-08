mod support;

use rw_canonical::CanonicalValue;
use rw_validation::require_integration_env;
use support::{capture_via_foxglove, capture_via_rosbridge, strip_volatile, PublisherGuard};

const TOPIC: &str = "/rw_parity_image";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sensor_msgs_image_decodes_identically_across_transports() {
    require_integration_env!();

    let _publisher = PublisherGuard::spawn(
        TOPIC,
        "sensor_msgs/msg/Image",
        "{ header: { stamp: { sec: 0, nanosec: 0 }, frame_id: cam }, height: 2, width: 2, encoding: rgb8, is_bigendian: 0, step: 6, data: [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255] }",
    );

    let fox = capture_via_foxglove(TOPIC).await;
    let ros = capture_via_rosbridge(TOPIC).await;
    assert_eq!(strip_volatile(&fox), strip_volatile(&ros));

    for value in [&fox, &ros] {
        let fields = match value {
            CanonicalValue::Struct(f) => f,
            other => panic!("expected struct, got {other:?}"),
        };
        assert_eq!(fields.get("width"), Some(&CanonicalValue::Uint(2)));
        assert_eq!(fields.get("height"), Some(&CanonicalValue::Uint(2)));
        assert_eq!(
            fields.get("encoding"),
            Some(&CanonicalValue::String("rgb8".into()))
        );
        match fields.get("data") {
            Some(CanonicalValue::Bytes(bytes)) => {
                assert_eq!(bytes.len(), 12, "12 byte rgb8 payload");
                assert_eq!(bytes[0], 255);
                assert_eq!(bytes[3], 0);
            }
            other => panic!("expected data: Bytes, got {other:?}"),
        }
    }
}
