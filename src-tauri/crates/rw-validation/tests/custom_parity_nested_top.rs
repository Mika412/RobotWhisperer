mod support;

use rw_canonical::CanonicalValue;
use rw_validation::require_integration_env;
use support::{capture_via_foxglove, capture_via_rosbridge, strip_volatile, PublisherGuard};

const TOPIC: &str = "/rw_parity_custom_top";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn custom_nested_top_decodes_identically_across_transports() {
    require_integration_env!();

    let yaml = "{ \
        header: { stamp: { sec: 0, nanosec: 0 }, frame_id: nested }, \
        main: { \
            inner: { label: a, value: 1, samples: [1.0, 2.0], fixed_triplet: [0.5, 0.5, 0.5], stamp: { sec: 0, nanosec: 0 } }, \
            inners: [ \
                { label: x, value: 10, samples: [], fixed_triplet: [0.1, 0.2, 0.3], stamp: { sec: 0, nanosec: 0 } }, \
                { label: y, value: 20, samples: [1.5], fixed_triplet: [0.4, 0.5, 0.6], stamp: { sec: 0, nanosec: 0 } } \
            ], \
            fixed_inners: [ \
                { label: f1, value: 100, samples: [3.14, 1.41], fixed_triplet: [1.0, 2.0, 3.0], stamp: { sec: 0, nanosec: 0 } }, \
                { label: f2, value: 200, samples: [], fixed_triplet: [4.0, 5.0, 6.0], stamp: { sec: 0, nanosec: 0 } } \
            ], \
            id: top-main, \
            count: 42, \
            weight: 1.5 \
        }, \
        alternatives: [ \
            { \
                inner: { label: alt1, value: -1, samples: [-1.0], fixed_triplet: [0.0, 0.0, 0.0], stamp: { sec: 0, nanosec: 0 } }, \
                inners: [], \
                fixed_inners: [ \
                    { label: alt1f1, value: 0, samples: [], fixed_triplet: [0.0, 0.0, 0.0], stamp: { sec: 0, nanosec: 0 } }, \
                    { label: alt1f2, value: 0, samples: [], fixed_triplet: [0.0, 0.0, 0.0], stamp: { sec: 0, nanosec: 0 } } \
                ], \
                id: alt1, \
                count: 0, \
                weight: 0.0 \
            } \
        ], \
        total: 9999, \
        finalized: true \
    }";

    let _publisher = PublisherGuard::spawn(TOPIC, "rw_test_msgs/msg/Top", yaml);

    let fox = capture_via_foxglove(TOPIC).await;
    let ros = capture_via_rosbridge(TOPIC).await;
    assert_eq!(strip_volatile(&fox), strip_volatile(&ros));

    for (label, value) in [("foxglove", &fox), ("rosbridge", &ros)] {
        let fields = match value {
            CanonicalValue::Struct(f) => f,
            other => panic!("[{label}] expected struct, got {other:?}"),
        };
        assert_eq!(fields.get("total"), Some(&CanonicalValue::Int(9999)));
        assert_eq!(fields.get("finalized"), Some(&CanonicalValue::Bool(true)));

        let main = match fields.get("main") {
            Some(CanonicalValue::Struct(m)) => m,
            other => panic!("[{label}] main not struct: {other:?}"),
        };
        assert_eq!(
            main.get("id"),
            Some(&CanonicalValue::String("top-main".into()))
        );
        assert_eq!(main.get("count"), Some(&CanonicalValue::Uint(42)));

        let fixed = match main.get("fixed_inners") {
            Some(CanonicalValue::Array(a)) => a,
            other => panic!("[{label}] fixed_inners not array: {other:?}"),
        };
        assert_eq!(fixed.len(), 2, "[{label}] fixed_inners must have length 2");
        let first = match &fixed[0] {
            CanonicalValue::Struct(s) => s,
            other => panic!("[{label}] fixed_inners[0] not struct: {other:?}"),
        };
        assert_eq!(
            first.get("label"),
            Some(&CanonicalValue::String("f1".into()))
        );
        let samples = match first.get("samples") {
            Some(CanonicalValue::Array(a)) => a,
            other => panic!("[{label}] samples not array: {other:?}"),
        };
        assert_eq!(samples.len(), 2);
        match &samples[0] {
            CanonicalValue::F32(_) => {}
            other => panic!("[{label}] expected F32 sample, got {other:?}"),
        }
        match first.get("fixed_triplet") {
            Some(CanonicalValue::Array(items)) => {
                assert_eq!(items.len(), 3);
                match &items[0] {
                    CanonicalValue::F64(_) => {}
                    other => panic!("[{label}] expected F64 triplet, got {other:?}"),
                }
            }
            other => panic!("[{label}] fixed_triplet not array: {other:?}"),
        }
    }
}
