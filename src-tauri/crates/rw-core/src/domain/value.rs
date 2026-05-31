use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
#[derive(Default)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Int(i64),
    Uint(u64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Struct(BTreeMap<String, Value>),
    Time {
        sec: i32,
        nanosec: u32,
    },
    Duration {
        sec: i32,
        nanosec: u32,
    },
}

impl Value {
    pub fn empty_struct() -> Self {
        Value::Struct(BTreeMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(value: &Value) -> Value {
        let json = serde_json::to_string(value).expect("serialize value");
        serde_json::from_str(&json).expect("deserialize value")
    }

    #[test]
    fn null_round_trips() {
        assert_eq!(round_trip(&Value::Null), Value::Null);
    }

    #[test]
    fn primitives_round_trip() {
        assert_eq!(round_trip(&Value::Bool(true)), Value::Bool(true));
        assert_eq!(round_trip(&Value::Int(-42)), Value::Int(-42));
        assert_eq!(round_trip(&Value::Uint(42)), Value::Uint(42));
        assert_eq!(round_trip(&Value::F64(2.5)), Value::F64(2.5));
        assert_eq!(
            round_trip(&Value::String("hello".into())),
            Value::String("hello".into())
        );
    }

    #[test]
    fn time_and_duration_round_trip() {
        let time = Value::Time {
            sec: 1735689600,
            nanosec: 123_456_789,
        };
        let duration = Value::Duration {
            sec: 60,
            nanosec: 0,
        };
        assert_eq!(round_trip(&time), time);
        assert_eq!(round_trip(&duration), duration);
    }

    #[test]
    fn nested_struct_round_trips() {
        let mut inner = BTreeMap::new();
        inner.insert("x".into(), Value::F64(1.0));
        inner.insert("y".into(), Value::F64(2.0));

        let mut outer = BTreeMap::new();
        outer.insert("position".into(), Value::Struct(inner));
        outer.insert(
            "tags".into(),
            Value::Array(vec![Value::String("a".into()), Value::String("b".into())]),
        );
        outer.insert("data".into(), Value::Bytes(vec![1, 2, 3]));

        let value = Value::Struct(outer);
        assert_eq!(round_trip(&value), value);
    }

    #[test]
    fn json_uses_kind_tag() {
        let json = serde_json::to_string(&Value::Bool(true)).unwrap();
        assert_eq!(json, r#"{"kind":"bool","value":true}"#);

        let json = serde_json::to_string(&Value::Null).unwrap();
        assert_eq!(json, r#"{"kind":"null"}"#);
    }

    use proptest::prelude::*;

    fn value_strategy() -> impl Strategy<Value = Value> {
        let leaf = prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<i64>().prop_map(Value::Int),
            any::<u64>().prop_map(Value::Uint),
            "[a-zA-Z0-9 ]{0,32}".prop_map(Value::String),
            prop::collection::vec(any::<u8>(), 0..16).prop_map(Value::Bytes),
            (any::<i32>(), any::<u32>()).prop_map(|(sec, nanosec)| Value::Time { sec, nanosec }),
            (any::<i32>(), any::<u32>())
                .prop_map(|(sec, nanosec)| Value::Duration { sec, nanosec }),
        ];

        leaf.prop_recursive(3, 16, 4, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..4).prop_map(Value::Array),
                prop::collection::btree_map("[a-z]{1,8}", inner, 0..4).prop_map(Value::Struct),
            ]
        })
    }

    proptest! {
        #[test]
        fn arbitrary_value_round_trips_through_json(value in value_strategy()) {
            let json = serde_json::to_string(&value).unwrap();
            let decoded: Value = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(decoded, value);
        }
    }
}
