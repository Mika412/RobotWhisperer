use rw_canonical::{ArrayLength, CanonicalValue, FieldDef, FieldType, MessageDef, PrimitiveType};

use crate::Resolver;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DecodeError {
    #[error("payload too short: needed {needed} bytes at offset {offset}, have {available}")]
    Eof {
        needed: usize,
        offset: usize,
        available: usize,
    },
    #[error("unknown encapsulation representation 0x{0:02x}{1:02x}")]
    UnsupportedEncapsulation(u8, u8),
    #[error("unresolved complex type '{0}'")]
    UnknownComplex(String),
    #[error("invalid string: {0}")]
    InvalidString(String),
}

pub type DecodeResult<T> = Result<T, DecodeError>;

pub fn decode_message(
    payload: &[u8],
    message: &MessageDef,
    resolver: &Resolver,
) -> DecodeResult<CanonicalValue> {
    let mut reader = Reader::open(payload)?;
    decode_message_inner(&mut reader, message, resolver)
}

pub fn decode_message_body(
    payload: &[u8],
    message: &MessageDef,
    resolver: &Resolver,
) -> DecodeResult<CanonicalValue> {
    let mut reader = Reader {
        bytes: payload,
        cursor: 0,
        body_anchor: 0,
    };
    decode_message_inner(&mut reader, message, resolver)
}

fn decode_message_inner(
    reader: &mut Reader<'_>,
    message: &MessageDef,
    resolver: &Resolver,
) -> DecodeResult<CanonicalValue> {
    let mut fields: std::collections::BTreeMap<String, CanonicalValue> =
        std::collections::BTreeMap::new();
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
) -> DecodeResult<CanonicalValue> {
    match field_type {
        FieldType::Primitive(primitive) => decode_primitive(reader, *primitive),
        FieldType::String { .. } => Ok(CanonicalValue::String(reader.read_string()?)),
        FieldType::WString { .. } => {
            let len = reader.read_u32()? as usize;
            reader.align(2)?;
            let mut chars: Vec<u16> = Vec::with_capacity(len);
            for _ in 0..len {
                chars.push(reader.read_u16()?);
            }
            String::from_utf16(&chars)
                .map(CanonicalValue::String)
                .map_err(|err| DecodeError::InvalidString(err.to_string()))
        }
        FieldType::Time => {
            reader.align(4)?;
            let sec = reader.read_i32()?;
            let nanosec = reader.read_u32()?;
            Ok(CanonicalValue::Time { sec, nanosec })
        }
        FieldType::Duration => {
            reader.align(4)?;
            let sec = reader.read_i32()?;
            let nanosec = reader.read_u32()?;
            Ok(CanonicalValue::Duration { sec, nanosec })
        }
        FieldType::Array { element, length } => {
            let count = match length {
                ArrayLength::Fixed(n) => *n,
                ArrayLength::Bounded(_) | ArrayLength::Unbounded => reader.read_u32()? as usize,
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
                .ok_or_else(|| DecodeError::UnknownComplex(type_name.clone()))?;
            decode_message_inner(reader, nested, resolver)
        }
    }
}

fn decode_primitive(
    reader: &mut Reader<'_>,
    primitive: PrimitiveType,
) -> DecodeResult<CanonicalValue> {
    Ok(match primitive {
        PrimitiveType::Bool => CanonicalValue::Bool(reader.read_u8()? != 0),
        PrimitiveType::Byte | PrimitiveType::Uint8 => {
            CanonicalValue::Uint(reader.read_u8()? as u64)
        }
        PrimitiveType::Char | PrimitiveType::Int8 => CanonicalValue::Int(reader.read_i8()? as i64),
        PrimitiveType::Uint16 => CanonicalValue::Uint(reader.read_u16()? as u64),
        PrimitiveType::Int16 => CanonicalValue::Int(reader.read_i16()? as i64),
        PrimitiveType::Uint32 => CanonicalValue::Uint(reader.read_u32()? as u64),
        PrimitiveType::Int32 => CanonicalValue::Int(reader.read_i32()? as i64),
        PrimitiveType::Uint64 => CanonicalValue::Uint(reader.read_u64()?),
        PrimitiveType::Int64 => CanonicalValue::Int(reader.read_i64()?),
        PrimitiveType::Float32 => CanonicalValue::F32(reader.read_f32()?),
        PrimitiveType::Float64 => CanonicalValue::F64(reader.read_f64()?),
    })
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
    body_anchor: usize,
}

impl<'a> Reader<'a> {
    fn open(bytes: &'a [u8]) -> DecodeResult<Reader<'a>> {
        if bytes.len() < 4 {
            return Err(DecodeError::Eof {
                needed: 4,
                offset: 0,
                available: bytes.len(),
            });
        }
        let rep = (bytes[0], bytes[1]);
        if rep != (0x00, 0x01) {
            return Err(DecodeError::UnsupportedEncapsulation(rep.0, rep.1));
        }
        Ok(Reader {
            bytes,
            cursor: 4,
            body_anchor: 4,
        })
    }

    fn align(&mut self, alignment: usize) -> DecodeResult<()> {
        let position = self.cursor - self.body_anchor;
        let remainder = position % alignment;
        if remainder == 0 {
            return Ok(());
        }
        self.advance(alignment - remainder)
    }

    fn advance(&mut self, count: usize) -> DecodeResult<()> {
        if self.cursor + count > self.bytes.len() {
            return Err(DecodeError::Eof {
                needed: count,
                offset: self.cursor,
                available: self.bytes.len() - self.cursor,
            });
        }
        self.cursor += count;
        Ok(())
    }

    fn slice(&mut self, count: usize) -> DecodeResult<&'a [u8]> {
        if self.cursor + count > self.bytes.len() {
            return Err(DecodeError::Eof {
                needed: count,
                offset: self.cursor,
                available: self.bytes.len() - self.cursor,
            });
        }
        let out = &self.bytes[self.cursor..self.cursor + count];
        self.cursor += count;
        Ok(out)
    }

    fn read_into(&mut self, buffer: &mut [u8]) -> DecodeResult<()> {
        let len = buffer.len();
        let slice = self.slice(len)?;
        buffer.copy_from_slice(slice);
        Ok(())
    }

    fn read_u8(&mut self) -> DecodeResult<u8> {
        Ok(self.slice(1)?[0])
    }

    fn read_i8(&mut self) -> DecodeResult<i8> {
        Ok(self.read_u8()? as i8)
    }

    fn read_u16(&mut self) -> DecodeResult<u16> {
        self.align(2)?;
        let slice = self.slice(2)?;
        Ok(u16::from_le_bytes([slice[0], slice[1]]))
    }

    fn read_i16(&mut self) -> DecodeResult<i16> {
        Ok(self.read_u16()? as i16)
    }

    fn read_u32(&mut self) -> DecodeResult<u32> {
        self.align(4)?;
        let slice = self.slice(4)?;
        Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }

    fn read_i32(&mut self) -> DecodeResult<i32> {
        Ok(self.read_u32()? as i32)
    }

    fn read_u64(&mut self) -> DecodeResult<u64> {
        self.align(8)?;
        let slice = self.slice(8)?;
        let mut buf = [0u8; 8];
        buf.copy_from_slice(slice);
        Ok(u64::from_le_bytes(buf))
    }

    fn read_i64(&mut self) -> DecodeResult<i64> {
        Ok(self.read_u64()? as i64)
    }

    fn read_f32(&mut self) -> DecodeResult<f32> {
        self.align(4)?;
        let slice = self.slice(4)?;
        let mut buf = [0u8; 4];
        buf.copy_from_slice(slice);
        Ok(f32::from_le_bytes(buf))
    }

    fn read_f64(&mut self) -> DecodeResult<f64> {
        self.align(8)?;
        let slice = self.slice(8)?;
        let mut buf = [0u8; 8];
        buf.copy_from_slice(slice);
        Ok(f64::from_le_bytes(buf))
    }

    fn read_string(&mut self) -> DecodeResult<String> {
        let length = self.read_u32()? as usize;
        if length == 0 {
            return Ok(String::new());
        }
        let raw = self.slice(length)?;
        let body = if raw.last() == Some(&0) {
            &raw[..raw.len() - 1]
        } else {
            raw
        };
        std::str::from_utf8(body)
            .map(|text| text.to_string())
            .map_err(|err| DecodeError::InvalidString(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn primitive_field(name: &str, primitive: PrimitiveType) -> FieldDef {
        FieldDef {
            name: name.into(),
            field_type: FieldType::Primitive(primitive),
            default: None,
            comment: None,
        }
    }

    fn cdr_le(body: impl IntoIterator<Item = u8>) -> Vec<u8> {
        let mut out = vec![0x00, 0x01, 0x00, 0x00];
        out.extend(body);
        out
    }

    #[test]
    fn two_int32s() {
        let message = MessageDef {
            fields: vec![
                primitive_field("a", PrimitiveType::Int32),
                primitive_field("b", PrimitiveType::Int32),
            ],
            constants: vec![],
        };
        let payload = cdr_le([5, 0, 0, 0, 7, 0, 0, 0]);
        let resolver = Resolver::new();
        let decoded = decode_message(&payload, &message, &resolver).unwrap();
        match decoded {
            CanonicalValue::Struct(fields) => {
                assert_eq!(fields.get("a"), Some(&CanonicalValue::Int(5)));
                assert_eq!(fields.get("b"), Some(&CanonicalValue::Int(7)));
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn uint8_array_returns_bytes() {
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
        let payload = cdr_le([4, 0, 0, 0, 0xDE, 0xAD, 0xBE, 0xEF]);
        let decoded = decode_message(&payload, &message, &Resolver::new()).unwrap();
        match decoded {
            CanonicalValue::Struct(fields) => {
                assert_eq!(
                    fields.get("data"),
                    Some(&CanonicalValue::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]))
                );
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn unsupported_encapsulation_errors() {
        let payload = vec![0xFF, 0xFF, 0, 0, 1];
        let message = MessageDef::default();
        assert!(matches!(
            decode_message(&payload, &message, &Resolver::new()),
            Err(DecodeError::UnsupportedEncapsulation(0xFF, 0xFF))
        ));
    }
}
