use crate::schema::{SchemaKind, SchemaRegistry};
use crate::CoreResult;

const BUNDLED: &[(&str, SchemaKind, &str)] = &[
    (
        "builtin_interfaces/Time",
        SchemaKind::Message,
        include_str!("defaults_data/builtin_interfaces/msg/Time.msg"),
    ),
    (
        "builtin_interfaces/Duration",
        SchemaKind::Message,
        include_str!("defaults_data/builtin_interfaces/msg/Duration.msg"),
    ),
    (
        "std_msgs/Empty",
        SchemaKind::Message,
        include_str!("defaults_data/std_msgs/msg/Empty.msg"),
    ),
    (
        "std_msgs/Bool",
        SchemaKind::Message,
        include_str!("defaults_data/std_msgs/msg/Bool.msg"),
    ),
    (
        "std_msgs/String",
        SchemaKind::Message,
        include_str!("defaults_data/std_msgs/msg/String.msg"),
    ),
    (
        "std_msgs/Float32",
        SchemaKind::Message,
        include_str!("defaults_data/std_msgs/msg/Float32.msg"),
    ),
    (
        "std_msgs/Int32",
        SchemaKind::Message,
        include_str!("defaults_data/std_msgs/msg/Int32.msg"),
    ),
    (
        "std_msgs/ColorRGBA",
        SchemaKind::Message,
        include_str!("defaults_data/std_msgs/msg/ColorRGBA.msg"),
    ),
    (
        "std_msgs/Header",
        SchemaKind::Message,
        include_str!("defaults_data/std_msgs/msg/Header.msg"),
    ),
    (
        "geometry_msgs/Vector3",
        SchemaKind::Message,
        include_str!("defaults_data/geometry_msgs/msg/Vector3.msg"),
    ),
    (
        "geometry_msgs/Point",
        SchemaKind::Message,
        include_str!("defaults_data/geometry_msgs/msg/Point.msg"),
    ),
    (
        "geometry_msgs/Quaternion",
        SchemaKind::Message,
        include_str!("defaults_data/geometry_msgs/msg/Quaternion.msg"),
    ),
    (
        "geometry_msgs/Pose",
        SchemaKind::Message,
        include_str!("defaults_data/geometry_msgs/msg/Pose.msg"),
    ),
    (
        "geometry_msgs/Twist",
        SchemaKind::Message,
        include_str!("defaults_data/geometry_msgs/msg/Twist.msg"),
    ),
    (
        "geometry_msgs/Transform",
        SchemaKind::Message,
        include_str!("defaults_data/geometry_msgs/msg/Transform.msg"),
    ),
    (
        "geometry_msgs/PoseWithCovariance",
        SchemaKind::Message,
        include_str!("defaults_data/geometry_msgs/msg/PoseWithCovariance.msg"),
    ),
    (
        "geometry_msgs/TwistWithCovariance",
        SchemaKind::Message,
        include_str!("defaults_data/geometry_msgs/msg/TwistWithCovariance.msg"),
    ),
    (
        "geometry_msgs/PoseStamped",
        SchemaKind::Message,
        include_str!("defaults_data/geometry_msgs/msg/PoseStamped.msg"),
    ),
    (
        "geometry_msgs/TwistStamped",
        SchemaKind::Message,
        include_str!("defaults_data/geometry_msgs/msg/TwistStamped.msg"),
    ),
    (
        "geometry_msgs/TransformStamped",
        SchemaKind::Message,
        include_str!("defaults_data/geometry_msgs/msg/TransformStamped.msg"),
    ),
    (
        "sensor_msgs/PointField",
        SchemaKind::Message,
        include_str!("defaults_data/sensor_msgs/msg/PointField.msg"),
    ),
    (
        "sensor_msgs/NavSatStatus",
        SchemaKind::Message,
        include_str!("defaults_data/sensor_msgs/msg/NavSatStatus.msg"),
    ),
    (
        "sensor_msgs/Image",
        SchemaKind::Message,
        include_str!("defaults_data/sensor_msgs/msg/Image.msg"),
    ),
    (
        "sensor_msgs/CompressedImage",
        SchemaKind::Message,
        include_str!("defaults_data/sensor_msgs/msg/CompressedImage.msg"),
    ),
    (
        "sensor_msgs/LaserScan",
        SchemaKind::Message,
        include_str!("defaults_data/sensor_msgs/msg/LaserScan.msg"),
    ),
    (
        "sensor_msgs/PointCloud2",
        SchemaKind::Message,
        include_str!("defaults_data/sensor_msgs/msg/PointCloud2.msg"),
    ),
    (
        "sensor_msgs/JointState",
        SchemaKind::Message,
        include_str!("defaults_data/sensor_msgs/msg/JointState.msg"),
    ),
    (
        "sensor_msgs/Imu",
        SchemaKind::Message,
        include_str!("defaults_data/sensor_msgs/msg/Imu.msg"),
    ),
    (
        "sensor_msgs/NavSatFix",
        SchemaKind::Message,
        include_str!("defaults_data/sensor_msgs/msg/NavSatFix.msg"),
    ),
    (
        "sensor_msgs/Range",
        SchemaKind::Message,
        include_str!("defaults_data/sensor_msgs/msg/Range.msg"),
    ),
    (
        "sensor_msgs/Temperature",
        SchemaKind::Message,
        include_str!("defaults_data/sensor_msgs/msg/Temperature.msg"),
    ),
    (
        "visualization_msgs/UVCoordinate",
        SchemaKind::Message,
        include_str!("defaults_data/visualization_msgs/msg/UVCoordinate.msg"),
    ),
    (
        "visualization_msgs/MeshFile",
        SchemaKind::Message,
        include_str!("defaults_data/visualization_msgs/msg/MeshFile.msg"),
    ),
    (
        "visualization_msgs/Marker",
        SchemaKind::Message,
        include_str!("defaults_data/visualization_msgs/msg/Marker.msg"),
    ),
    (
        "visualization_msgs/MarkerArray",
        SchemaKind::Message,
        include_str!("defaults_data/visualization_msgs/msg/MarkerArray.msg"),
    ),
    (
        "nav_msgs/MapMetaData",
        SchemaKind::Message,
        include_str!("defaults_data/nav_msgs/msg/MapMetaData.msg"),
    ),
    (
        "nav_msgs/OccupancyGrid",
        SchemaKind::Message,
        include_str!("defaults_data/nav_msgs/msg/OccupancyGrid.msg"),
    ),
    (
        "nav_msgs/Path",
        SchemaKind::Message,
        include_str!("defaults_data/nav_msgs/msg/Path.msg"),
    ),
    (
        "nav_msgs/Odometry",
        SchemaKind::Message,
        include_str!("defaults_data/nav_msgs/msg/Odometry.msg"),
    ),
    (
        "example_interfaces/AddTwoInts",
        SchemaKind::Service,
        include_str!("defaults_data/example_interfaces/srv/AddTwoInts.srv"),
    ),
    (
        "example_interfaces/Fibonacci",
        SchemaKind::Action,
        include_str!("defaults_data/example_interfaces/action/Fibonacci.action"),
    ),
];

pub async fn install_into(registry: &SchemaRegistry) -> CoreResult<()> {
    for (name, kind, definition) in BUNDLED {
        registry.register(name, *kind, definition).await?;
    }
    Ok(())
}

pub const BUNDLED_COUNT: usize = BUNDLED.len();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SqliteStorage;
    use crate::util::MockClock;
    use chrono::TimeZone;
    use std::sync::Arc;

    async fn fresh_registry() -> SchemaRegistry {
        let clock = Arc::new(MockClock::new(
            chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        ));
        let storage: Arc<dyn crate::storage::Storage> =
            Arc::new(SqliteStorage::open_in_memory(clock).expect("in-memory storage"));
        SchemaRegistry::new(storage).await.unwrap()
    }

    #[tokio::test]
    async fn all_bundled_defaults_register() {
        let registry = fresh_registry().await;
        registry.ensure_defaults().await.expect("ensure_defaults");
        let names = registry.list_names();
        assert_eq!(
            names.len(),
            BUNDLED_COUNT,
            "registered {} but bundle declares {}",
            names.len(),
            BUNDLED_COUNT
        );
        for required in [
            "std_msgs/Header",
            "geometry_msgs/PoseStamped",
            "sensor_msgs/PointCloud2",
            "visualization_msgs/MarkerArray",
            "nav_msgs/Odometry",
            "example_interfaces/AddTwoInts",
            "example_interfaces/Fibonacci",
        ] {
            assert!(
                registry.get_by_name(required).len() == 1,
                "{required} missing"
            );
        }
    }

    #[tokio::test]
    async fn ensure_defaults_is_idempotent() {
        let registry = fresh_registry().await;
        registry.ensure_defaults().await.unwrap();
        registry.ensure_defaults().await.unwrap();
        assert_eq!(registry.list_names().len(), BUNDLED_COUNT);
    }

    #[tokio::test]
    async fn bundled_hashes_are_stable_across_invocations() {
        let first_registry = fresh_registry().await;
        first_registry.ensure_defaults().await.unwrap();
        let first_marker = first_registry
            .require_by_name("visualization_msgs/Marker")
            .unwrap();

        let second_registry = fresh_registry().await;
        second_registry.ensure_defaults().await.unwrap();
        let second_marker = second_registry
            .require_by_name("visualization_msgs/Marker")
            .unwrap();
        assert_eq!(first_marker.hash, second_marker.hash);
    }
}
