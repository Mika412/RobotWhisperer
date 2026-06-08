mod support;

use rw_canonical::CanonicalValue;
use rw_validation::require_integration_env;
use support::{capture_via_foxglove, capture_via_rosbridge, strip_volatile, PublisherGuard};

const TOPIC: &str = "/rw_parity_pointcloud2";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sensor_msgs_pointcloud2_decodes_identically_across_transports() {
    require_integration_env!();

    let _publisher = PublisherGuard::spawn(
        TOPIC,
        "sensor_msgs/msg/PointCloud2",
        "{ header: { stamp: { sec: 0, nanosec: 0 }, frame_id: cloud }, height: 1, width: 1, fields: [ { name: x, offset: 0, datatype: 7, count: 1 }, { name: y, offset: 4, datatype: 7, count: 1 }, { name: z, offset: 8, datatype: 7, count: 1 } ], is_bigendian: false, point_step: 12, row_step: 12, data: [0, 0, 128, 63, 0, 0, 0, 64, 0, 0, 64, 64], is_dense: true }",
    );

    let fox = capture_via_foxglove(TOPIC).await;
    let ros = capture_via_rosbridge(TOPIC).await;
    assert_eq!(strip_volatile(&fox), strip_volatile(&ros));

    for value in [&fox, &ros] {
        let fields_map = match value {
            CanonicalValue::Struct(f) => f,
            other => panic!("expected struct, got {other:?}"),
        };
        let fields_arr = match fields_map.get("fields") {
            Some(CanonicalValue::Array(items)) => items,
            other => panic!("expected fields array, got {other:?}"),
        };
        assert_eq!(fields_arr.len(), 3, "three PointField entries");
        let first = match &fields_arr[0] {
            CanonicalValue::Struct(s) => s,
            other => panic!("expected PointField struct, got {other:?}"),
        };
        assert_eq!(first.get("name"), Some(&CanonicalValue::String("x".into())));
        assert_eq!(first.get("offset"), Some(&CanonicalValue::Uint(0)));
        assert_eq!(first.get("datatype"), Some(&CanonicalValue::Uint(7)));
        assert_eq!(first.get("count"), Some(&CanonicalValue::Uint(1)));

        match fields_map.get("data") {
            Some(CanonicalValue::Bytes(bytes)) => {
                assert_eq!(bytes.len(), 12, "12 byte point payload");
            }
            other => panic!("expected data: Bytes, got {other:?}"),
        }
    }
}
