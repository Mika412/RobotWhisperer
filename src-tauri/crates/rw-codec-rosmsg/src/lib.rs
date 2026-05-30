#![deny(missing_debug_implementations)]

use rw_canonical::{ArrayLength, CanonicalValue, FieldDef, FieldType, MessageDef, PrimitiveType};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RosmsgError {
    #[error("payload too short: needed {needed} bytes at offset {offset}, have {available}")]
    Eof {
        needed: usize,
        offset: usize,
        available: usize,
    },
    #[error("unresolved complex type '{0}'")]
    UnknownComplex(String),
    #[error("invalid string: {0}")]
    InvalidString(String),
    #[error("type mismatch on field '{field}': expected {expected}, got {got}")]
    TypeMismatch {
        field: String,
        expected: String,
        got: String,
    },
    #[error("missing field '{0}'")]
    MissingField(String),
}

pub type RosmsgResult<T> = Result<T, RosmsgError>;

pub type Resolver = std::collections::HashMap<String, MessageDef>;

pub fn decode_message(
    payload: &[u8],
    message: &MessageDef,
    resolver: &Resolver,
) -> RosmsgResult<CanonicalValue> {
    let mut reader = Reader {
        bytes: payload,
        cursor: 0,
    };
    decode_message_inner(&mut reader, message, resolver)
}

pub fn encode_message(
    value: &CanonicalValue,
    message: &MessageDef,
    resolver: &Resolver,
) -> RosmsgResult<Vec<u8>> {
    let mut writer = Writer { bytes: Vec::new() };
    encode_message_inner(&mut writer, value, message, resolver, "<root>")?;
    Ok(writer.bytes)
}

fn decode_message_inner(
    reader: &mut Reader<'_>,
    message: &MessageDef,
    resolver: &Resolver,
) -> RosmsgResult<CanonicalValue> {
    let mut fields = std::collections::BTreeMap::new();
    for FieldDef {
        name, field_type, ..
    } in &message.fields
    {
        let value = decode_field(reader, field_type, resolver)?;
        fields.insert(name.clone(), value);
    }
    Ok(CanonicalValue::Struct(fields))
}

fn decode_field(
    reader: &mut Reader<'_>,
    field_type: &FieldType,
    resolver: &Resolver,
) -> RosmsgResult<CanonicalValue> {
    match field_type {
        FieldType::Primitive(primitive) => decode_primitive(reader, *primitive),
        FieldType::String { .. } => Ok(CanonicalValue::String(reader.read_string()?)),
        FieldType::WString { .. } => Ok(CanonicalValue::String(reader.read_string()?)),
        FieldType::Time | FieldType::Duration => {
            let sec = reader.read_i32()?;
            let nanosec = reader.read_u32()?;
            Ok(if matches!(field_type, FieldType::Time) {
                CanonicalValue::Time { sec, nanosec }
            } else {
                CanonicalValue::Duration { sec, nanosec }
            })
        }
        FieldType::Array { element, length } => {
            let count = match length {
                ArrayLength::Fixed(n) => *n,
                _ => reader.read_u32()? as usize,
            };
            if let FieldType::Primitive(PrimitiveType::Uint8 | PrimitiveType::Byte) = **element {
                let mut buffer = vec![0u8; count];
                reader.read_into(&mut buffer)?;
                return Ok(CanonicalValue::Bytes(buffer));
            }
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                items.push(decode_field(reader, element, resolver)?);
            }
            Ok(CanonicalValue::Array(items))
        }
        FieldType::Complex { type_name } => {
            let nested = resolver
                .get(type_name)
                .ok_or_else(|| RosmsgError::UnknownComplex(type_name.clone()))?;
            decode_message_inner(reader, nested, resolver)
        }
    }
}

fn decode_primitive(
    reader: &mut Reader<'_>,
    primitive: PrimitiveType,
) -> RosmsgResult<CanonicalValue> {
    Ok(match primitive {
        PrimitiveType::Bool => CanonicalValue::Bool(reader.read_u8()? != 0),
        PrimitiveType::Byte | PrimitiveType::Uint8 => {
            CanonicalValue::Uint(reader.read_u8()? as u64)
        }
        PrimitiveType::Char | PrimitiveType::Int8 => {
            CanonicalValue::Int(reader.read_u8()? as i8 as i64)
        }
        PrimitiveType::Uint16 => CanonicalValue::Uint(reader.read_u16()? as u64),
        PrimitiveType::Int16 => CanonicalValue::Int(reader.read_u16()? as i16 as i64),
        PrimitiveType::Uint32 => CanonicalValue::Uint(reader.read_u32()? as u64),
        PrimitiveType::Int32 => CanonicalValue::Int(reader.read_i32()? as i64),
        PrimitiveType::Uint64 => CanonicalValue::Uint(reader.read_u64()?),
        PrimitiveType::Int64 => CanonicalValue::Int(reader.read_u64()? as i64),
        PrimitiveType::Float32 => CanonicalValue::F32(reader.read_f32()?),
        PrimitiveType::Float64 => CanonicalValue::F64(reader.read_f64()?),
    })
}

fn encode_message_inner(
    writer: &mut Writer,
    value: &CanonicalValue,
    message: &MessageDef,
    resolver: &Resolver,
    field_path: &str,
) -> RosmsgResult<()> {
    let fields = match value {
        CanonicalValue::Struct(fields) => fields,
        other => {
            return Err(RosmsgError::TypeMismatch {
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
            .ok_or_else(|| RosmsgError::MissingField(name.clone()))?;
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
) -> RosmsgResult<()> {
    macro_rules! mismatch {
        ($expected:expr) => {
            RosmsgError::TypeMismatch {
                field: field_name.into(),
                expected: $expected.into(),
                got: variant_name(value).into(),
            }
        };
    }
    match field_type {
        FieldType::Primitive(primitive) => match (primitive, value) {
            (PrimitiveType::Bool, CanonicalValue::Bool(b)) => writer.write_u8(*b as u8),
            (PrimitiveType::Byte | PrimitiveType::Uint8, CanonicalValue::Uint(v)) => {
                writer.write_u8(*v as u8)
            }
            (PrimitiveType::Char | PrimitiveType::Int8, CanonicalValue::Int(v)) => {
                writer.write_u8(*v as i8 as u8)
            }
            (PrimitiveType::Uint16, CanonicalValue::Uint(v)) => writer.write_u16(*v as u16),
            (PrimitiveType::Int16, CanonicalValue::Int(v)) => writer.write_u16(*v as i16 as u16),
            (PrimitiveType::Uint32, CanonicalValue::Uint(v)) => writer.write_u32(*v as u32),
            (PrimitiveType::Int32, CanonicalValue::Int(v)) => writer.write_i32(*v as i32),
            (PrimitiveType::Uint64, CanonicalValue::Uint(v)) => writer.write_u64(*v),
            (PrimitiveType::Int64, CanonicalValue::Int(v)) => writer.write_u64(*v as u64),
            (PrimitiveType::Float32, CanonicalValue::F32(v)) => writer.write_f32(*v),
            (PrimitiveType::Float64, CanonicalValue::F64(v)) => writer.write_f64(*v),
            (PrimitiveType::Float64, CanonicalValue::F32(v)) => writer.write_f64(*v as f64),
            (PrimitiveType::Float32, CanonicalValue::F64(v)) => writer.write_f32(*v as f32),
            _ => return Err(mismatch!(primitive.as_str())),
        },
        FieldType::String { .. } | FieldType::WString { .. } => match value {
            CanonicalValue::String(s) => writer.write_string(s),
            _ => return Err(mismatch!("string")),
        },
        FieldType::Time | FieldType::Duration => match value {
            CanonicalValue::Time { sec, nanosec } | CanonicalValue::Duration { sec, nanosec } => {
                writer.write_i32(*sec);
                writer.write_u32(*nanosec);
            }
            _ => return Err(mismatch!("time")),
        },
        FieldType::Array { element, length } => {
            if let FieldType::Primitive(PrimitiveType::Uint8 | PrimitiveType::Byte) = **element {
                if let CanonicalValue::Bytes(bytes) = value {
                    match length {
                        ArrayLength::Fixed(_) => {}
                        _ => writer.write_u32(bytes.len() as u32),
                    }
                    writer.bytes.extend_from_slice(bytes);
                    return Ok(());
                }
            }
            let items = match value {
                CanonicalValue::Array(items) => items,
                _ => return Err(mismatch!("array")),
            };
            match length {
                ArrayLength::Fixed(_) => {}
                _ => writer.write_u32(items.len() as u32),
            }
            for item in items {
                encode_field(writer, item, element, resolver, field_name)?;
            }
        }
        FieldType::Complex { type_name } => {
            let nested = resolver
                .get(type_name)
                .ok_or_else(|| RosmsgError::UnknownComplex(type_name.clone()))?;
            encode_message_inner(writer, value, nested, resolver, field_name)?;
        }
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

#[derive(Debug)]
struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    fn slice(&mut self, count: usize) -> RosmsgResult<&'a [u8]> {
        if self.cursor + count > self.bytes.len() {
            return Err(RosmsgError::Eof {
                needed: count,
                offset: self.cursor,
                available: self.bytes.len() - self.cursor,
            });
        }
        let out = &self.bytes[self.cursor..self.cursor + count];
        self.cursor += count;
        Ok(out)
    }
    fn read_into(&mut self, buf: &mut [u8]) -> RosmsgResult<()> {
        let len = buf.len();
        let s = self.slice(len)?;
        buf.copy_from_slice(s);
        Ok(())
    }
    fn read_u8(&mut self) -> RosmsgResult<u8> {
        Ok(self.slice(1)?[0])
    }
    fn read_u16(&mut self) -> RosmsgResult<u16> {
        let s = self.slice(2)?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }
    fn read_u32(&mut self) -> RosmsgResult<u32> {
        let s = self.slice(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn read_i32(&mut self) -> RosmsgResult<i32> {
        Ok(self.read_u32()? as i32)
    }
    fn read_u64(&mut self) -> RosmsgResult<u64> {
        let s = self.slice(8)?;
        let mut b = [0u8; 8];
        b.copy_from_slice(s);
        Ok(u64::from_le_bytes(b))
    }
    fn read_f32(&mut self) -> RosmsgResult<f32> {
        let s = self.slice(4)?;
        let mut b = [0u8; 4];
        b.copy_from_slice(s);
        Ok(f32::from_le_bytes(b))
    }
    fn read_f64(&mut self) -> RosmsgResult<f64> {
        let s = self.slice(8)?;
        let mut b = [0u8; 8];
        b.copy_from_slice(s);
        Ok(f64::from_le_bytes(b))
    }
    fn read_string(&mut self) -> RosmsgResult<String> {
        let len = self.read_u32()? as usize;
        if len == 0 {
            return Ok(String::new());
        }
        let raw = self.slice(len)?;
        std::str::from_utf8(raw)
            .map(|s| s.to_string())
            .map_err(|err| RosmsgError::InvalidString(err.to_string()))
    }
}

#[derive(Debug)]
struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn write_u8(&mut self, v: u8) {
        self.bytes.push(v);
    }
    fn write_u16(&mut self, v: u16) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    fn write_u32(&mut self, v: u32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    fn write_i32(&mut self, v: i32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    fn write_u64(&mut self, v: u64) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    fn write_f32(&mut self, v: f32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    fn write_f64(&mut self, v: f64) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    fn write_string(&mut self, s: &str) {
        self.write_u32(s.len() as u32);
        self.bytes.extend_from_slice(s.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn ros1_int32_roundtrip_has_no_padding() {
        let message = MessageDef {
            fields: vec![
                primitive_field("a", PrimitiveType::Int8),
                primitive_field("b", PrimitiveType::Int8),
                primitive_field("c", PrimitiveType::Int32),
            ],
            constants: vec![],
        };
        let value = CanonicalValue::Struct(BTreeMap::from([
            ("a".into(), CanonicalValue::Int(1)),
            ("b".into(), CanonicalValue::Int(2)),
            ("c".into(), CanonicalValue::Int(0x0A0B0C0D)),
        ]));
        let resolver = Resolver::new();
        let encoded = encode_message(&value, &message, &resolver).unwrap();
        assert_eq!(encoded.len(), 1 + 1 + 4);
        let decoded = decode_message(&encoded, &message, &resolver).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn ros1_string_has_no_nul_terminator() {
        let message = MessageDef {
            fields: vec![FieldDef {
                name: "data".into(),
                field_type: FieldType::String { bound: None },
                default: None,
                comment: None,
            }],
            constants: vec![],
        };
        let value = CanonicalValue::Struct(BTreeMap::from([(
            "data".into(),
            CanonicalValue::String("hi".into()),
        )]));
        let resolver = Resolver::new();
        let encoded = encode_message(&value, &message, &resolver).unwrap();
        assert_eq!(encoded, vec![2, 0, 0, 0, b'h', b'i']);
        let decoded = decode_message(&encoded, &message, &resolver).unwrap();
        assert_eq!(decoded, value);
    }
}
