use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
#[derive(Default)]
pub enum VisualizationRole {
    Image,
    CompressedImage,
    MarkerArray,
    Marker,
    PointCloud2,
    LaserScan,
    Pose,
    PoseStamped,
    Path,
    Odometry,
    Tf,
    Plot {
        field_path: String,
    },
    Text,
    #[default]
    JsonTree,
}

impl VisualizationRole {
    pub fn wire_id(&self) -> String {
        match self {
            VisualizationRole::Image => "image".into(),
            VisualizationRole::CompressedImage => "compressed_image".into(),
            VisualizationRole::MarkerArray => "marker_array".into(),
            VisualizationRole::Marker => "marker".into(),
            VisualizationRole::PointCloud2 => "point_cloud2".into(),
            VisualizationRole::LaserScan => "laser_scan".into(),
            VisualizationRole::Pose => "pose".into(),
            VisualizationRole::PoseStamped => "pose_stamped".into(),
            VisualizationRole::Path => "path".into(),
            VisualizationRole::Odometry => "odometry".into(),
            VisualizationRole::Tf => "tf".into(),
            VisualizationRole::Plot { field_path } => format!("plot:{field_path}"),
            VisualizationRole::Text => "text".into(),
            VisualizationRole::JsonTree => "json_tree".into(),
        }
    }
}

fn normalize_name(name: &str) -> String {
    let segments: Vec<&str> = name.split('/').collect();
    if segments.len() == 3 && matches!(segments[1], "msg" | "srv" | "action") {
        format!("{}/{}", segments[0], segments[2])
    } else {
        name.to_string()
    }
}

pub fn viz_role_for_schema(name: &str) -> VisualizationRole {
    match normalize_name(name).as_str() {
        "sensor_msgs/Image" => VisualizationRole::Image,
        "sensor_msgs/CompressedImage" => VisualizationRole::CompressedImage,
        "sensor_msgs/PointCloud2" => VisualizationRole::PointCloud2,
        "sensor_msgs/LaserScan" => VisualizationRole::LaserScan,
        "sensor_msgs/Imu" => VisualizationRole::JsonTree,
        "geometry_msgs/Pose" => VisualizationRole::Pose,
        "geometry_msgs/PoseStamped" => VisualizationRole::PoseStamped,
        "geometry_msgs/PoseWithCovarianceStamped" => VisualizationRole::PoseStamped,
        "visualization_msgs/Marker" => VisualizationRole::Marker,
        "visualization_msgs/MarkerArray" => VisualizationRole::MarkerArray,
        "nav_msgs/Path" => VisualizationRole::Path,
        "nav_msgs/Odometry" => VisualizationRole::Odometry,
        "tf2_msgs/TFMessage" => VisualizationRole::Tf,
        "std_msgs/String" => VisualizationRole::Text,
        "std_msgs/Bool" | "std_msgs/Int8" | "std_msgs/Int16" | "std_msgs/Int32"
        | "std_msgs/Int64" | "std_msgs/UInt8" | "std_msgs/UInt16" | "std_msgs/UInt32"
        | "std_msgs/UInt64" | "std_msgs/Float32" | "std_msgs/Float64" => VisualizationRole::Plot {
            field_path: "data".into(),
        },
        _ => VisualizationRole::JsonTree,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ros1_and_ros2_names_resolve_identically() {
        assert_eq!(
            viz_role_for_schema("sensor_msgs/Image"),
            VisualizationRole::Image
        );
        assert_eq!(
            viz_role_for_schema("sensor_msgs/msg/Image"),
            VisualizationRole::Image
        );
        assert_eq!(
            viz_role_for_schema("visualization_msgs/MarkerArray"),
            VisualizationRole::MarkerArray
        );
        assert_eq!(
            viz_role_for_schema("visualization_msgs/msg/MarkerArray"),
            VisualizationRole::MarkerArray
        );
    }

    #[test]
    fn unknown_schemas_default_to_json_tree() {
        assert_eq!(
            viz_role_for_schema("some_pkg/Custom"),
            VisualizationRole::JsonTree
        );
    }

    #[test]
    fn numeric_std_msgs_become_plot_with_data_path() {
        match viz_role_for_schema("std_msgs/Float64") {
            VisualizationRole::Plot { field_path } => assert_eq!(field_path, "data"),
            other => panic!("expected Plot, got {other:?}"),
        }
    }

    #[test]
    fn wire_id_is_stable() {
        assert_eq!(VisualizationRole::Image.wire_id(), "image");
        assert_eq!(VisualizationRole::MarkerArray.wire_id(), "marker_array");
        assert_eq!(
            VisualizationRole::Plot {
                field_path: "x.y".into()
            }
            .wire_id(),
            "plot:x.y"
        );
        assert_eq!(VisualizationRole::JsonTree.wire_id(), "json_tree");
    }
}
