mod support;

use rw_canonical::CanonicalValue;
use rw_validation::require_integration_env;
use support::{capture_via_foxglove, capture_via_rosbridge, strip_volatile, PublisherGuard};

const TOPIC: &str = "/rw_parity_laserscan";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sensor_msgs_laserscan_decodes_identically_across_transports() {
    require_integration_env!();

    let _publisher = PublisherGuard::spawn(
        TOPIC,
        "sensor_msgs/msg/LaserScan",
        "{ header: { stamp: { sec: 0, nanosec: 0 }, frame_id: laser }, angle_min: -1.0, angle_max: 1.0, angle_increment: 0.5, time_increment: 0.0, scan_time: 0.0, range_min: 0.0, range_max: 10.0, ranges: [1.0, 2.0, 3.0, 4.0, 5.0], intensities: [0.1, 0.2, 0.3, 0.4, 0.5] }",
    );

    let fox = capture_via_foxglove(TOPIC).await;
    let ros = capture_via_rosbridge(TOPIC).await;
    assert_eq!(strip_volatile(&fox), strip_volatile(&ros));

    for value in [&fox, &ros] {
        let fields = match value {
            CanonicalValue::Struct(f) => f,
            other => panic!("expected struct, got {other:?}"),
        };
        match fields.get("ranges") {
            Some(CanonicalValue::Array(items)) => {
                assert_eq!(items.len(), 5, "5 ranges entries");
                let first = &items[0];
                let v: f64 = match first {
                    CanonicalValue::F32(v) => *v as f64,
                    CanonicalValue::F64(v) => *v,
                    other => panic!("expected float, got {other:?}"),
                };
                assert!((v - 1.0).abs() < 1e-6, "first range == 1.0, got {v}");
            }
            other => panic!("expected ranges array, got {other:?}"),
        }
    }
}
