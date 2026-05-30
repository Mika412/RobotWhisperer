use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Dialect {
    Ros1,
    Ros2,
    Foxglove,
    Custom(String),
}

impl Dialect {
    pub fn is_ros(&self) -> bool {
        matches!(self, Dialect::Ros1 | Dialect::Ros2)
    }

    pub fn label(&self) -> String {
        match self {
            Dialect::Ros1 => "ros1".into(),
            Dialect::Ros2 => "ros2".into(),
            Dialect::Foxglove => "foxglove".into(),
            Dialect::Custom(name) => format!("custom:{name}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialect_serde_roundtrip() {
        for dialect in [
            Dialect::Ros1,
            Dialect::Ros2,
            Dialect::Foxglove,
            Dialect::Custom("mqtt".into()),
        ] {
            let json = serde_json::to_string(&dialect).unwrap();
            let decoded: Dialect = serde_json::from_str(&json).unwrap();
            assert_eq!(dialect, decoded);
        }
    }
}
