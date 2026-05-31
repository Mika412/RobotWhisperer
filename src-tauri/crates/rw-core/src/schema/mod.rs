pub mod defaults;
pub mod hash;
pub mod parser;
pub mod registry;

use crate::domain::Value;
use serde::{Deserialize, Serialize};

pub use registry::{SchemaRegistry, SchemaSummary};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaDefinition {
    pub name: String,
    pub kind: SchemaKind,
    pub hash: String,
    pub definition: String,
    pub parsed: ParsedSchema,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    pub default: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstantDef {
    pub name: String,
    pub field_type: FieldType,
    pub value: Value,
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
            FieldType::Complex { type_name } => {
                sink.push(type_name.clone());
            }
            FieldType::Array { element, .. } => element.complex_dependencies(sink),
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    #[test]
    fn schema_definition_round_trips_through_json() {
        let definition = SchemaDefinition {
            name: "std_msgs/Header".into(),
            kind: SchemaKind::Message,
            hash: "deadbeef".into(),
            definition: "uint32 seq".into(),
            parsed: ParsedSchema::Message(MessageDef::default()),
            dependencies: vec!["builtin_interfaces/Time".into()],
        };
        let json = serde_json::to_string(&definition).unwrap();
        let decoded: SchemaDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, definition);
    }

    #[test]
    fn primitive_round_trip() {
        for primitive in [
            PrimitiveType::Bool,
            PrimitiveType::Int32,
            PrimitiveType::Uint8,
            PrimitiveType::Float64,
        ] {
            assert_eq!(PrimitiveType::parse(primitive.as_str()), Some(primitive));
        }
    }

    #[test]
    fn field_type_serde_matches_typescript_types() {
        let cases: Vec<(FieldType, serde_json::Value)> = vec![
            (
                FieldType::Primitive(PrimitiveType::Int32),
                serde_json::json!({"kind": "primitive", "value": "int32"}),
            ),
            (
                FieldType::String { bound: Some(256) },
                serde_json::json!({"kind": "string", "value": {"bound": 256}}),
            ),
            (
                FieldType::String { bound: None },
                serde_json::json!({"kind": "string", "value": {"bound": null}}),
            ),
            (
                FieldType::WString { bound: None },
                serde_json::json!({"kind": "w_string", "value": {"bound": null}}),
            ),
            (
                FieldType::Complex {
                    type_name: "geometry_msgs/Point".into(),
                },
                serde_json::json!({"kind": "complex", "value": {"type_name": "geometry_msgs/Point"}}),
            ),
            (
                FieldType::Array {
                    element: Box::new(FieldType::Primitive(PrimitiveType::Float64)),
                    length: ArrayLength::Bounded(10),
                },
                serde_json::json!({"kind": "array", "value": {"element": {"kind": "primitive", "value": "float64"}, "length": {"kind": "bounded", "value": 10}}}),
            ),
            (FieldType::Time, serde_json::json!({"kind": "time"})),
            (FieldType::Duration, serde_json::json!({"kind": "duration"})),
        ];

        for (field_type, expected_json) in &cases {
            let serialized = serde_json::to_value(field_type).unwrap();
            assert_eq!(&serialized, expected_json, "mismatch for {field_type:?}");
            let deserialized: FieldType = serde_json::from_value(serialized).unwrap();
            assert_eq!(&deserialized, field_type);
        }
    }
}
