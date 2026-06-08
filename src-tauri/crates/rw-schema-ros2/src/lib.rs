use std::collections::BTreeMap;

use rw_canonical::{
    ArrayLength, CanonicalValue, ConstantDef, FieldDef, FieldType, MessageDef, ParsedSchema,
    PrimitiveType, SchemaKind,
};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("expected {expected} section(s) for {kind}, got {got}")]
    SectionCount {
        kind: &'static str,
        expected: &'static str,
        got: usize,
    },
    #[error("line {line}: {detail}")]
    Line { line: usize, detail: String },
}

pub type ParseResult<T> = Result<T, ParseError>;

pub fn parse(kind: SchemaKind, source: &str) -> ParseResult<ParsedSchema> {
    let parts = split_sections(source);
    match (kind, parts.as_slice()) {
        (SchemaKind::Message, [single]) => Ok(ParsedSchema::Message(parse_message(single)?)),
        (SchemaKind::Service, [request, response]) => Ok(ParsedSchema::Service {
            request: parse_message(request)?,
            response: parse_message(response)?,
        }),
        (SchemaKind::Action, [goal, result, feedback]) => Ok(ParsedSchema::Action {
            goal: parse_message(goal)?,
            result: parse_message(result)?,
            feedback: parse_message(feedback)?,
        }),
        (kind, parts) => Err(ParseError::SectionCount {
            kind: kind.as_str(),
            expected: expected_section_count(kind),
            got: parts.len(),
        }),
    }
}

fn expected_section_count(kind: SchemaKind) -> &'static str {
    match kind {
        SchemaKind::Message => "1",
        SchemaKind::Service => "2",
        SchemaKind::Action => "3",
    }
}

fn split_sections(source: &str) -> Vec<String> {
    let mut sections: Vec<Vec<&str>> = vec![Vec::new()];
    for line in source.lines() {
        if line.trim() == "---" {
            sections.push(Vec::new());
        } else {
            sections.last_mut().expect("section").push(line);
        }
    }
    sections
        .into_iter()
        .map(|lines| {
            let mut joined = lines.join("\n");
            joined.push('\n');
            joined
        })
        .collect()
}

fn parse_message(text: &str) -> ParseResult<MessageDef> {
    let mut message = MessageDef::default();
    let mut pending_comment: Vec<String> = Vec::new();

    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let (body, trailing) = split_off_comment(raw_line);
        let trimmed = body.trim();
        if trimmed.is_empty() {
            if let Some(comment) = trailing {
                pending_comment.push(comment);
            }
            continue;
        }
        match parse_field_or_constant(trimmed, line_number)? {
            ParsedLine::Field(mut field) => {
                let mut joined = std::mem::take(&mut pending_comment);
                if let Some(trailing) = trailing {
                    joined.push(trailing);
                }
                if !joined.is_empty() {
                    field.comment = Some(joined.join("\n"));
                }
                message.fields.push(field);
            }
            ParsedLine::Constant(constant) => {
                pending_comment.clear();
                message.constants.push(constant);
            }
        }
    }
    Ok(message)
}

enum ParsedLine {
    Field(FieldDef),
    Constant(ConstantDef),
}

fn split_off_comment(line: &str) -> (&str, Option<String>) {
    if let Some(position) = line.find('#') {
        let body = &line[..position];
        let comment = line[position + 1..].trim();
        (
            body,
            if comment.is_empty() {
                None
            } else {
                Some(comment.to_string())
            },
        )
    } else {
        (line, None)
    }
}

fn parse_field_or_constant(line: &str, line_number: usize) -> ParseResult<ParsedLine> {
    let mut tokens = line.splitn(2, char::is_whitespace);
    let type_token = tokens.next().ok_or_else(|| ParseError::Line {
        line: line_number,
        detail: "expected type token".into(),
    })?;
    let remainder = tokens
        .next()
        .ok_or_else(|| ParseError::Line {
            line: line_number,
            detail: format!("missing field name after type '{type_token}'"),
        })?
        .trim();

    let field_type = parse_type(type_token, line_number)?;

    if let Some(position) = remainder.find('=') {
        let name = remainder[..position].trim();
        let value_text = remainder[position + 1..].trim();
        let value = parse_default_value(&field_type, value_text, line_number)?;
        return Ok(ParsedLine::Constant(ConstantDef {
            name: name.into(),
            field_type,
            value,
        }));
    }

    let mut split = remainder.splitn(2, char::is_whitespace);
    let name = split.next().ok_or_else(|| ParseError::Line {
        line: line_number,
        detail: "missing field name".into(),
    })?;
    let default_text = split.next().map(str::trim);
    let default = match default_text {
        Some(text) if !text.is_empty() => {
            Some(parse_default_value(&field_type, text, line_number)?)
        }
        _ => None,
    };

    Ok(ParsedLine::Field(FieldDef {
        name: name.into(),
        field_type,
        default,
        comment: None,
    }))
}

fn parse_type(token: &str, line_number: usize) -> ParseResult<FieldType> {
    let (base_token, array_suffix) = split_array_suffix(token);
    let base = parse_base_type(base_token, line_number)?;
    Ok(match array_suffix {
        Some(length) => FieldType::Array {
            element: Box::new(base),
            length,
        },
        None => base,
    })
}

fn split_array_suffix(token: &str) -> (&str, Option<ArrayLength>) {
    if let Some(open) = token.rfind('[') {
        if token.ends_with(']') {
            let inside = &token[open + 1..token.len() - 1];
            let length = if inside.is_empty() {
                ArrayLength::Unbounded
            } else if let Some(rest) = inside.strip_prefix("<=") {
                rest.parse::<usize>()
                    .ok()
                    .map(ArrayLength::Bounded)
                    .unwrap_or(ArrayLength::Unbounded)
            } else {
                inside
                    .parse::<usize>()
                    .ok()
                    .map(ArrayLength::Fixed)
                    .unwrap_or(ArrayLength::Unbounded)
            };
            return (&token[..open], Some(length));
        }
    }
    (token, None)
}

fn parse_base_type(token: &str, line_number: usize) -> ParseResult<FieldType> {
    if let Some(rest) = token.strip_prefix("string") {
        return Ok(FieldType::String {
            bound: parse_bound(rest, line_number)?,
        });
    }
    if let Some(rest) = token.strip_prefix("wstring") {
        return Ok(FieldType::WString {
            bound: parse_bound(rest, line_number)?,
        });
    }
    if token == "time" {
        return Ok(FieldType::Time);
    }
    if token == "duration" {
        return Ok(FieldType::Duration);
    }
    if let Some(primitive) = PrimitiveType::parse(token) {
        return Ok(FieldType::Primitive(primitive));
    }
    if token.contains('/') {
        let normalised = normalize_complex_name(token);
        if normalised == "builtin_interfaces/Time" {
            return Ok(FieldType::Time);
        }
        if normalised == "builtin_interfaces/Duration" {
            return Ok(FieldType::Duration);
        }
        return Ok(FieldType::Complex {
            type_name: normalised,
        });
    }
    Err(ParseError::Line {
        line: line_number,
        detail: format!("unknown type token '{token}'"),
    })
}

fn parse_bound(rest: &str, line_number: usize) -> ParseResult<Option<usize>> {
    if rest.is_empty() {
        return Ok(None);
    }
    let body = rest.strip_prefix("<=").ok_or_else(|| ParseError::Line {
        line: line_number,
        detail: format!("invalid string suffix '{rest}'"),
    })?;
    body.parse::<usize>()
        .map(Some)
        .map_err(|err| ParseError::Line {
            line: line_number,
            detail: format!("bad string bound '{body}': {err}"),
        })
}

fn normalize_complex_name(token: &str) -> String {
    let segments: Vec<&str> = token.split('/').collect();
    if segments.len() == 3 && matches!(segments[1], "msg" | "srv" | "action") {
        format!("{}/{}", segments[0], segments[2])
    } else {
        token.into()
    }
}

fn parse_default_value(
    field_type: &FieldType,
    text: &str,
    line_number: usize,
) -> ParseResult<CanonicalValue> {
    match field_type {
        FieldType::Primitive(PrimitiveType::Bool) => {
            Ok(CanonicalValue::Bool(parse_bool(text, line_number)?))
        }
        FieldType::Primitive(primitive) => parse_numeric(*primitive, text, line_number),
        FieldType::String { .. } | FieldType::WString { .. } => {
            Ok(CanonicalValue::String(strip_quotes(text)))
        }
        _ => Ok(CanonicalValue::String(text.into())),
    }
}

fn parse_bool(text: &str, line_number: usize) -> ParseResult<bool> {
    match text {
        "true" | "True" | "1" => Ok(true),
        "false" | "False" | "0" => Ok(false),
        other => Err(ParseError::Line {
            line: line_number,
            detail: format!("invalid bool default '{other}'"),
        }),
    }
}

fn parse_numeric(
    primitive: PrimitiveType,
    text: &str,
    line_number: usize,
) -> ParseResult<CanonicalValue> {
    let err = |what: &str, e: std::num::ParseFloatError| ParseError::Line {
        line: line_number,
        detail: format!("bad {what} '{text}': {e}"),
    };
    let int_err = |what: &str, e: std::num::ParseIntError| ParseError::Line {
        line: line_number,
        detail: format!("bad {what} '{text}': {e}"),
    };
    match primitive {
        PrimitiveType::Float32 => text
            .parse::<f32>()
            .map(CanonicalValue::F32)
            .map_err(|e| err("f32", e)),
        PrimitiveType::Float64 => text
            .parse::<f64>()
            .map(CanonicalValue::F64)
            .map_err(|e| err("f64", e)),
        PrimitiveType::Int8
        | PrimitiveType::Int16
        | PrimitiveType::Int32
        | PrimitiveType::Int64 => text
            .parse::<i64>()
            .map(CanonicalValue::Int)
            .map_err(|e| int_err("int", e)),
        _ => text
            .parse::<u64>()
            .map(CanonicalValue::Uint)
            .map_err(|e| int_err("uint", e)),
    }
}

fn strip_quotes(text: &str) -> String {
    let trimmed = text.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2)
    {
        return trimmed[1..trimmed.len() - 1].into();
    }
    trimmed.into()
}

pub fn collect_dependencies(parsed: &ParsedSchema) -> Vec<String> {
    let mut order: Vec<String> = Vec::new();
    let mut seen: BTreeMap<String, ()> = BTreeMap::new();
    for message in parsed.parts() {
        for field in &message.fields {
            let mut deps = Vec::new();
            field.field_type.complex_dependencies(&mut deps);
            for dep in deps {
                if seen.insert(dep.clone(), ()).is_none() {
                    order.push(dep);
                }
            }
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_parses() {
        let parsed = parse(
            SchemaKind::Message,
            "builtin_interfaces/Time stamp\nstring frame_id\n",
        )
        .unwrap();
        match parsed {
            ParsedSchema::Message(message) => {
                assert_eq!(message.fields.len(), 2);
                assert!(matches!(message.fields[0].field_type, FieldType::Time));
            }
            _ => panic!("expected message"),
        }
    }

    #[test]
    fn builtin_duration_resolves_to_canonical_duration() {
        let parsed = parse(
            SchemaKind::Message,
            "builtin_interfaces/Duration lifetime\n",
        )
        .unwrap();
        match parsed {
            ParsedSchema::Message(message) => {
                assert!(matches!(message.fields[0].field_type, FieldType::Duration));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn ros2_msg_segment_canonicalises() {
        let parsed = parse(SchemaKind::Message, "geometry_msgs/msg/Point position\n").unwrap();
        match parsed {
            ParsedSchema::Message(message) => assert!(matches!(
                message.fields[0].field_type,
                FieldType::Complex { ref type_name } if type_name == "geometry_msgs/Point"
            )),
            _ => panic!("expected message"),
        }
    }

    #[test]
    fn arrays_parse_in_all_three_flavors() {
        let parsed = parse(
            SchemaKind::Message,
            "float32[] ranges\nfloat32[<=10] subset\nfloat64[16] matrix\n",
        )
        .unwrap();
        if let ParsedSchema::Message(message) = parsed {
            assert!(matches!(
                message.fields[0].field_type,
                FieldType::Array {
                    length: ArrayLength::Unbounded,
                    ..
                }
            ));
            assert!(matches!(
                message.fields[1].field_type,
                FieldType::Array {
                    length: ArrayLength::Bounded(10),
                    ..
                }
            ));
            assert!(matches!(
                message.fields[2].field_type,
                FieldType::Array {
                    length: ArrayLength::Fixed(16),
                    ..
                }
            ));
        }
    }

    #[test]
    fn services_split_on_dashes() {
        let parsed = parse(SchemaKind::Service, "int64 a\nint64 b\n---\nint64 sum\n").unwrap();
        match parsed {
            ParsedSchema::Service { request, response } => {
                assert_eq!(request.fields.len(), 2);
                assert_eq!(response.fields.len(), 1);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn dependencies_are_unique() {
        let parsed = parse(
            SchemaKind::Message,
            "geometry_msgs/Pose pose\ngeometry_msgs/Pose other\nstd_msgs/Header header\n",
        )
        .unwrap();
        assert_eq!(
            collect_dependencies(&parsed),
            vec!["geometry_msgs/Pose", "std_msgs/Header"]
        );
    }
}
