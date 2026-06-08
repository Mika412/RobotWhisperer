use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
#[derive(Default)]
pub enum CanonicalValue {
    #[default]
    Null,
    Bool(bool),
    Int(i64),
    Uint(u64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<CanonicalValue>),
    Struct(BTreeMap<String, CanonicalValue>),
    Time {
        sec: i32,
        nanosec: u32,
    },
    Duration {
        sec: i32,
        nanosec: u32,
    },
}

impl CanonicalValue {
    pub fn empty_struct() -> Self {
        CanonicalValue::Struct(BTreeMap::new())
    }

    pub fn get_path(&self, path: &str) -> Option<&CanonicalValue> {
        let mut current = self;
        for segment in path.split('.') {
            if segment.is_empty() {
                return None;
            }
            current = match current {
                CanonicalValue::Struct(fields) => fields.get(segment)?,
                CanonicalValue::Array(items) => {
                    let index: usize = segment.parse().ok()?;
                    items.get(index)?
                }
                _ => return None,
            };
        }
        Some(current)
    }

    pub fn project(&self, keep: &[String]) -> CanonicalValue {
        if keep.is_empty() {
            return self.clone();
        }
        self.project_at("", keep)
    }

    fn project_at(&self, prefix: &str, keep: &[String]) -> CanonicalValue {
        match self {
            CanonicalValue::Struct(fields) => {
                let mut out = BTreeMap::new();
                for (name, child) in fields {
                    let child_path = join_path(prefix, name);
                    if path_kept(&child_path, keep) {
                        out.insert(name.clone(), child.project_at(&child_path, keep));
                    }
                }
                CanonicalValue::Struct(out)
            }
            CanonicalValue::Array(items) => {
                CanonicalValue::Array(items.iter().map(|it| it.project_at(prefix, keep)).collect())
            }
            other => other.clone(),
        }
    }
}

fn join_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}.{name}")
    }
}

fn path_kept(path: &str, keep: &[String]) -> bool {
    keep.iter().any(|k| {
        k == path || k.starts_with(&format!("{path}.")) || path.starts_with(&format!("{k}."))
    })
}

#[derive(Debug)]
pub struct ProjectedValue<'a> {
    value: &'a CanonicalValue,
    keep: &'a [String],
    prefix: String,
}

impl<'a> ProjectedValue<'a> {
    pub fn new(value: &'a CanonicalValue, keep: &'a [String]) -> Self {
        ProjectedValue {
            value,
            keep,
            prefix: String::new(),
        }
    }
}

impl serde::Serialize for ProjectedValue<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self.value {
            CanonicalValue::Struct(fields) => {
                let mut m = s.serialize_map(Some(2))?;
                m.serialize_entry("kind", "struct")?;
                m.serialize_entry(
                    "value",
                    &ProjectedStruct {
                        fields,
                        keep: self.keep,
                        prefix: &self.prefix,
                    },
                )?;
                m.end()
            }
            CanonicalValue::Array(items) => {
                let mut m = s.serialize_map(Some(2))?;
                m.serialize_entry("kind", "array")?;
                m.serialize_entry(
                    "value",
                    &ProjectedSeq {
                        items,
                        keep: self.keep,
                        prefix: &self.prefix,
                    },
                )?;
                m.end()
            }
            other => other.serialize(s),
        }
    }
}

struct ProjectedStruct<'a> {
    fields: &'a BTreeMap<String, CanonicalValue>,
    keep: &'a [String],
    prefix: &'a str,
}

impl serde::Serialize for ProjectedStruct<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let kept: Vec<(&String, &CanonicalValue, String)> = self
            .fields
            .iter()
            .filter_map(|(name, child)| {
                let child_path = join_path(self.prefix, name);
                if path_kept(&child_path, self.keep) {
                    Some((name, child, child_path))
                } else {
                    None
                }
            })
            .collect();
        let mut m = s.serialize_map(Some(kept.len()))?;
        for (name, child, child_path) in &kept {
            m.serialize_entry(
                name,
                &ProjectedValue {
                    value: child,
                    keep: self.keep,
                    prefix: child_path.clone(),
                },
            )?;
        }
        m.end()
    }
}

struct ProjectedSeq<'a> {
    items: &'a [CanonicalValue],
    keep: &'a [String],
    prefix: &'a str,
}

impl serde::Serialize for ProjectedSeq<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = s.serialize_seq(Some(self.items.len()))?;
        for item in self.items {
            seq.serialize_element(&ProjectedValue {
                value: item,
                keep: self.keep,
                prefix: self.prefix.to_string(),
            })?;
        }
        seq.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(value: &CanonicalValue) -> CanonicalValue {
        let json = serde_json::to_string(value).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn primitives_roundtrip() {
        for value in [
            CanonicalValue::Null,
            CanonicalValue::Bool(true),
            CanonicalValue::Int(-42),
            CanonicalValue::Uint(42),
            CanonicalValue::F32(3.5),
            CanonicalValue::F64(2.5),
            CanonicalValue::String("hi".into()),
            CanonicalValue::Bytes(vec![1, 2, 3]),
            CanonicalValue::Time {
                sec: 1_700_000_000,
                nanosec: 123,
            },
            CanonicalValue::Duration {
                sec: 60,
                nanosec: 0,
            },
        ] {
            assert_eq!(roundtrip(&value), value);
        }
    }

    #[test]
    fn structs_roundtrip() {
        let mut inner = BTreeMap::new();
        inner.insert("x".into(), CanonicalValue::F64(1.0));
        inner.insert("y".into(), CanonicalValue::F64(2.0));
        let outer = CanonicalValue::Struct(BTreeMap::from([
            ("position".into(), CanonicalValue::Struct(inner)),
            (
                "tags".into(),
                CanonicalValue::Array(vec![
                    CanonicalValue::String("a".into()),
                    CanonicalValue::String("b".into()),
                ]),
            ),
        ]));
        assert_eq!(roundtrip(&outer), outer);
    }

    fn nested_like() -> CanonicalValue {
        let finger = |n: &str| {
            CanonicalValue::Struct(BTreeMap::from([
                ("name".into(), CanonicalValue::String(n.into())),
                (
                    "joint_name".into(),
                    CanonicalValue::Array(vec![CanonicalValue::String("J0".into())]),
                ),
                (
                    "joint_position".into(),
                    CanonicalValue::Array(vec![CanonicalValue::F64(0.5)]),
                ),
                (
                    "tactile_sensors".into(),
                    CanonicalValue::Array(vec![CanonicalValue::Struct(BTreeMap::from([(
                        "z".into(),
                        CanonicalValue::F64(-8.0),
                    )]))]),
                ),
                (
                    "actuator_position".into(),
                    CanonicalValue::Array(vec![CanonicalValue::F64(1.0); 4]),
                ),
            ]))
        };
        CanonicalValue::Struct(BTreeMap::from([
            (
                "header".into(),
                CanonicalValue::Struct(BTreeMap::from([(
                    "frame_id".into(),
                    CanonicalValue::String("rh_palm".into()),
                )])),
            ),
            (
                "finger_name".into(),
                CanonicalValue::Array(vec![CanonicalValue::String("index".into())]),
            ),
            (
                "finger".into(),
                CanonicalValue::Array(vec![finger("F0"), finger("F1")]),
            ),
        ]))
    }

    fn hand_keep() -> Vec<String> {
        [
            "header",
            "finger_name",
            "finger.name",
            "finger.joint_name",
            "finger.joint_position",
            "finger.tactile_sensors",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    #[test]
    fn project_keeps_joints_and_tactile_drops_actuators() {
        let pruned = nested_like().project(&hand_keep());
        assert!(pruned.get_path("finger.0.joint_position.0").is_some());
        assert!(pruned.get_path("finger.0.tactile_sensors.0.z").is_some());
        assert!(pruned.get_path("finger.0.actuator_position").is_none());
        assert!(pruned.get_path("header.frame_id").is_some());
    }

    #[test]
    fn project_empty_keep_is_identity() {
        let v = nested_like();
        assert_eq!(v.project(&[]), v);
    }

    #[test]
    fn projected_value_serialize_matches_clone_based_project() {
        let v = nested_like();
        let keep = hand_keep();
        let clone_json = serde_json::to_value(v.project(&keep)).unwrap();
        let free_json = serde_json::to_value(ProjectedValue::new(&v, &keep)).unwrap();
        assert_eq!(clone_json, free_json);
        let keep_all = vec!["header".into(), "finger_name".into(), "finger".into()];
        assert_eq!(
            serde_json::to_value(v.project(&keep_all)).unwrap(),
            serde_json::to_value(ProjectedValue::new(&v, &keep_all)).unwrap(),
        );
    }

    #[test]
    fn json_tag_matches_frontend_contract() {
        assert_eq!(
            serde_json::to_string(&CanonicalValue::Bool(true)).unwrap(),
            r#"{"kind":"bool","value":true}"#
        );
        assert_eq!(
            serde_json::to_string(&CanonicalValue::Null).unwrap(),
            r#"{"kind":"null"}"#
        );
    }

    #[test]
    fn get_path_traverses_struct_and_array() {
        let value: CanonicalValue = serde_json::from_value(serde_json::json!({
            "kind": "struct",
            "value": {
                "points": {
                    "kind": "array",
                    "value": [
                        {"kind": "struct", "value": {"x": {"kind": "f64", "value": 1.0}}},
                        {"kind": "struct", "value": {"x": {"kind": "f64", "value": 2.0}}}
                    ]
                }
            }
        }))
        .unwrap();
        assert_eq!(
            value.get_path("points.0.x"),
            Some(&CanonicalValue::F64(1.0))
        );
        assert_eq!(
            value.get_path("points.1.x"),
            Some(&CanonicalValue::F64(2.0))
        );
        assert_eq!(value.get_path("points.2.x"), None);
        assert_eq!(value.get_path("nope"), None);
    }

    use proptest::prelude::*;

    fn value_strategy() -> impl Strategy<Value = CanonicalValue> {
        let leaf = prop_oneof![
            Just(CanonicalValue::Null),
            any::<bool>().prop_map(CanonicalValue::Bool),
            any::<i64>().prop_map(CanonicalValue::Int),
            any::<u64>().prop_map(CanonicalValue::Uint),
            "[a-zA-Z0-9 _]{0,32}".prop_map(CanonicalValue::String),
            prop::collection::vec(any::<u8>(), 0..16).prop_map(CanonicalValue::Bytes),
            (any::<i32>(), any::<u32>())
                .prop_map(|(sec, nanosec)| CanonicalValue::Time { sec, nanosec }),
            (any::<i32>(), any::<u32>())
                .prop_map(|(sec, nanosec)| CanonicalValue::Duration { sec, nanosec }),
        ];
        leaf.prop_recursive(3, 16, 4, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..4).prop_map(CanonicalValue::Array),
                prop::collection::btree_map("[a-z]{1,8}", inner, 0..4)
                    .prop_map(CanonicalValue::Struct),
            ]
        })
    }

    proptest! {
        #[test]
        fn arbitrary_value_roundtrips_through_json(value in value_strategy()) {
            let json = serde_json::to_string(&value).unwrap();
            let decoded: CanonicalValue = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(decoded, value);
        }
    }
}
