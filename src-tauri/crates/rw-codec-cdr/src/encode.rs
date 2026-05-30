use rw_canonical::{ArrayLength, CanonicalValue, FieldDef, FieldType, MessageDef, PrimitiveType};

use crate::Resolver;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EncodeError {
    #[error("type mismatch encoding field '{field}': expected {expected}, got {got}")]
    TypeMismatch {
        field: String,
        expected: String,
        got: String,
    },
    #[error("missing field '{0}' in value during encode")]
    MissingField(String),
    #[error("unresolved complex type '{0}'")]
    UnknownComplex(String),
    #[error("array length mismatch on field '{field}': expected {expected}, got {got}")]
    ArrayLength {
        field: String,
        expected: usize,
        got: usize,
    },
}

pub type EncodeResult<T> = Result<T, EncodeError>;

pub fn encode_message(
    value: &CanonicalValue,
    message: &MessageDef,
    resolver: &Resolver,
) -> EncodeResult<Vec<u8>> {
    let mut writer = Writer::new_with_encapsulation();
    encode_message_inner(&mut writer, value, message, resolver, "<root>")?;
    Ok(writer.into_bytes())
}

pub fn encode_message_body(
    value: &CanonicalValue,
    message: &MessageDef,
    resolver: &Resolver,
) -> EncodeResult<Vec<u8>> {
    let mut writer = Writer::new_body_only();
    encode_message_inner(&mut writer, value, message, resolver, "<root>")?;
    Ok(writer.into_bytes())
}

fn encode_message_inner(
    writer: &mut Writer,
    value: &CanonicalValue,
    message: &MessageDef,
    resolver: &Resolver,
    field_path: &str,
) -> EncodeResult<()> {
    let fields = match value {
        CanonicalValue::Struct(fields) => fields,
        other => {
            return Err(EncodeError::TypeMismatch {
                field: field_path.into(),
                expected: "struct".into(),
                got: variant_name(other).into(),
            })
        }
    };
    for FieldDef {
        name, field_type, ..
    } in &message.fields
    {
        let inner = fields
            .get(name)
            .ok_or_else(|| EncodeError::MissingField(name.clone()))?;
        encode_field(writer, inner, field_type, resolver, name)?;
    }
    Ok(())
}

fn encode_field(
    writer: &mut Writer,
    value: &CanonicalValue,
    field_type: &FieldType,
    resolver: &Resolver,
    field_name: &str,
) -> EncodeResult<()> {
    match field_type {
        FieldType::Primitive(primitive) => encode_primitive(writer, value, *primitive, field_name),
        FieldType::String { .. } => match value {
            CanonicalValue::String(text) => {
                writer.write_string(text);
                Ok(())
            }
            other => Err(EncodeError::TypeMismatch {
                field: field_name.into(),
                expected: "string".into(),
                got: variant_name(other).into(),
            }),
        },
        FieldType::WString { .. } => match value {
            CanonicalValue::String(text) => {
                let units: Vec<u16> = text.encode_utf16().collect();
                writer.write_u32(units.len() as u32);
                writer.align(2);
                for unit in &units {
                    writer.write_u16(*unit);
                }
                Ok(())
            }
            other => Err(EncodeError::TypeMismatch {
                field: field_name.into(),
                expected: "wstring".into(),
                got: variant_name(other).into(),
            }),
        },
        FieldType::Time => match value {
            CanonicalValue::Time { sec, nanosec } => {
                writer.align(4);
                writer.write_i32(*sec);
                writer.write_u32(*nanosec);
                Ok(())
            }
            other => Err(EncodeError::TypeMismatch {
                field: field_name.into(),
                expected: "time".into(),
                got: variant_name(other).into(),
            }),
        },
        FieldType::Duration => match value {
            CanonicalValue::Duration { sec, nanosec } => {
                writer.align(4);
                writer.write_i32(*sec);
                writer.write_u32(*nanosec);
                Ok(())
            }
            other => Err(EncodeError::TypeMismatch {
                field: field_name.into(),
                expected: "duration".into(),
                got: variant_name(other).into(),
            }),
        },
        FieldType::Array { element, length } => {
            encode_array(writer, value, element, length, resolver, field_name)
        }
        FieldType::Complex { type_name } => {
            let nested = resolver
                .get(type_name)
                .ok_or_else(|| EncodeError::UnknownComplex(type_name.clone()))?;
            encode_message_inner(writer, value, nested, resolver, field_name)
        }
    }
}

fn encode_array(
    writer: &mut Writer,
    value: &CanonicalValue,
    element: &FieldType,
    length: &ArrayLength,
    resolver: &Resolver,
    field_name: &str,
) -> EncodeResult<()> {
    if let FieldType::Primitive(PrimitiveType::Uint8 | PrimitiveType::Byte) = *element {
        if let CanonicalValue::Bytes(bytes) = value {
            match length {
                ArrayLength::Fixed(n) if bytes.len() != *n => {
                    return Err(EncodeError::ArrayLength {
                        field: field_name.into(),
                        expected: *n,
                        got: bytes.len(),
                    });
                }
                ArrayLength::Fixed(_) => {}
                _ => writer.write_u32(bytes.len() as u32),
            }
            writer.write_bytes(bytes);
            return Ok(());
        }
    }
    let items = match value {
        CanonicalValue::Array(items) => items,
        other => {
            return Err(EncodeError::TypeMismatch {
                field: field_name.into(),
                expected: "array".into(),
                got: variant_name(other).into(),
            })
        }
    };
    match length {
        ArrayLength::Fixed(n) if items.len() != *n => {
            return Err(EncodeError::ArrayLength {
                field: field_name.into(),
                expected: *n,
                got: items.len(),
            });
        }
        ArrayLength::Fixed(_) => {}
        _ => writer.write_u32(items.len() as u32),
    }
    for item in items {
        encode_field(writer, item, element, resolver, field_name)?;
    }
    Ok(())
}

fn encode_primitive(
    writer: &mut Writer,
    value: &CanonicalValue,
    primitive: PrimitiveType,
    field_name: &str,
) -> EncodeResult<()> {
    macro_rules! mismatch {
        ($expected:expr) => {
            EncodeError::TypeMismatch {
                field: field_name.into(),
                expected: $expected.into(),
                got: variant_name(value).into(),
            }
        };
    }
    match primitive {
        PrimitiveType::Bool => match value {
            CanonicalValue::Bool(b) => writer.write_u8(if *b { 1 } else { 0 }),
            _ => return Err(mismatch!("bool")),
        },
        PrimitiveType::Byte | PrimitiveType::Uint8 => match value {
            CanonicalValue::Uint(v) => writer.write_u8(*v as u8),
            CanonicalValue::Int(v) if *v >= 0 && *v <= u8::MAX as i64 => writer.write_u8(*v as u8),
            _ => return Err(mismatch!("uint8")),
        },
        PrimitiveType::Char | PrimitiveType::Int8 => match value {
            CanonicalValue::Int(v) => writer.write_u8(*v as i8 as u8),
            _ => return Err(mismatch!("int8")),
        },
        PrimitiveType::Uint16 => match value {
            CanonicalValue::Uint(v) => writer.write_u16(*v as u16),
            CanonicalValue::Int(v) if *v >= 0 => writer.write_u16(*v as u16),
            _ => return Err(mismatch!("uint16")),
        },
        PrimitiveType::Int16 => match value {
            CanonicalValue::Int(v) => writer.write_i16(*v as i16),
            _ => return Err(mismatch!("int16")),
        },
        PrimitiveType::Uint32 => match value {
            CanonicalValue::Uint(v) => writer.write_u32(*v as u32),
            CanonicalValue::Int(v) if *v >= 0 => writer.write_u32(*v as u32),
            _ => return Err(mismatch!("uint32")),
        },
        PrimitiveType::Int32 => match value {
            CanonicalValue::Int(v) => writer.write_i32(*v as i32),
            _ => return Err(mismatch!("int32")),
        },
        PrimitiveType::Uint64 => match value {
            CanonicalValue::Uint(v) => writer.write_u64(*v),
            CanonicalValue::Int(v) if *v >= 0 => writer.write_u64(*v as u64),
            _ => return Err(mismatch!("uint64")),
        },
        PrimitiveType::Int64 => match value {
            CanonicalValue::Int(v) => writer.write_i64(*v),
            _ => return Err(mismatch!("int64")),
        },
        PrimitiveType::Float32 => match value {
            CanonicalValue::F32(v) => writer.write_f32(*v),
            CanonicalValue::F64(v) => writer.write_f32(*v as f32),
            _ => return Err(mismatch!("float32")),
        },
        PrimitiveType::Float64 => match value {
            CanonicalValue::F64(v) => writer.write_f64(*v),
            CanonicalValue::F32(v) => writer.write_f64(*v as f64),
            _ => return Err(mismatch!("float64")),
        },
    }
    Ok(())
}

fn variant_name(value: &CanonicalValue) -> &'static str {
    match value {
        CanonicalValue::Null => "null",
        CanonicalValue::Bool(_) => "bool",
        CanonicalValue::Int(_) => "int",
        CanonicalValue::Uint(_) => "uint",
        CanonicalValue::F32(_) => "f32",
        CanonicalValue::F64(_) => "f64",
        CanonicalValue::String(_) => "string",
        CanonicalValue::Bytes(_) => "bytes",
        CanonicalValue::Array(_) => "array",
        CanonicalValue::Struct(_) => "struct",
        CanonicalValue::Time { .. } => "time",
        CanonicalValue::Duration { .. } => "duration",
    }
}

struct Writer {
    bytes: Vec<u8>,
    body_anchor: usize,
}

impl Writer {
    fn new_with_encapsulation() -> Self {
        Writer {
            bytes: vec![0x00, 0x01, 0x00, 0x00],
            body_anchor: 4,
        }
    }

    fn new_body_only() -> Self {
        Writer {
            bytes: Vec::new(),
            body_anchor: 0,
        }
    }

    fn align(&mut self, alignment: usize) {
        let position = self.bytes.len() - self.body_anchor;
        let remainder = position % alignment;
        if remainder != 0 {
            for _ in 0..(alignment - remainder) {
                self.bytes.push(0);
            }
        }
    }

    fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn write_u16(&mut self, value: u16) {
        self.align(2);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_i16(&mut self, value: i16) {
        self.write_u16(value as u16);
    }

    fn write_u32(&mut self, value: u32) {
        self.align(4);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_i32(&mut self, value: i32) {
        self.write_u32(value as u32);
    }

    fn write_u64(&mut self, value: u64) {
        self.align(8);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_i64(&mut self, value: i64) {
        self.write_u64(value as u64);
    }

    fn write_f32(&mut self, value: f32) {
        self.align(4);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_f64(&mut self, value: f64) {
        self.align(8);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_string(&mut self, text: &str) {
        let body = text.as_bytes();
        self.write_u32(body.len() as u32 + 1);
        self.bytes.extend_from_slice(body);
        self.bytes.push(0);
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_message;
    use proptest::prelude::*;
    use rw_canonical::ArrayLength;
    use std::collections::BTreeMap;

    fn primitive_field(name: &str, primitive: PrimitiveType) -> FieldDef {
        FieldDef {
            name: name.into(),
            field_type: FieldType::Primitive(primitive),
            default: None,
            comment: None,
        }
    }

    #[test]
    fn encode_decode_two_int32s_roundtrips() {
        let message = MessageDef {
            fields: vec![
                primitive_field("a", PrimitiveType::Int32),
                primitive_field("b", PrimitiveType::Int32),
            ],
            constants: vec![],
        };
        let value = CanonicalValue::Struct(BTreeMap::from([
            ("a".into(), CanonicalValue::Int(5)),
            ("b".into(), CanonicalValue::Int(7)),
        ]));
        let resolver = Resolver::new();
        let encoded = encode_message(&value, &message, &resolver).unwrap();
        let decoded = decode_message(&encoded, &message, &resolver).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn encode_decode_string_roundtrips() {
        let message = MessageDef {
            fields: vec![FieldDef {
                name: "frame_id".into(),
                field_type: FieldType::String { bound: None },
                default: None,
                comment: None,
            }],
            constants: vec![],
        };
        let value = CanonicalValue::Struct(BTreeMap::from([(
            "frame_id".into(),
            CanonicalValue::String("map".into()),
        )]));
        let resolver = Resolver::new();
        let encoded = encode_message(&value, &message, &resolver).unwrap();
        let decoded = decode_message(&encoded, &message, &resolver).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn encode_decode_bytes_array_roundtrips() {
        let message = MessageDef {
            fields: vec![FieldDef {
                name: "data".into(),
                field_type: FieldType::Array {
                    element: Box::new(FieldType::Primitive(PrimitiveType::Uint8)),
                    length: ArrayLength::Unbounded,
                },
                default: None,
                comment: None,
            }],
            constants: vec![],
        };
        let value = CanonicalValue::Struct(BTreeMap::from([(
            "data".into(),
            CanonicalValue::Bytes(vec![1, 2, 3, 4]),
        )]));
        let resolver = Resolver::new();
        let encoded = encode_message(&value, &message, &resolver).unwrap();
        let decoded = decode_message(&encoded, &message, &resolver).unwrap();
        assert_eq!(decoded, value);
    }

    proptest! {
        #[test]
        fn int32_roundtrips_through_cdr(value in any::<i32>()) {
            let message = MessageDef {
                fields: vec![primitive_field("v", PrimitiveType::Int32)],
                constants: vec![],
            };
            let canonical = CanonicalValue::Struct(BTreeMap::from([(
                "v".into(),
                CanonicalValue::Int(value as i64),
            )]));
            let resolver = Resolver::new();
            let encoded = encode_message(&canonical, &message, &resolver).unwrap();
            let decoded = decode_message(&encoded, &message, &resolver).unwrap();
            prop_assert_eq!(decoded, canonical);
        }

        #[test]
        fn float64_roundtrips_through_cdr(value in -1.0e9_f64..1.0e9_f64) {
            let message = MessageDef {
                fields: vec![primitive_field("v", PrimitiveType::Float64)],
                constants: vec![],
            };
            let canonical = CanonicalValue::Struct(BTreeMap::from([(
                "v".into(),
                CanonicalValue::F64(value),
            )]));
            let resolver = Resolver::new();
            let encoded = encode_message(&canonical, &message, &resolver).unwrap();
            let decoded = decode_message(&encoded, &message, &resolver).unwrap();
            prop_assert_eq!(decoded, canonical);
        }
    }
}
