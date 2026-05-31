use std::collections::BTreeMap;

use rw_canonical::{CanonicalValue, FieldType, MessageDef};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionGoalId([u8; 16]);

impl ActionGoalId {
    pub fn new_v4() -> Self {
        ActionGoalId(*Uuid::new_v4().as_bytes())
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        ActionGoalId(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 16];
        for (index, slot) in bytes.iter_mut().enumerate() {
            *slot = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).ok()?;
        }
        Some(ActionGoalId(bytes))
    }

    pub fn as_uuid_value(&self) -> CanonicalValue {
        CanonicalValue::Struct(BTreeMap::from([(
            "uuid".to_string(),
            CanonicalValue::Bytes(self.0.to_vec()),
        )]))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionTargets {
    pub send_goal: String,
    pub get_result: String,
    pub cancel_goal: String,
    pub feedback: String,
}

impl ActionTargets {
    pub fn for_action(action: &str) -> Self {
        let base = action.trim_end_matches('/');
        ActionTargets {
            send_goal: format!("{base}/_action/send_goal"),
            get_result: format!("{base}/_action/get_result"),
            cancel_goal: format!("{base}/_action/cancel_goal"),
            feedback: format!("{base}/_action/feedback"),
        }
    }
}

pub fn field<'a>(value: &'a CanonicalValue, name: &str) -> Option<&'a CanonicalValue> {
    match value {
        CanonicalValue::Struct(fields) => fields.get(name),
        _ => None,
    }
}

pub fn take_field(value: CanonicalValue, name: &str) -> Option<CanonicalValue> {
    match value {
        CanonicalValue::Struct(mut fields) => fields.remove(name),
        _ => None,
    }
}

pub fn goal_accepted(value: &CanonicalValue) -> bool {
    match field(value, "accepted") {
        Some(CanonicalValue::Bool(accepted)) => *accepted,
        _ => true,
    }
}

pub fn goal_uuid_matches(value: &CanonicalValue, wanted: &ActionGoalId) -> bool {
    let uuid = field(value, "goal_id").and_then(|id| field(id, "uuid"));
    match uuid {
        Some(CanonicalValue::Bytes(bytes)) => bytes.as_slice() == wanted.as_bytes(),
        Some(CanonicalValue::Array(items)) => {
            items.len() == wanted.as_bytes().len()
                && items
                    .iter()
                    .zip(wanted.as_bytes())
                    .all(|(item, expected)| {
                        matches!(item, CanonicalValue::Uint(byte) if *byte as u8 == *expected)
                    })
        }
        _ => false,
    }
}

pub fn shape_send_goal_request(
    request_schema: &MessageDef,
    goal_id: &ActionGoalId,
    goal: CanonicalValue,
) -> CanonicalValue {
    let mut fields = BTreeMap::new();
    fields.insert("goal_id".to_string(), goal_id.as_uuid_value());
    if goal_is_nested(request_schema) {
        fields.insert("goal".to_string(), goal);
    } else if let CanonicalValue::Struct(goal_fields) = goal {
        for (name, value) in goal_fields {
            if name != "goal_id" {
                fields.insert(name, value);
            }
        }
    }
    CanonicalValue::Struct(fields)
}

pub fn goal_result_request(goal_id: &ActionGoalId) -> CanonicalValue {
    CanonicalValue::Struct(BTreeMap::from([(
        "goal_id".to_string(),
        goal_id.as_uuid_value(),
    )]))
}

pub fn cancel_request(goal_id: &ActionGoalId) -> CanonicalValue {
    let goal_info = CanonicalValue::Struct(BTreeMap::from([
        ("goal_id".to_string(), goal_id.as_uuid_value()),
        (
            "stamp".to_string(),
            CanonicalValue::Time { sec: 0, nanosec: 0 },
        ),
    ]));
    CanonicalValue::Struct(BTreeMap::from([("goal_info".to_string(), goal_info)]))
}

fn goal_is_nested(request_schema: &MessageDef) -> bool {
    request_schema
        .fields
        .iter()
        .any(|field| field.name == "goal" && matches!(field.field_type, FieldType::Complex { .. }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rw_canonical::{FieldDef, PrimitiveType};

    fn goal_with_order(order: i64) -> CanonicalValue {
        CanonicalValue::Struct(BTreeMap::from([(
            "order".to_string(),
            CanonicalValue::Int(order),
        )]))
    }

    #[test]
    fn goal_id_hex_round_trips() {
        let goal_id =
            ActionGoalId::from_bytes([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 0xff]);
        let hex = goal_id.to_hex();
        assert_eq!(hex, "000102030405060708090a0b0c0d0eff");
        assert_eq!(ActionGoalId::from_hex(&hex), Some(goal_id));
        assert_eq!(ActionGoalId::from_hex("nothex"), None);
        assert_eq!(ActionGoalId::from_hex(""), None);
    }

    #[test]
    fn new_v4_is_unique_and_carries_into_a_uuid_struct() {
        let first = ActionGoalId::new_v4();
        let second = ActionGoalId::new_v4();
        assert_ne!(first, second);
        assert_eq!(
            first.as_uuid_value(),
            CanonicalValue::Struct(BTreeMap::from([(
                "uuid".to_string(),
                CanonicalValue::Bytes(first.as_bytes().to_vec()),
            )]))
        );
    }

    #[test]
    fn targets_derive_the_ros2_service_and_topic_names() {
        let targets = ActionTargets::for_action("/fibonacci");
        assert_eq!(targets.send_goal, "/fibonacci/_action/send_goal");
        assert_eq!(targets.get_result, "/fibonacci/_action/get_result");
        assert_eq!(targets.cancel_goal, "/fibonacci/_action/cancel_goal");
        assert_eq!(targets.feedback, "/fibonacci/_action/feedback");
        assert_eq!(
            ActionTargets::for_action("/fibonacci/").send_goal,
            "/fibonacci/_action/send_goal"
        );
    }

    #[test]
    fn field_helpers_return_none_outside_a_struct() {
        let value = CanonicalValue::Struct(BTreeMap::from([(
            "feedback".to_string(),
            CanonicalValue::Int(42),
        )]));
        assert_eq!(field(&value, "feedback"), Some(&CanonicalValue::Int(42)));
        assert_eq!(field(&value, "missing"), None);
        assert_eq!(field(&CanonicalValue::Int(7), "feedback"), None);

        assert_eq!(
            take_field(value.clone(), "feedback"),
            Some(CanonicalValue::Int(42))
        );
        assert_eq!(take_field(value, "missing"), None);
        assert_eq!(take_field(CanonicalValue::Int(7), "feedback"), None);
    }

    #[test]
    fn goal_accepted_defaults_to_true_without_an_accepted_field() {
        let rejected = CanonicalValue::Struct(BTreeMap::from([(
            "accepted".to_string(),
            CanonicalValue::Bool(false),
        )]));
        assert!(!goal_accepted(&rejected));
        assert!(goal_accepted(&CanonicalValue::Null));
    }

    #[test]
    fn goal_uuid_matches_handles_bytes_and_array_uuids() {
        let goal_id = ActionGoalId::from_bytes([3u8; 16]);
        let bytes_message = CanonicalValue::Struct(BTreeMap::from([(
            "goal_id".to_string(),
            goal_id.as_uuid_value(),
        )]));
        assert!(goal_uuid_matches(&bytes_message, &goal_id));
        assert!(!goal_uuid_matches(
            &bytes_message,
            &ActionGoalId::from_bytes([4u8; 16])
        ));

        let array_uuid = CanonicalValue::Array(
            goal_id
                .as_bytes()
                .iter()
                .map(|byte| CanonicalValue::Uint(*byte as u64))
                .collect(),
        );
        let array_message = CanonicalValue::Struct(BTreeMap::from([(
            "goal_id".to_string(),
            CanonicalValue::Struct(BTreeMap::from([("uuid".to_string(), array_uuid)])),
        )]));
        assert!(goal_uuid_matches(&array_message, &goal_id));
        assert!(!goal_uuid_matches(&CanonicalValue::Null, &goal_id));
    }

    fn goal_id_field() -> FieldDef {
        FieldDef {
            name: "goal_id".into(),
            field_type: FieldType::Complex {
                type_name: "unique_identifier_msgs/UUID".into(),
            },
            default: None,
            comment: None,
        }
    }

    #[test]
    fn shape_nests_the_goal_when_the_schema_declares_a_complex_goal_field() {
        let send_goal_def = MessageDef {
            fields: vec![
                goal_id_field(),
                FieldDef {
                    name: "goal".into(),
                    field_type: FieldType::Complex {
                        type_name: "example_interfaces/Fibonacci_Goal".into(),
                    },
                    default: None,
                    comment: None,
                },
            ],
            constants: vec![],
        };
        let goal_id = ActionGoalId::from_bytes([1u8; 16]);
        let request = shape_send_goal_request(&send_goal_def, &goal_id, goal_with_order(5));
        assert_eq!(
            field(&request, "goal_id").and_then(|id| field(id, "uuid")),
            Some(&CanonicalValue::Bytes(goal_id.as_bytes().to_vec()))
        );
        assert_eq!(
            field(&request, "goal").and_then(|goal| field(goal, "order")),
            Some(&CanonicalValue::Int(5))
        );
        assert!(field(&request, "order").is_none());
    }

    #[test]
    fn shape_flattens_the_goal_when_the_schema_inlines_its_fields() {
        let send_goal_def = MessageDef {
            fields: vec![
                goal_id_field(),
                FieldDef {
                    name: "order".into(),
                    field_type: FieldType::Primitive(PrimitiveType::Int32),
                    default: None,
                    comment: None,
                },
            ],
            constants: vec![],
        };
        let goal_id = ActionGoalId::from_bytes([2u8; 16]);
        let request = shape_send_goal_request(&send_goal_def, &goal_id, goal_with_order(9));
        assert_eq!(field(&request, "order"), Some(&CanonicalValue::Int(9)));
        assert!(field(&request, "goal").is_none());
        assert!(field(&request, "goal_id").is_some());
    }

    #[test]
    fn result_and_cancel_requests_carry_the_goal_id() {
        let goal_id = ActionGoalId::from_bytes([7u8; 16]);
        let result_request = goal_result_request(&goal_id);
        assert!(field(&result_request, "goal_id").is_some());
        assert!(field(&result_request, "goal").is_none());

        let cancel = cancel_request(&goal_id);
        let goal_info = field(&cancel, "goal_info").expect("goal_info");
        assert!(field(goal_info, "goal_id").is_some());
        assert!(matches!(
            field(goal_info, "stamp"),
            Some(CanonicalValue::Time { .. })
        ));
    }
}
