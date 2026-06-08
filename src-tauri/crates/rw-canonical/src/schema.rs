use serde::{Deserialize, Serialize};

use crate::dialect::Dialect;
use crate::id::SchemaId;
use crate::value::CanonicalValue;
use crate::viz::VisualizationRole;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalSchema {
    pub id: SchemaId,
    pub name: String,
    pub kind: SchemaKind,
    pub dialect: Dialect,
    pub definition: String,
    pub parsed: ParsedSchema,
    pub dependencies: Vec<SchemaId>,
    #[serde(default = "VisualizationRole::default")]
    pub viz_role: VisualizationRole,
}

impl CanonicalSchema {
    pub fn primary(&self) -> &MessageDef {
        self.parsed.primary()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemaKind {
    Message,
    Service,
    Action,
}

impl SchemaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SchemaKind::Message => "message",
            SchemaKind::Service => "service",
            SchemaKind::Action => "action",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ParsedSchema {
    Message(MessageDef),
    Service {
        request: MessageDef,
        response: MessageDef,
    },
    Action {
        goal: MessageDef,
        result: MessageDef,
        feedback: MessageDef,
    },
}

impl ParsedSchema {
    pub fn primary(&self) -> &MessageDef {
        match self {
            ParsedSchema::Message(message) => message,
            ParsedSchema::Service { request, .. } => request,
            ParsedSchema::Action { goal, .. } => goal,
        }
    }

    pub fn parts(&self) -> Vec<&MessageDef> {
        match self {
            ParsedSchema::Message(message) => vec![message],
            ParsedSchema::Service { request, response } => vec![request, response],
            ParsedSchema::Action {
                goal,
                result,
                feedback,
            } => vec![goal, result, feedback],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MessageDef {
    #[serde(default)]
    pub fields: Vec<FieldDef>,
    #[serde(default)]
    pub constants: Vec<ConstantDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub field_type: FieldType,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub default: Option<CanonicalValue>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstantDef {
    pub name: String,
    pub field_type: FieldType,
    pub value: CanonicalValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum FieldType {
    Primitive(PrimitiveType),
    String {
        bound: Option<usize>,
    },
    WString {
        bound: Option<usize>,
    },
    Array {
        element: Box<FieldType>,
        length: ArrayLength,
    },
    Complex {
        type_name: String,
    },
    Time,
    Duration,
}

impl FieldType {
    pub fn complex_dependencies(&self, sink: &mut Vec<String>) {
        match self {
            FieldType::Complex { type_name } => sink.push(type_name.clone()),
            FieldType::Array { element, .. } => element.complex_dependencies(sink),
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveType {
    Bool,
    Byte,
    Char,
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Int64,
    Uint64,
    Float32,
    Float64,
}

impl PrimitiveType {
    pub fn parse(token: &str) -> Option<Self> {
        Some(match token {
            "bool" => PrimitiveType::Bool,
            "byte" => PrimitiveType::Byte,
            "char" => PrimitiveType::Char,
            "int8" => PrimitiveType::Int8,
            "uint8" => PrimitiveType::Uint8,
            "int16" => PrimitiveType::Int16,
            "uint16" => PrimitiveType::Uint16,
            "int32" => PrimitiveType::Int32,
            "uint32" => PrimitiveType::Uint32,
            "int64" => PrimitiveType::Int64,
            "uint64" => PrimitiveType::Uint64,
            "float32" => PrimitiveType::Float32,
            "float64" => PrimitiveType::Float64,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            PrimitiveType::Bool => "bool",
            PrimitiveType::Byte => "byte",
            PrimitiveType::Char => "char",
            PrimitiveType::Int8 => "int8",
            PrimitiveType::Uint8 => "uint8",
            PrimitiveType::Int16 => "int16",
            PrimitiveType::Uint16 => "uint16",
            PrimitiveType::Int32 => "int32",
            PrimitiveType::Uint32 => "uint32",
            PrimitiveType::Int64 => "int64",
            PrimitiveType::Uint64 => "uint64",
            PrimitiveType::Float32 => "float32",
            PrimitiveType::Float64 => "float64",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ArrayLength {
    Unbounded,
    Bounded(usize),
    Fixed(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_message() -> MessageDef {
        MessageDef {
            fields: vec![
                FieldDef {
                    name: "stamp".into(),
                    field_type: FieldType::Time,
                    default: None,
                    comment: None,
                },
                FieldDef {
                    name: "frame_id".into(),
                    field_type: FieldType::String { bound: None },
                    default: None,
                    comment: None,
                },
                FieldDef {
                    name: "ranges".into(),
                    field_type: FieldType::Array {
                        element: Box::new(FieldType::Primitive(PrimitiveType::Float32)),
                        length: ArrayLength::Unbounded,
                    },
                    default: None,
                    comment: None,
                },
            ],
            constants: vec![],
        }
    }

    #[test]
    fn canonical_schema_roundtrips() {
        let schema = CanonicalSchema {
            id: SchemaId::new("abc123"),
            name: "std_msgs/Header".into(),
            kind: SchemaKind::Message,
            dialect: Dialect::Ros2,
            definition: "builtin_interfaces/Time stamp\nstring frame_id\n".into(),
            parsed: ParsedSchema::Message(sample_message()),
            dependencies: vec![SchemaId::new("dep1")],
            viz_role: VisualizationRole::JsonTree,
        };
        let json = serde_json::to_string(&schema).unwrap();
        let decoded: CanonicalSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(schema, decoded);
    }

    #[test]
    fn field_type_serde_shape_is_stable() {
        let cases: Vec<(FieldType, serde_json::Value)> = vec![
            (
                FieldType::Primitive(PrimitiveType::Int32),
                serde_json::json!({"kind": "primitive", "value": "int32"}),
            ),
            (
                FieldType::String { bound: Some(64) },
                serde_json::json!({"kind": "string", "value": {"bound": 64}}),
            ),
            (
                FieldType::WString { bound: None },
                serde_json::json!({"kind": "w_string", "value": {"bound": null}}),
            ),
            (
                FieldType::Array {
                    element: Box::new(FieldType::Primitive(PrimitiveType::Float64)),
                    length: ArrayLength::Bounded(10),
                },
                serde_json::json!({
                    "kind": "array",
                    "value": {
                        "element": {"kind": "primitive", "value": "float64"},
                        "length": {"kind": "bounded", "value": 10}
                    }
                }),
            ),
            (FieldType::Time, serde_json::json!({"kind": "time"})),
        ];
        for (ty, expected) in cases {
            let actual = serde_json::to_value(&ty).unwrap();
            assert_eq!(actual, expected, "mismatch for {ty:?}");
            let decoded: FieldType = serde_json::from_value(actual).unwrap();
            assert_eq!(decoded, ty);
        }
    }

    #[test]
    fn complex_dependencies_recurses_through_arrays() {
        let ty = FieldType::Array {
            element: Box::new(FieldType::Complex {
                type_name: "geometry_msgs/Pose".into(),
            }),
            length: ArrayLength::Unbounded,
        };
        let mut deps = Vec::new();
        ty.complex_dependencies(&mut deps);
        assert_eq!(deps, vec!["geometry_msgs/Pose"]);
    }
}
