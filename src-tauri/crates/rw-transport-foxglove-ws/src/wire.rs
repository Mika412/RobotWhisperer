use bytes::Bytes;
use serde::{Deserialize, Serialize};

pub const SUBPROTOCOL_ROS2: &str = "foxglove.sdk.v1";
pub const SUBPROTOCOL_ROS1: &str = "foxglove.websocket.v1";
pub const SUBPROTOCOLS: &[&str] = &[SUBPROTOCOL_ROS2, SUBPROTOCOL_ROS1];

pub const OPCODE_MESSAGE_DATA: u8 = 0x01;
pub const OPCODE_TIME: u8 = 0x02;
pub const OPCODE_SERVICE_CALL_RESPONSE: u8 = 0x03;

pub const OPCODE_CLIENT_SERVICE_CALL_REQUEST: u8 = 0x02;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op")]
pub enum ServerMessage {
    #[serde(rename = "serverInfo")]
    ServerInfo {
        name: String,
        #[serde(default)]
        capabilities: Vec<String>,
        #[serde(default, rename = "supportedEncodings")]
        supported_encodings: Vec<String>,
        #[serde(default, rename = "sessionId")]
        session_id: Option<String>,
    },
    #[serde(rename = "advertise")]
    Advertise { channels: Vec<AdvertisedChannel> },
    #[serde(rename = "unadvertise")]
    Unadvertise {
        #[serde(rename = "channelIds")]
        channel_ids: Vec<u32>,
    },
    #[serde(rename = "advertiseServices")]
    AdvertiseServices { services: Vec<AdvertisedServiceRaw> },
    #[serde(rename = "unadvertiseServices")]
    UnadvertiseServices {
        #[serde(rename = "serviceIds")]
        service_ids: Vec<u32>,
    },
    #[serde(rename = "status")]
    Status {
        #[serde(default)]
        level: u8,
        #[serde(default)]
        message: String,
    },
    #[serde(other)]
    Unknown,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct AdvertisedServiceRaw {
    pub id: u32,
    pub name: String,
    #[serde(default, rename = "type")]
    pub type_name: String,

    #[serde(default)]
    pub request: Option<AdvertisedServiceHalf>,
    #[serde(default)]
    pub response: Option<AdvertisedServiceHalf>,

    #[serde(default, rename = "requestSchema")]
    pub request_schema_flat: String,
    #[serde(default, rename = "responseSchema")]
    pub response_schema_flat: String,
    #[serde(default, rename = "requestSchemaName")]
    pub request_schema_name_flat: String,
    #[serde(default, rename = "responseSchemaName")]
    pub response_schema_name_flat: String,
    #[serde(default)]
    pub encoding: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct AdvertisedServiceHalf {
    #[serde(default)]
    pub encoding: String,
    #[serde(default, rename = "schemaName")]
    pub schema_name: String,
    #[serde(default)]
    pub schema: String,
    #[serde(default, rename = "schemaEncoding")]
    pub schema_encoding: String,
}

#[derive(Debug, Clone)]
pub struct AdvertisedService {
    pub id: u32,
    pub name: String,
    pub type_name: String,
    pub request_schema: String,
    pub response_schema: String,
    pub request_schema_name: String,
    pub response_schema_name: String,
    pub encoding: String,
}

impl AdvertisedServiceRaw {
    pub fn normalised(self) -> AdvertisedService {
        let (req_schema, req_name, req_encoding) = self
            .request
            .map(|h| (h.schema, h.schema_name, h.encoding))
            .filter(|(s, n, _)| !s.is_empty() || !n.is_empty())
            .unwrap_or((
                self.request_schema_flat,
                self.request_schema_name_flat,
                String::new(),
            ));
        let (res_schema, res_name, res_encoding) = self
            .response
            .map(|h| (h.schema, h.schema_name, h.encoding))
            .filter(|(s, n, _)| !s.is_empty() || !n.is_empty())
            .unwrap_or((
                self.response_schema_flat,
                self.response_schema_name_flat,
                String::new(),
            ));
        let encoding = if !req_encoding.is_empty() {
            req_encoding
        } else if !res_encoding.is_empty() {
            res_encoding
        } else {
            self.encoding
        };
        AdvertisedService {
            id: self.id,
            name: self.name,
            type_name: self.type_name,
            request_schema: req_schema,
            response_schema: res_schema,
            request_schema_name: req_name,
            response_schema_name: res_name,
            encoding,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct AdvertisedChannel {
    pub id: u32,
    pub topic: String,
    #[serde(default)]
    pub encoding: String,
    #[serde(rename = "schemaName")]
    pub schema_name: String,
    #[serde(default)]
    pub schema: String,
    #[serde(default, rename = "schemaEncoding")]
    pub schema_encoding: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "op")]
pub enum ClientMessage {
    #[serde(rename = "subscribe")]
    Subscribe {
        subscriptions: Vec<SubscriptionRequest>,
    },
    #[serde(rename = "unsubscribe")]
    Unsubscribe {
        #[serde(rename = "subscriptionIds")]
        subscription_ids: Vec<u32>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct SubscriptionRequest {
    pub id: u32,
    #[serde(rename = "channelId")]
    pub channel_id: u32,
}

#[derive(Debug, Clone)]
pub struct MessageData {
    pub subscription_id: u32,
    pub timestamp_ns: u64,
    pub payload: Bytes,
}

#[derive(Debug, Clone)]
pub struct ServiceCallResponseFrame {
    #[allow(dead_code)]
    pub service_id: u32,
    pub call_id: u32,
    pub encoding: String,
    pub payload: Bytes,
}

pub fn pack_service_call_request(
    service_id: u32,
    call_id: u32,
    encoding: &str,
    payload: &[u8],
) -> Vec<u8> {
    let encoding_bytes = encoding.as_bytes();
    let mut out = Vec::with_capacity(1 + 4 + 4 + 4 + encoding_bytes.len() + payload.len());
    out.push(OPCODE_CLIENT_SERVICE_CALL_REQUEST);
    out.extend_from_slice(&service_id.to_le_bytes());
    out.extend_from_slice(&call_id.to_le_bytes());
    out.extend_from_slice(&(encoding_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(encoding_bytes);
    out.extend_from_slice(payload);
    out
}

pub fn parse_service_call_response(bytes: &[u8]) -> Result<ServiceCallResponseFrame, WireError> {
    if bytes.is_empty() {
        return Err(WireError::Empty);
    }
    if bytes[0] != OPCODE_SERVICE_CALL_RESPONSE {
        return Err(WireError::UnknownOpcode(bytes[0]));
    }
    if bytes.len() < 1 + 4 + 4 + 4 {
        return Err(WireError::TooShort(bytes.len()));
    }
    let service_id = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
    let call_id = u32::from_le_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);
    let encoding_len = u32::from_le_bytes([bytes[9], bytes[10], bytes[11], bytes[12]]) as usize;
    let encoding_start = 13;
    let encoding_end = encoding_start + encoding_len;
    if bytes.len() < encoding_end {
        return Err(WireError::TooShort(bytes.len()));
    }
    let encoding = String::from_utf8_lossy(&bytes[encoding_start..encoding_end]).into_owned();
    let payload = Bytes::copy_from_slice(&bytes[encoding_end..]);
    Ok(ServiceCallResponseFrame {
        service_id,
        call_id,
        encoding,
        payload,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum WireError {
    #[error("empty binary frame")]
    Empty,
    #[error("unsupported foxglove binary opcode 0x{0:02x}")]
    UnknownOpcode(u8),
    #[error("binary frame too short: {0} bytes, need at least 13")]
    TooShort(usize),
}

pub fn parse_binary_frame(bytes: &[u8]) -> Result<MessageData, WireError> {
    if bytes.is_empty() {
        return Err(WireError::Empty);
    }
    if bytes[0] != OPCODE_MESSAGE_DATA {
        return Err(WireError::UnknownOpcode(bytes[0]));
    }
    if bytes.len() < 13 {
        return Err(WireError::TooShort(bytes.len()));
    }
    let subscription_id = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
    let timestamp_ns = u64::from_le_bytes([
        bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12],
    ]);
    Ok(MessageData {
        subscription_id,
        timestamp_ns,
        payload: Bytes::copy_from_slice(&bytes[13..]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_frame_parses() {
        let mut bytes = vec![OPCODE_MESSAGE_DATA];
        bytes.extend_from_slice(&42u32.to_le_bytes());
        bytes.extend_from_slice(&100u64.to_le_bytes());
        bytes.extend_from_slice(b"payload");
        let parsed = parse_binary_frame(&bytes).unwrap();
        assert_eq!(parsed.subscription_id, 42);
        assert_eq!(parsed.timestamp_ns, 100);
        assert_eq!(parsed.payload.as_ref(), b"payload");
    }

    #[test]
    fn service_call_request_pack_round_trips() {
        let packed = pack_service_call_request(42, 7, "cdr", &[1, 2, 3]);
        assert_eq!(packed[0], OPCODE_CLIENT_SERVICE_CALL_REQUEST);
        let mut response_bytes = packed.clone();
        response_bytes[0] = OPCODE_SERVICE_CALL_RESPONSE;
        let resp = parse_service_call_response(&response_bytes).unwrap();
        assert_eq!(resp.service_id, 42);
        assert_eq!(resp.call_id, 7);
        assert_eq!(resp.encoding, "cdr");
        assert_eq!(resp.payload.as_ref(), &[1, 2, 3]);
    }

    #[test]
    fn advertise_services_parses() {
        let json = r#"{
            "op": "advertiseServices",
            "services": [
                {
                    "id": 1,
                    "name": "/add_two_ints",
                    "type": "example_interfaces/srv/AddTwoInts",
                    "requestSchema": "int64 a\nint64 b\n",
                    "responseSchema": "int64 sum\n",
                    "requestSchemaName": "example_interfaces/srv/AddTwoInts_Request",
                    "responseSchemaName": "example_interfaces/srv/AddTwoInts_Response",
                    "encoding": "cdr"
                }
            ]
        }"#;
        let parsed: ServerMessage = serde_json::from_str(json).unwrap();
        match parsed {
            ServerMessage::AdvertiseServices { services } => {
                assert_eq!(services.len(), 1);
                let svc = services.into_iter().next().unwrap().normalised();
                assert_eq!(svc.name, "/add_two_ints");
                assert_eq!(svc.id, 1);
                assert!(svc.request_schema.contains("int64 a"));
                assert_eq!(
                    svc.request_schema_name,
                    "example_interfaces/srv/AddTwoInts_Request"
                );
            }
            _ => panic!("expected advertiseServices"),
        }
    }

    #[test]
    fn advertise_services_parses_nested_modern_shape() {
        let json = r#"{
            "op": "advertiseServices",
            "services": [
                {
                    "id": 7,
                    "name": "/add_two_ints",
                    "type": "example_interfaces/srv/AddTwoInts",
                    "request": {
                        "encoding": "cdr",
                        "schemaName": "example_interfaces/srv/AddTwoInts_Request",
                        "schemaEncoding": "ros2msg",
                        "schema": "int64 a\nint64 b"
                    },
                    "response": {
                        "encoding": "cdr",
                        "schemaName": "example_interfaces/srv/AddTwoInts_Response",
                        "schemaEncoding": "ros2msg",
                        "schema": "int64 sum"
                    }
                }
            ]
        }"#;
        let parsed: ServerMessage = serde_json::from_str(json).unwrap();
        match parsed {
            ServerMessage::AdvertiseServices { services } => {
                let svc = services.into_iter().next().unwrap().normalised();
                assert_eq!(svc.id, 7);
                assert_eq!(svc.encoding, "cdr");
                assert!(svc.request_schema.contains("int64 a"));
                assert!(svc.request_schema.contains("int64 b"));
                assert!(svc.response_schema.contains("int64 sum"));
                assert_eq!(
                    svc.request_schema_name,
                    "example_interfaces/srv/AddTwoInts_Request"
                );
                assert_eq!(
                    svc.response_schema_name,
                    "example_interfaces/srv/AddTwoInts_Response"
                );
            }
            _ => panic!("expected advertiseServices"),
        }
    }

    #[test]
    fn unknown_op_lands_in_unknown_variant() {
        let json = r#"{"op": "subscribeConnectionGraph"}"#;
        let parsed: ServerMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(parsed, ServerMessage::Unknown));
    }

    #[test]
    fn subscribe_serialises_with_channel_id() {
        let message = ClientMessage::Subscribe {
            subscriptions: vec![SubscriptionRequest {
                id: 1,
                channel_id: 7,
            }],
        };
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains(r#""channelId":7"#));
        assert!(json.contains(r#""op":"subscribe""#));
    }
}
