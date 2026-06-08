use std::collections::BTreeMap;

use crate::domain::Value;
use crate::schema::{
    ArrayLength, ConstantDef, FieldDef, FieldType, MessageDef, ParsedSchema, PrimitiveType,
    SchemaKind,
};
use crate::{CoreError, CoreResult};

pub fn parse(kind: SchemaKind, source: &str) -> CoreResult<ParsedSchema> {
    parse_with_package(kind, source, None)
}

pub fn parse_with_package(
    kind: SchemaKind,
    source: &str,
    default_package: Option<&str>,
) -> CoreResult<ParsedSchema> {
    let parts = split_sections(source);
    match (kind, parts.as_slice()) {
        (SchemaKind::Message, [single]) => Ok(ParsedSchema::Message(parse_message(
            single,
            default_package,
        )?)),
        (SchemaKind::Service, [request, response]) => Ok(ParsedSchema::Service {
            request: parse_message(request, default_package)?,
            response: parse_message(response, default_package)?,
        }),
        (SchemaKind::Action, [goal, result, feedback]) => Ok(ParsedSchema::Action {
            goal: parse_message(goal, default_package)?,
            result: parse_message(result, default_package)?,
            feedback: parse_message(feedback, default_package)?,
        }),
        (kind, parts) => Err(CoreError::Schema(format!(
            "expected {} section(s) for {}, got {}",
            expected_section_count(kind),
            kind.as_str(),
            parts.len(),
        ))),
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
            sections
                .last_mut()
                .expect("at least one section")
                .push(line);
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

fn parse_message(text: &str, default_package: Option<&str>) -> CoreResult<MessageDef> {
    let mut message = MessageDef::default();
    let mut pending_comment: Vec<String> = Vec::new();

    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let (body, trailing_comment) = split_off_comment(raw_line);
        let trimmed = body.trim();

        if trimmed.is_empty() {
            if let Some(comment) = trailing_comment {
                pending_comment.push(comment);
            }
            continue;
        }

        let parsed = parse_field_or_constant(trimmed, line_number, default_package)?;
        match parsed {
            ParsedLine::Field(mut field) => {
                let mut joined = std::mem::take(&mut pending_comment);
                if let Some(trailing) = trailing_comment {
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
    if let Some(hash_position) = line.find('#') {
        let body = &line[..hash_position];
        let comment = line[hash_position + 1..].trim();
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

fn parse_field_or_constant(
    line: &str,
    line_number: usize,
    default_package: Option<&str>,
) -> CoreResult<ParsedLine> {
    let mut tokens = line.splitn(2, char::is_whitespace);
    let type_token = tokens
        .next()
        .ok_or_else(|| CoreError::Schema(format!("line {line_number}: expected type token")))?;
    let remainder = tokens
        .next()
        .ok_or_else(|| {
            CoreError::Schema(format!(
                "line {line_number}: missing field name after type '{type_token}'"
            ))
        })?
        .trim();

    let field_type = parse_type(type_token, line_number, default_package)?;

    if let Some(equals_position) = remainder.find('=') {
        let name = remainder[..equals_position].trim();
        let value_text = remainder[equals_position + 1..].trim();
        let value = parse_value(&field_type, value_text, line_number)?;
        return Ok(ParsedLine::Constant(ConstantDef {
            name: name.to_string(),
            field_type,
            value,
        }));
    }

    let mut name_and_default = remainder.splitn(2, char::is_whitespace);
    let name = name_and_default
        .next()
        .ok_or_else(|| CoreError::Schema(format!("line {line_number}: missing field name")))?;
    let default_text = name_and_default.next().map(str::trim);
    let default = match default_text {
        Some(text) if !text.is_empty() => Some(parse_value(&field_type, text, line_number)?),
        _ => None,
    };

    Ok(ParsedLine::Field(FieldDef {
        name: name.to_string(),
        field_type,
        default,
        comment: None,
    }))
}

fn parse_type(
    token: &str,
    line_number: usize,
    default_package: Option<&str>,
) -> CoreResult<FieldType> {
    let (base_token, array_suffix) = split_array_suffix(token);
    let base = parse_base_type(base_token, line_number, default_package)?;
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

fn parse_base_type(
    token: &str,
    line_number: usize,
    default_package: Option<&str>,
) -> CoreResult<FieldType> {
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
        return Ok(FieldType::Complex {
            type_name: normalize_complex_name(token),
        });
    }
    if let Some(package) = default_package {
        let qualified = if token == "Header" {
            "std_msgs/Header".to_string()
        } else {
            format!("{package}/{token}")
        };
        return Ok(FieldType::Complex {
            type_name: normalize_complex_name(&qualified),
        });
    }
    Err(CoreError::Schema(format!(
        "line {line_number}: unknown type token '{token}'"
    )))
}

fn parse_bound(rest: &str, line_number: usize) -> CoreResult<Option<usize>> {
    if rest.is_empty() {
        return Ok(None);
    }
    let bound_text = rest.strip_prefix("<=").ok_or_else(|| {
        CoreError::Schema(format!(
            "line {line_number}: invalid string suffix '{rest}'"
        ))
    })?;
    let bound = bound_text.parse::<usize>().map_err(|err| {
        CoreError::Schema(format!(
            "line {line_number}: bad string bound '{bound_text}': {err}"
        ))
    })?;
    Ok(Some(bound))
}

fn normalize_complex_name(token: &str) -> String {
    let segments: Vec<&str> = token.split('/').collect();
    if segments.len() == 3
        && (segments[1] == "msg" || segments[1] == "srv" || segments[1] == "action")
    {
        format!("{}/{}", segments[0], segments[2])
    } else {
        token.to_string()
    }
}

fn parse_value(field_type: &FieldType, text: &str, line_number: usize) -> CoreResult<Value> {
    match field_type {
        FieldType::Primitive(PrimitiveType::Bool) => parse_bool(text, line_number).map(Value::Bool),
        FieldType::Primitive(primitive) => parse_numeric(*primitive, text, line_number),
        FieldType::String { .. } | FieldType::WString { .. } => {
            Ok(Value::String(strip_quotes(text)))
        }
        FieldType::Array { .. }
        | FieldType::Complex { .. }
        | FieldType::Time
        | FieldType::Duration => Ok(Value::String(text.to_string())),
    }
}

fn parse_bool(text: &str, line_number: usize) -> CoreResult<bool> {
    match text {
        "true" | "True" | "1" => Ok(true),
        "false" | "False" | "0" => Ok(false),
        other => Err(CoreError::Schema(format!(
            "line {line_number}: invalid bool default '{other}'"
        ))),
    }
}

fn parse_numeric(primitive: PrimitiveType, text: &str, line_number: usize) -> CoreResult<Value> {
    match primitive {
        PrimitiveType::Float32 => text.parse::<f32>().map(Value::F32).map_err(|err| {
            CoreError::Schema(format!("line {line_number}: bad f32 '{text}': {err}"))
        }),
        PrimitiveType::Float64 => text.parse::<f64>().map(Value::F64).map_err(|err| {
            CoreError::Schema(format!("line {line_number}: bad f64 '{text}': {err}"))
        }),
        PrimitiveType::Int8
        | PrimitiveType::Int16
        | PrimitiveType::Int32
        | PrimitiveType::Int64 => text.parse::<i64>().map(Value::Int).map_err(|err| {
            CoreError::Schema(format!("line {line_number}: bad int '{text}': {err}"))
        }),
        PrimitiveType::Uint8
        | PrimitiveType::Uint16
        | PrimitiveType::Uint32
        | PrimitiveType::Uint64
        | PrimitiveType::Byte
        | PrimitiveType::Char => text.parse::<u64>().map(Value::Uint).map_err(|err| {
            CoreError::Schema(format!("line {line_number}: bad uint '{text}': {err}"))
        }),
        PrimitiveType::Bool => unreachable!("handled by caller"),
    }
}

fn strip_quotes(text: &str) -> String {
    let trimmed = text.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2)
    {
        return trimmed[1..trimmed.len() - 1].to_string();
    }
    trimmed.to_string()
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

    fn parse_msg(text: &str) -> ParsedSchema {
        parse(SchemaKind::Message, text).expect("message parses")
    }

    #[test]
    fn header_parses() {
        let parsed = parse_msg("builtin_interfaces/Time stamp\nstring frame_id\n");
        match &parsed {
            ParsedSchema::Message(message) => {
                assert_eq!(message.fields.len(), 2);
                assert_eq!(message.fields[0].name, "stamp");
                assert!(matches!(
                    message.fields[0].field_type,
                    FieldType::Complex { ref type_name } if type_name == "builtin_interfaces/Time"
                ));
                assert_eq!(message.fields[1].name, "frame_id");
                assert!(matches!(
                    message.fields[1].field_type,
                    FieldType::String { bound: None }
                ));
            }
            _ => panic!("expected message"),
        }
    }

    #[test]
    fn arrays_parse_in_all_three_flavors() {
        let parsed = parse_msg("float32[] ranges\nfloat32[<=10] subset\nfloat64[16] matrix\n");
        if let ParsedSchema::Message(message) = parsed {
            assert!(matches!(
                message.fields[0].field_type,
                FieldType::Array { ref element, length: ArrayLength::Unbounded }
                    if matches!(**element, FieldType::Primitive(PrimitiveType::Float32))
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
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn constants_distinguish_from_defaults() {
        let parsed = parse_msg("uint8 INT8=1\nuint8 datatype 7\n");
        if let ParsedSchema::Message(message) = parsed {
            assert_eq!(message.constants.len(), 1);
            assert_eq!(message.constants[0].name, "INT8");
            assert_eq!(message.fields.len(), 1);
            assert_eq!(message.fields[0].name, "datatype");
            assert_eq!(message.fields[0].default, Some(Value::Uint(7)));
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn comments_attach_to_following_field() {
        let parsed = parse_msg("# Header for the scan\nstd_msgs/Header header\n");
        if let ParsedSchema::Message(message) = parsed {
            assert_eq!(
                message.fields[0].comment.as_deref(),
                Some("Header for the scan")
            );
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn trailing_comment_is_attached() {
        let parsed = parse_msg("uint32 height # vertical resolution\n");
        if let ParsedSchema::Message(message) = parsed {
            assert_eq!(
                message.fields[0].comment.as_deref(),
                Some("vertical resolution")
            );
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn service_splits_on_separator() {
        let parsed = parse(SchemaKind::Service, "int64 a\nint64 b\n---\nint64 sum\n").unwrap();
        match parsed {
            ParsedSchema::Service { request, response } => {
                assert_eq!(request.fields.len(), 2);
                assert_eq!(response.fields.len(), 1);
            }
            _ => panic!("expected service"),
        }
    }

    #[test]
    fn action_splits_on_two_separators() {
        let parsed = parse(
            SchemaKind::Action,
            "int32 order\n---\nint32[] sequence\n---\nint32[] partial_sequence\n",
        )
        .unwrap();
        match parsed {
            ParsedSchema::Action {
                goal,
                result,
                feedback,
            } => {
                assert_eq!(goal.fields.len(), 1);
                assert_eq!(result.fields.len(), 1);
                assert_eq!(feedback.fields.len(), 1);
            }
            _ => panic!("expected action"),
        }
    }

    #[test]
    fn complex_pkg_msg_type_collapses() {
        let parsed = parse_msg("geometry_msgs/msg/Point position\n");
        if let ParsedSchema::Message(message) = parsed {
            assert!(matches!(
                message.fields[0].field_type,
                FieldType::Complex { ref type_name } if type_name == "geometry_msgs/Point"
            ));
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn time_and_duration_are_recognized() {
        let parsed = parse_msg("time stamp\nduration lifetime\n");
        if let ParsedSchema::Message(message) = parsed {
            assert!(matches!(message.fields[0].field_type, FieldType::Time));
            assert!(matches!(message.fields[1].field_type, FieldType::Duration));
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn string_with_bound_parses() {
        let parsed = parse_msg("string<=64 frame_id\n");
        if let ParsedSchema::Message(message) = parsed {
            assert!(matches!(
                message.fields[0].field_type,
                FieldType::String { bound: Some(64) }
            ));
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn unknown_type_token_errors() {
        let result = parse(SchemaKind::Message, "weirdtype field\n");
        assert!(matches!(result, Err(CoreError::Schema(_))));
    }

    #[test]
    fn dependencies_are_collected_unique() {
        let parsed = parse_msg(
            "geometry_msgs/Pose pose\ngeometry_msgs/Pose other_pose\nstd_msgs/Header header\n",
        );
        let deps = collect_dependencies(&parsed);
        assert_eq!(deps, vec!["geometry_msgs/Pose", "std_msgs/Header"]);
    }
}
