use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "op")]
pub enum ClientOp {
    #[serde(rename = "subscribe")]
    Subscribe {
        topic: String,
        #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
        type_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        throttle_rate: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        queue_length: Option<u32>,
    },
    #[allow(dead_code)]
    #[serde(rename = "unsubscribe")]
    Unsubscribe { topic: String },
    #[serde(rename = "publish")]
    Publish { topic: String, msg: JsonValue },
    #[serde(rename = "call_service")]
    CallService {
        id: String,
        service: String,
        args: JsonValue,
    },
    #[serde(rename = "send_action_goal")]
    SendActionGoal {
        id: String,
        action: String,
        action_type: String,
        args: JsonValue,
        feedback: bool,
    },
    #[serde(rename = "cancel_action_goal")]
    CancelActionGoal { id: String, action: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op")]
pub enum ServerOp {
    #[serde(rename = "publish")]
    Publish { topic: String, msg: JsonValue },
    #[serde(rename = "service_response")]
    ServiceResponse {
        #[serde(default)]
        id: String,
        #[allow(dead_code)]
        #[serde(default)]
        service: String,
        #[serde(default)]
        values: JsonValue,
        #[serde(default)]
        result: Option<bool>,
    },
    #[serde(rename = "action_feedback")]
    ActionFeedback {
        #[serde(default)]
        id: String,
        #[serde(default)]
        values: JsonValue,
    },
    #[serde(rename = "action_result")]
    ActionResult {
        #[serde(default)]
        id: String,
        #[serde(default)]
        values: JsonValue,
        #[allow(dead_code)]
        #[serde(default)]
        status: JsonValue,
        #[serde(default)]
        result: Option<bool>,
    },
    #[serde(other)]
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_serialises_with_type() {
        let op = ClientOp::Subscribe {
            topic: "/scan".into(),
            type_name: Some("sensor_msgs/LaserScan".into()),
            throttle_rate: None,
            queue_length: None,
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains(r#""op":"subscribe""#));
        assert!(json.contains(r#""topic":"/scan""#));
        assert!(json.contains(r#""type":"sensor_msgs/LaserScan""#));
        assert!(!json.contains("throttle_rate"));
        assert!(!json.contains("queue_length"));
    }

    #[test]
    fn subscribe_serialises_with_throttle() {
        let op = ClientOp::Subscribe {
            topic: "/scan".into(),
            type_name: None,
            throttle_rate: Some(5),
            queue_length: Some(4),
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains(r#""throttle_rate":5"#));
        assert!(json.contains(r#""queue_length":4"#));
    }

    #[test]
    fn call_service_serialises_with_id() {
        let op = ClientOp::CallService {
            id: "rw-call-1".into(),
            service: "/rosapi/topics".into(),
            args: serde_json::json!({}),
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains(r#""op":"call_service""#));
        assert!(json.contains(r#""id":"rw-call-1""#));
    }

    #[test]
    fn service_response_parses() {
        let json = r#"{"op":"service_response","id":"x","service":"/rosapi/topics","values":{"topics":["/a"],"types":["std_msgs/String"]},"result":true}"#;
        let parsed: ServerOp = serde_json::from_str(json).unwrap();
        match parsed {
            ServerOp::ServiceResponse {
                id, values, result, ..
            } => {
                assert_eq!(id, "x");
                assert_eq!(result, Some(true));
                assert!(values.get("topics").is_some());
            }
            _ => panic!("expected service_response"),
        }
    }

    #[test]
    fn send_action_goal_serialises_with_all_fields() {
        let op = ClientOp::SendActionGoal {
            id: "fib::42".into(),
            action: "/fibonacci".into(),
            action_type: "action_tutorials_interfaces/Fibonacci".into(),
            args: serde_json::json!({"order": 5}),
            feedback: true,
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains(r#""op":"send_action_goal""#));
        assert!(json.contains(r#""action":"/fibonacci""#));
        assert!(json.contains(r#""action_type":"action_tutorials_interfaces/Fibonacci""#));
        assert!(json.contains(r#""id":"fib::42""#));
        assert!(json.contains(r#""feedback":true"#));
    }

    #[test]
    fn cancel_action_goal_serialises() {
        let op = ClientOp::CancelActionGoal {
            id: "fib::42".into(),
            action: "/fibonacci".into(),
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains(r#""op":"cancel_action_goal""#));
        assert!(json.contains(r#""id":"fib::42""#));
        assert!(json.contains(r#""action":"/fibonacci""#));
    }

    #[test]
    fn action_feedback_parses() {
        let parsed: ServerOp = serde_json::from_str(
            r#"{"op":"action_feedback","id":"fib::42","values":{"partial_sequence":[0,1,1]}}"#,
        )
        .unwrap();
        match parsed {
            ServerOp::ActionFeedback { id, values } => {
                assert_eq!(id, "fib::42");
                assert!(values.get("partial_sequence").is_some());
            }
            _ => panic!("expected action_feedback"),
        }
    }

    #[test]
    fn action_result_parses() {
        let parsed: ServerOp = serde_json::from_str(
            r#"{"op":"action_result","id":"fib::42","values":{"sequence":[0,1,1,2,3,5]},"status":4,"result":true}"#,
        )
        .unwrap();
        match parsed {
            ServerOp::ActionResult {
                id, values, result, ..
            } => {
                assert_eq!(id, "fib::42");
                assert_eq!(result, Some(true));
                assert!(values.get("sequence").is_some());
            }
            _ => panic!("expected action_result"),
        }
    }

    #[test]
    fn unknown_op_lands_in_other() {
        let parsed: ServerOp =
            serde_json::from_str(r#"{"op":"status","level":"info","msg":"hi"}"#).unwrap();
        assert!(matches!(parsed, ServerOp::Other));
    }

    #[test]
    fn publish_serialises_with_topic_and_msg() {
        let op = ClientOp::Publish {
            topic: "/scan".into(),
            msg: serde_json::json!({"data": "hello"}),
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains(r#""op":"publish""#));
        assert!(json.contains(r#""topic":"/scan""#));
        assert!(json.contains(r#""data":"hello""#));
    }

    #[test]
    fn publish_parses() {
        let parsed: ServerOp =
            serde_json::from_str(r#"{"op":"publish","topic":"/chat","msg":{"data":"hello"}}"#)
                .unwrap();
        match parsed {
            ServerOp::Publish { topic, msg } => {
                assert_eq!(topic, "/chat");
                assert_eq!(msg["data"], "hello");
            }
            _ => panic!(),
        }
    }
}
