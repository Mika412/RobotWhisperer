mod support;

use rw_canonical::CanonicalValue;
use rw_validation::require_integration_env;
use support::{capture_via_foxglove, capture_via_rosbridge, strip_volatile, PublisherGuard};

const TOPIC: &str = "/rw_parity_markers";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn visualization_msgs_marker_array_decodes_identically_across_transports() {
    require_integration_env!();

    let _publisher = PublisherGuard::spawn(
        TOPIC,
        "visualization_msgs/msg/MarkerArray",
        "{ markers: [ { header: { stamp: { sec: 0, nanosec: 0 }, frame_id: world }, ns: parity, id: 7, type: 4, action: 0, pose: { position: { x: 1.0, y: 2.0, z: 3.0 }, orientation: { x: 0.0, y: 0.0, z: 0.0, w: 1.0 } }, scale: { x: 0.5, y: 0.5, z: 0.5 }, color: { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }, lifetime: { sec: 0, nanosec: 0 }, frame_locked: false, points: [ { x: 1.0, y: 0.0, z: 0.0 }, { x: 0.0, y: 1.0, z: 0.0 } ], colors: [ { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }, { r: 0.0, g: 1.0, b: 0.0, a: 1.0 } ], texture_resource: '', texture: { header: { stamp: { sec: 0, nanosec: 0 }, frame_id: '' }, format: '', data: [] }, uv_coordinates: [], text: '', mesh_resource: '', mesh_file: { filename: '', data: [] }, mesh_use_embedded_materials: false } ] }",
    );

    let fox = capture_via_foxglove(TOPIC).await;
    let ros = capture_via_rosbridge(TOPIC).await;
    assert_eq!(strip_volatile(&fox), strip_volatile(&ros));

    for value in [&fox, &ros] {
        let fields = match value {
            CanonicalValue::Struct(f) => f,
            other => panic!("expected struct, got {other:?}"),
        };
        let markers = match fields.get("markers") {
            Some(CanonicalValue::Array(items)) => items,
            other => panic!("expected markers array, got {other:?}"),
        };
        assert_eq!(markers.len(), 1, "one marker entry");
        let marker = match &markers[0] {
            CanonicalValue::Struct(m) => m,
            other => panic!("expected Marker struct, got {other:?}"),
        };
        assert_eq!(
            marker.get("ns"),
            Some(&CanonicalValue::String("parity".into()))
        );
        match marker.get("points") {
            Some(CanonicalValue::Array(points)) => {
                assert_eq!(points.len(), 2);
                let first = match &points[0] {
                    CanonicalValue::Struct(p) => p,
                    other => panic!("expected Point struct, got {other:?}"),
                };
                assert_eq!(first.get("x"), Some(&CanonicalValue::F64(1.0)));
            }
            other => panic!("expected points array, got {other:?}"),
        }
    }
}
