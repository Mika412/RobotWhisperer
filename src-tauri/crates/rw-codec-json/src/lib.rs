use std::collections::{BTreeMap, HashMap};

use rw_canonical::{ArrayLength, CanonicalValue, FieldDef, FieldType, MessageDef, PrimitiveType};
use serde_json::Value as JsonValue;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum JsonCodecError {
    #[error("invalid json value: {0}")]
    InvalidJson(String),
    #[error("unresolved complex type '{0}'")]
    UnknownComplex(String),
}

pub type JsonCodecResult<T> = Result<T, JsonCodecError>;

pub type Resolver = HashMap<String, MessageDef>;

pub fn json_to_canonical(value: &JsonValue) -> JsonCodecResult<CanonicalValue> {
    Ok(match value {
        JsonValue::Null => CanonicalValue::Null,
        JsonValue::Bool(b) => CanonicalValue::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                CanonicalValue::Int(i)
            } else if let Some(u) = n.as_u64() {
                CanonicalValue::Uint(u)
            } else if let Some(f) = n.as_f64() {
                CanonicalValue::F64(f)
            } else {
                return Err(JsonCodecError::InvalidJson(format!(
                    "unrepresentable number {n}"
                )));
            }
        }
        JsonValue::String(s) => CanonicalValue::String(s.clone()),
        JsonValue::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(json_to_canonical(item)?);
            }
            CanonicalValue::Array(out)
        }
        JsonValue::Object(obj) => {
            if let Some(time) = try_time_object(obj) {
                return Ok(time);
            }
            let mut out = BTreeMap::new();
            for (key, value) in obj {
                out.insert(key.clone(), json_to_canonical(value)?);
            }
            CanonicalValue::Struct(out)
        }
    })
}

pub fn json_to_canonical_with_schema(
    value: &JsonValue,
    message: &MessageDef,
    resolver: &Resolver,
) -> JsonCodecResult<CanonicalValue> {
    let obj = match value {
        JsonValue::Object(obj) => obj,
        _ => {
            return Err(JsonCodecError::InvalidJson(
                "expected object at top level".into(),
            ))
        }
    };
    let mut out = BTreeMap::new();
    for FieldDef {
        name, field_type, ..
    } in &message.fields
    {
        let json_field = obj.get(name).cloned().unwrap_or(JsonValue::Null);
        out.insert(
            name.clone(),
            decode_field(&json_field, field_type, resolver)?,
        );
    }
    Ok(CanonicalValue::Struct(out))
}

fn decode_field(
    value: &JsonValue,
    field_type: &FieldType,
    resolver: &Resolver,
) -> JsonCodecResult<CanonicalValue> {
    match field_type {
        FieldType::Primitive(primitive) => decode_primitive(value, *primitive),
        FieldType::String { .. } | FieldType::WString { .. } => match value {
            JsonValue::String(s) => Ok(CanonicalValue::String(s.clone())),
            JsonValue::Null => Ok(CanonicalValue::String(String::new())),
            other => Err(JsonCodecError::InvalidJson(format!(
                "expected string, got {other}"
            ))),
        },
        FieldType::Time => match value {
            JsonValue::Object(obj) => time_or_null(obj, false),
            JsonValue::Null => Ok(CanonicalValue::Time { sec: 0, nanosec: 0 }),
            other => Err(JsonCodecError::InvalidJson(format!(
                "expected time object, got {other}"
            ))),
        },
        FieldType::Duration => match value {
            JsonValue::Object(obj) => time_or_null(obj, true),
            JsonValue::Null => Ok(CanonicalValue::Duration { sec: 0, nanosec: 0 }),
            other => Err(JsonCodecError::InvalidJson(format!(
                "expected duration object, got {other}"
            ))),
        },
        FieldType::Array { element, length } => decode_array(value, element, length, resolver),
        FieldType::Complex { type_name } => {
            let nested = resolver
                .get(type_name)
                .ok_or_else(|| JsonCodecError::UnknownComplex(type_name.clone()))?;
            json_to_canonical_with_schema(value, nested, resolver)
        }
    }
}

fn decode_array(
    value: &JsonValue,
    element: &FieldType,
    length: &ArrayLength,
    resolver: &Resolver,
) -> JsonCodecResult<CanonicalValue> {
    let items = match value {
        JsonValue::Array(items) => items,
        JsonValue::Null => return Ok(CanonicalValue::Array(Vec::new())),
        JsonValue::String(s)
            if matches!(
                element,
                FieldType::Primitive(PrimitiveType::Uint8 | PrimitiveType::Byte)
            ) =>
        {
            let bytes = decode_base64_loose(s)?;
            return Ok(CanonicalValue::Bytes(bytes));
        }
        other => {
            return Err(JsonCodecError::InvalidJson(format!(
                "expected array, got {other}"
            )))
        }
    };
    let _ = length;
    if let FieldType::Primitive(PrimitiveType::Uint8 | PrimitiveType::Byte) = element {
        let mut buffer = Vec::with_capacity(items.len());
        for v in items {
            let byte = v
                .as_u64()
                .ok_or_else(|| JsonCodecError::InvalidJson("uint8 array element".into()))?;
            buffer.push(byte as u8);
        }
        return Ok(CanonicalValue::Bytes(buffer));
    }
    let mut out = Vec::with_capacity(items.len());
    for v in items {
        out.push(decode_field(v, element, resolver)?);
    }
    Ok(CanonicalValue::Array(out))
}

fn decode_primitive(
    value: &JsonValue,
    primitive: PrimitiveType,
) -> JsonCodecResult<CanonicalValue> {
    let n = match value {
        JsonValue::Number(n) => n,
        JsonValue::Bool(b) => return Ok(CanonicalValue::Bool(*b)),
        JsonValue::Null => return Ok(CanonicalValue::Null),
        other => {
            return Err(JsonCodecError::InvalidJson(format!(
                "expected number for {primitive:?}, got {other}"
            )))
        }
    };
    Ok(match primitive {
        PrimitiveType::Bool => CanonicalValue::Bool(n.as_u64() != Some(0)),
        PrimitiveType::Byte
        | PrimitiveType::Uint8
        | PrimitiveType::Uint16
        | PrimitiveType::Uint32
        | PrimitiveType::Uint64 => CanonicalValue::Uint(
            n.as_u64()
                .ok_or_else(|| JsonCodecError::InvalidJson(format!("u64 from {n}")))?,
        ),
        PrimitiveType::Char
        | PrimitiveType::Int8
        | PrimitiveType::Int16
        | PrimitiveType::Int32
        | PrimitiveType::Int64 => CanonicalValue::Int(
            n.as_i64()
                .ok_or_else(|| JsonCodecError::InvalidJson(format!("i64 from {n}")))?,
        ),
        PrimitiveType::Float32 => CanonicalValue::F32(
            n.as_f64()
                .ok_or_else(|| JsonCodecError::InvalidJson(format!("f32 from {n}")))?
                as f32,
        ),
        PrimitiveType::Float64 => CanonicalValue::F64(
            n.as_f64()
                .ok_or_else(|| JsonCodecError::InvalidJson(format!("f64 from {n}")))?,
        ),
    })
}

fn time_or_null(
    obj: &serde_json::Map<String, JsonValue>,
    duration: bool,
) -> JsonCodecResult<CanonicalValue> {
    if let Some(value) = try_time_object(obj) {
        if duration {
            if let CanonicalValue::Time { sec, nanosec } = value {
                return Ok(CanonicalValue::Duration { sec, nanosec });
            }
        }
        return Ok(value);
    }
    Err(JsonCodecError::InvalidJson(
        "expected {sec, nanosec} or {secs, nsecs}".into(),
    ))
}

fn decode_base64_loose(input: &str) -> JsonCodecResult<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = input.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let mut chunk = [0u8; 4];
    let mut chunk_len = 0;
    for &c in &bytes {
        if c == b'=' {
            break;
        }
        let v = val(c)
            .ok_or_else(|| JsonCodecError::InvalidJson(format!("invalid base64 char '{c}'")))?;
        chunk[chunk_len] = v;
        chunk_len += 1;
        if chunk_len == 4 {
            out.push((chunk[0] << 2) | (chunk[1] >> 4));
            out.push((chunk[1] << 4) | (chunk[2] >> 2));
            out.push((chunk[2] << 6) | chunk[3]);
            chunk_len = 0;
        }
    }
    match chunk_len {
        2 => out.push((chunk[0] << 2) | (chunk[1] >> 4)),
        3 => {
            out.push((chunk[0] << 2) | (chunk[1] >> 4));
            out.push((chunk[1] << 4) | (chunk[2] >> 2));
        }
        _ => {}
    }
    Ok(out)
}

fn try_time_object(obj: &serde_json::Map<String, JsonValue>) -> Option<CanonicalValue> {
    if obj.len() == 2 {
        if let (Some(s), Some(n)) = (obj.get("sec"), obj.get("nanosec")) {
            if let (Some(sec), Some(nanosec)) = (s.as_i64(), n.as_u64()) {
                return Some(CanonicalValue::Time {
                    sec: sec as i32,
                    nanosec: nanosec as u32,
                });
            }
        }
        if let (Some(s), Some(n)) = (obj.get("secs"), obj.get("nsecs")) {
            if let (Some(sec), Some(nanosec)) = (s.as_i64(), n.as_u64()) {
                return Some(CanonicalValue::Time {
                    sec: sec as i32,
                    nanosec: nanosec as u32,
                });
            }
        }
    }
    None
}

pub fn canonical_to_json(value: &CanonicalValue, ros1_time_naming: bool) -> JsonValue {
    match value {
        CanonicalValue::Null => JsonValue::Null,
        CanonicalValue::Bool(b) => JsonValue::Bool(*b),
        CanonicalValue::Int(i) => JsonValue::Number((*i).into()),
        CanonicalValue::Uint(u) => JsonValue::Number((*u).into()),
        CanonicalValue::F32(f) => serde_json::Number::from_f64(*f as f64)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        CanonicalValue::F64(f) => serde_json::Number::from_f64(*f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        CanonicalValue::String(s) => JsonValue::String(s.clone()),
        CanonicalValue::Bytes(bytes) => JsonValue::Array(
            bytes
                .iter()
                .map(|byte| JsonValue::Number((*byte as u64).into()))
                .collect(),
        ),
        CanonicalValue::Array(items) => JsonValue::Array(
            items
                .iter()
                .map(|v| canonical_to_json(v, ros1_time_naming))
                .collect(),
        ),
        CanonicalValue::Struct(fields) => {
            let mut out = serde_json::Map::with_capacity(fields.len());
            for (key, value) in fields {
                out.insert(key.clone(), canonical_to_json(value, ros1_time_naming));
            }
            JsonValue::Object(out)
        }
        CanonicalValue::Time { sec, nanosec } | CanonicalValue::Duration { sec, nanosec } => {
            let mut obj = serde_json::Map::new();
            if ros1_time_naming {
                obj.insert("secs".into(), JsonValue::Number((*sec as i64).into()));
                obj.insert("nsecs".into(), JsonValue::Number((*nanosec as u64).into()));
            } else {
                obj.insert("sec".into(), JsonValue::Number((*sec as i64).into()));
                obj.insert(
                    "nanosec".into(),
                    JsonValue::Number((*nanosec as u64).into()),
                );
            }
            JsonValue::Object(obj)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ros2_time_object_lifts_to_canonical_time() {
        let json: JsonValue = serde_json::from_str(r#"{"sec": 5, "nanosec": 100}"#).unwrap();
        match json_to_canonical(&json).unwrap() {
            CanonicalValue::Time { sec, nanosec } => {
                assert_eq!(sec, 5);
                assert_eq!(nanosec, 100);
            }
            other => panic!("expected Time, got {other:?}"),
        }
    }

    #[test]
    fn ros1_time_object_lifts_to_canonical_time() {
        let json: JsonValue = serde_json::from_str(r#"{"secs": 7, "nsecs": 999}"#).unwrap();
        match json_to_canonical(&json).unwrap() {
            CanonicalValue::Time { sec, nanosec } => {
                assert_eq!(sec, 7);
                assert_eq!(nanosec, 999);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn canonical_time_emits_ros2_naming_by_default() {
        let value = CanonicalValue::Time {
            sec: 5,
            nanosec: 100,
        };
        let json = canonical_to_json(&value, false);
        assert_eq!(json["sec"], JsonValue::Number(5.into()));
        assert_eq!(json["nanosec"], JsonValue::Number(100u64.into()));
    }

    #[test]
    fn canonical_time_emits_ros1_naming_when_requested() {
        let value = CanonicalValue::Time {
            sec: 5,
            nanosec: 100,
        };
        let json = canonical_to_json(&value, true);
        assert_eq!(json["secs"], JsonValue::Number(5.into()));
        assert_eq!(json["nsecs"], JsonValue::Number(100u64.into()));
    }

    #[test]
    fn nested_struct_roundtrips() {
        let json: JsonValue =
            serde_json::from_str(r#"{"frame_id": "map", "stamp": {"sec": 1, "nanosec": 2}}"#)
                .unwrap();
        let canonical = json_to_canonical(&json).unwrap();
        let back = canonical_to_json(&canonical, false);
        assert_eq!(back, json);
    }
}
