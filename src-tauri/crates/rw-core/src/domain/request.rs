use crate::domain::value::Value;
use crate::ids::{CollectionId, ConnectionId, RequestId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaRef {
    pub name: String,
    pub hash: String,
}

pub type Visualization = serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RequestKind {
    Topic,
    Service,
    Action,
}

impl RequestKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RequestKind::Topic => "topic",
            RequestKind::Service => "service",
            RequestKind::Action => "action",
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match input {
            "topic" => Some(RequestKind::Topic),
            "service" => Some(RequestKind::Service),
            "action" => Some(RequestKind::Action),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub id: RequestId,
    pub collection_id: Option<CollectionId>,
    pub connection_id: Option<ConnectionId>,
    pub name: String,
    pub kind: RequestKind,
    pub target: String,
    pub schema: Option<SchemaRef>,
    pub input: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visualization: Option<Visualization>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn request_kind_serializes_lowercase() {
        let json = serde_json::to_string(&RequestKind::Service).unwrap();
        assert_eq!(json, r#""service""#);

        let parsed: RequestKind = serde_json::from_str(r#""topic""#).unwrap();
        assert_eq!(parsed, RequestKind::Topic);
    }

    #[test]
    fn request_kind_parse_is_inverse_of_as_str() {
        for kind in [
            RequestKind::Topic,
            RequestKind::Service,
            RequestKind::Action,
        ] {
            assert_eq!(RequestKind::parse(kind.as_str()), Some(kind));
        }
    }

    #[test]
    fn request_round_trips_through_json() {
        let now = Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap();
        let request = Request {
            id: 1,
            collection_id: Some(2),
            connection_id: Some(3),
            name: "Scan".into(),
            kind: RequestKind::Topic,
            target: "/scan".into(),
            schema: Some(SchemaRef {
                name: "sensor_msgs/msg/LaserScan".into(),
                hash: "abc123".into(),
            }),
            input: Value::empty_struct(),
            visualization: None,
            created_at: now,
            updated_at: now,
        };

        let json = serde_json::to_string(&request).unwrap();
        let decoded: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, request);
    }
}
