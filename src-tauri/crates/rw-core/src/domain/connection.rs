use crate::ids::ConnectionId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    FoxgloveWs,
    Rosbridge,
    NativeRos2,
    Dummy,
}

impl TransportKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TransportKind::FoxgloveWs => "foxglove_ws",
            TransportKind::Rosbridge => "rosbridge",
            TransportKind::NativeRos2 => "native_ros2",
            TransportKind::Dummy => "dummy",
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match input {
            "foxglove_ws" => Some(TransportKind::FoxgloveWs),
            "rosbridge" => Some(TransportKind::Rosbridge),
            "native_ros2" => Some(TransportKind::NativeRos2),
            "dummy" => Some(TransportKind::Dummy),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransportConfig {
    FoxgloveWs {
        url: String,
        #[serde(default)]
        headers: Vec<(String, String)>,
    },
    Rosbridge {
        url: String,
    },
    NativeRos2 {
        domain_id: u32,
    },
    Dummy {},
}

impl TransportConfig {
    pub fn kind(&self) -> TransportKind {
        match self {
            TransportConfig::FoxgloveWs { .. } => TransportKind::FoxgloveWs,
            TransportConfig::Rosbridge { .. } => TransportKind::Rosbridge,
            TransportConfig::NativeRos2 { .. } => TransportKind::NativeRos2,
            TransportConfig::Dummy {} => TransportKind::Dummy,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    pub id: ConnectionId,
    pub name: String,
    pub config: TransportConfig,
    pub auto_connect: bool,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Connection {
    pub fn transport_kind(&self) -> TransportKind {
        self.config.kind()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn transport_kind_serializes_snake_case() {
        let json = serde_json::to_string(&TransportKind::FoxgloveWs).unwrap();
        assert_eq!(json, r#""foxglove_ws""#);
    }

    #[test]
    fn transport_kind_parse_is_inverse_of_as_str() {
        for kind in [
            TransportKind::FoxgloveWs,
            TransportKind::Rosbridge,
            TransportKind::NativeRos2,
        ] {
            assert_eq!(TransportKind::parse(kind.as_str()), Some(kind));
        }
    }

    #[test]
    fn transport_config_round_trips_through_json() {
        let config = TransportConfig::FoxgloveWs {
            url: "ws://localhost:8765".into(),
            headers: vec![("X-Token".into(), "secret".into())],
        };
        let json = serde_json::to_string(&config).unwrap();
        let decoded: TransportConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, config);
        assert_eq!(decoded.kind(), TransportKind::FoxgloveWs);
    }

    #[test]
    fn connection_round_trips_through_json() {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let connection = Connection {
            id: 1,
            name: "Lab Robot".into(),
            config: TransportConfig::FoxgloveWs {
                url: "ws://192.168.1.10:8765".into(),
                headers: vec![],
            },
            auto_connect: true,
            color: Some("#00aaff".into()),
            created_at: now,
            updated_at: now,
        };

        let json = serde_json::to_string(&connection).unwrap();
        let decoded: Connection = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, connection);
        assert_eq!(decoded.transport_kind(), TransportKind::FoxgloveWs);
    }
}
