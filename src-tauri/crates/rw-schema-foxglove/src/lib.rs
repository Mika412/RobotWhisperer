use std::collections::HashMap;

use rw_canonical::hash::canonical_schema_id_with_deps;
use rw_canonical::{
    canonical_schema_id, CanonicalSchema, Dialect, MessageDef, ParsedSchema, SchemaId, SchemaKind,
};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FoxgloveSchemaError {
    #[error("dependency section is missing 'MSG: <name>' header")]
    MissingDependencyHeader,
    #[error("ros2 parse error: {0}")]
    Parse(#[from] rw_schema_ros2::ParseError),
}

pub type FoxgloveSchemaResult<T> = Result<T, FoxgloveSchemaError>;

const SEPARATOR: &str =
    "================================================================================";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcatenatedSchema {
    pub root_text: String,
    pub dependencies: Vec<(String, String)>,
}

pub fn split_concatenated(definition: &str) -> ConcatenatedSchema {
    let mut iter = definition.split(SEPARATOR);
    let root = iter.next().unwrap_or("").trim().to_string();
    let mut dependencies = Vec::new();
    for section in iter {
        let trimmed = section.trim_start_matches('\n');
        if trimmed.is_empty() {
            continue;
        }
        let mut lines = trimmed.lines();
        let header = lines.next().unwrap_or("");
        let name = header.strip_prefix("MSG:").map(str::trim).unwrap_or("");
        let body: String = lines.collect::<Vec<_>>().join("\n");
        dependencies.push((name.to_string(), body));
    }
    ConcatenatedSchema {
        root_text: root,
        dependencies,
    }
}

pub fn parse_concatenated(
    name: &str,
    kind: SchemaKind,
    definition: &str,
    dialect: Dialect,
) -> FoxgloveSchemaResult<CanonicalSchema> {
    let split = split_concatenated(definition);
    let root_pkg = package_of(name);
    let qualified_root = qualify_bare_types(&split.root_text, root_pkg);
    let parsed = rw_schema_ros2::parse(kind, &qualified_root)?;
    let dependency_ids: Vec<SchemaId> = split
        .dependencies
        .iter()
        .map(|(_, text)| canonical_schema_id(text))
        .collect();
    let id = canonical_schema_id_with_deps(&split.root_text, split.dependencies.iter().cloned());
    let viz_role = rw_canonical::viz_role_for_schema(name);
    Ok(CanonicalSchema {
        id,
        name: name.to_string(),
        kind,
        dialect,
        definition: definition.to_string(),
        parsed,
        dependencies: dependency_ids,
        viz_role,
    })
}

pub fn parse_concatenated_with_resolver(
    name: &str,
    kind: SchemaKind,
    definition: &str,
    dialect: Dialect,
) -> FoxgloveSchemaResult<(CanonicalSchema, HashMap<String, MessageDef>)> {
    let schema = parse_concatenated(name, kind, definition, dialect)?;
    let split = split_concatenated(definition);
    let mut resolver: HashMap<String, MessageDef> = HashMap::new();
    for (dep_name, body) in split.dependencies {
        let normalized = normalise_pkg_type(&dep_name);
        let dep_pkg = package_of(&normalized).to_string();
        let qualified = qualify_bare_types(&body, &dep_pkg);
        match rw_schema_ros2::parse(SchemaKind::Message, &qualified) {
            Ok(ParsedSchema::Message(message)) => {
                resolver.insert(normalized, message);
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }
    Ok((schema, resolver))
}

fn normalise_pkg_type(name: &str) -> String {
    let parts: Vec<&str> = name.split('/').collect();
    if parts.len() == 3 && matches!(parts[1], "msg" | "srv" | "action") {
        format!("{}/{}", parts[0], parts[2])
    } else {
        name.to_string()
    }
}

pub fn fallback_id(parsed: &ParsedSchema, definition: &str) -> SchemaId {
    let _ = parsed;
    canonical_schema_id(definition)
}

fn package_of(name: &str) -> &str {
    name.split_once('/').map(|(pkg, _)| pkg).unwrap_or("")
}

fn qualify_bare_types(section: &str, pkg: &str) -> String {
    if pkg.is_empty() {
        return section.to_string();
    }
    let mut out = String::with_capacity(section.len());
    for line in section.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if trimmed
            .split_whitespace()
            .nth(1)
            .is_some_and(|t| t.contains('='))
        {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let indent_len = line.len() - trimmed.len();
        let (type_token, rest) = match trimmed.split_once(char::is_whitespace) {
            Some(pair) => pair,
            None => {
                out.push_str(line);
                out.push('\n');
                continue;
            }
        };
        let rewritten_token = rewrite_type_token(type_token, pkg);
        out.push_str(&line[..indent_len]);
        out.push_str(&rewritten_token);
        out.push(' ');
        out.push_str(rest);
        out.push('\n');
    }
    out
}

fn rewrite_type_token(token: &str, pkg: &str) -> String {
    let (base, array_suffix) = match token.rfind('[') {
        Some(open) if token.ends_with(']') => (&token[..open], &token[open..]),
        _ => (token, ""),
    };
    let qualified = if base.contains('/') || is_builtin_token(base) {
        base.to_string()
    } else if base == "Header" {
        "std_msgs/Header".to_string()
    } else {
        format!("{pkg}/{base}")
    };
    format!("{qualified}{array_suffix}")
}

fn is_builtin_token(token: &str) -> bool {
    if token == "time" || token == "duration" {
        return true;
    }
    if token.starts_with("string") || token.starts_with("wstring") {
        return true;
    }
    matches!(
        token,
        "bool"
            | "byte"
            | "char"
            | "int8"
            | "uint8"
            | "int16"
            | "uint16"
            | "int32"
            | "uint32"
            | "int64"
            | "uint64"
            | "float32"
            | "float64"
    )
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ActionDefinitionParts {
    goal: Option<String>,
    result: Option<String>,
    feedback: Option<String>,
}

impl ActionDefinitionParts {
    pub fn set_goal_from_send_goal_request(&mut self, send_goal_request: &str) {
        if let Some(body) = extract_goal_body(send_goal_request) {
            self.goal = Some(body);
        }
    }

    pub fn set_result_from_get_result_response(&mut self, get_result_response: &str) {
        if let Some(body) = extract_member_body(get_result_response, "result") {
            self.result = Some(body);
        }
    }

    pub fn set_feedback_from_feedback_topic(&mut self, feedback_schema: &str) {
        if let Some(body) = extract_member_body(feedback_schema, "feedback") {
            self.feedback = Some(body);
        }
    }

    pub fn to_action_definition(&self) -> Option<String> {
        let goal = self.goal.as_deref()?;
        let result = self.result.as_deref().unwrap_or("");
        let feedback = self.feedback.as_deref().unwrap_or("");
        Some(format!("{goal}\n---\n{result}\n---\n{feedback}"))
    }
}

fn extract_member_body(concatenated: &str, member: &str) -> Option<String> {
    let split = split_concatenated(concatenated);
    let type_name = split.root_text.lines().find_map(|line| {
        let line = line.split('#').next().unwrap_or("").trim();
        let mut parts = line.split_whitespace();
        let type_token = parts.next()?;
        let field_name = parts.next()?;
        if field_name == member && !type_token.contains('=') {
            Some(type_token.to_string())
        } else {
            None
        }
    })?;
    let suffix = type_name.rsplit('/').next().unwrap_or(type_name.as_str());
    split
        .dependencies
        .into_iter()
        .find(|(dep_name, _)| dep_name == &type_name || dep_name.rsplit('/').next() == Some(suffix))
        .map(|(_, body)| body)
}

fn extract_goal_body(send_goal_request: &str) -> Option<String> {
    if let Some(body) = extract_member_body(send_goal_request, "goal") {
        return Some(body);
    }
    let split = split_concatenated(send_goal_request);
    let body: Vec<&str> = split
        .root_text
        .lines()
        .filter(|line| {
            let trimmed = line.split('#').next().unwrap_or("").trim();
            trimmed.split_whitespace().nth(1) != Some("goal_id")
        })
        .collect();
    let joined = body.join("\n");
    if joined.trim().is_empty() {
        None
    } else {
        Some(joined)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_root_and_dependencies() {
        let text = "std_msgs/Header header\nfloat32 value\n================================================================================\nMSG: std_msgs/Header\nuint32 seq\nstring frame_id\n";
        let split = split_concatenated(text);
        assert!(split.root_text.contains("std_msgs/Header header"));
        assert_eq!(split.dependencies.len(), 1);
        assert_eq!(split.dependencies[0].0, "std_msgs/Header");
        assert!(split.dependencies[0].1.contains("uint32 seq"));
    }

    #[test]
    fn parse_concatenated_assigns_deterministic_id() {
        let text = "std_msgs/Header header\nfloat32 value\n================================================================================\nMSG: std_msgs/Header\nuint32 seq\n";
        let a =
            parse_concatenated("pkg/Foo", SchemaKind::Message, text, Dialect::Foxglove).unwrap();
        let b =
            parse_concatenated("pkg/Foo", SchemaKind::Message, text, Dialect::Foxglove).unwrap();
        assert_eq!(a.id, b.id);
    }

    #[test]
    fn bare_type_token_gets_qualified_with_root_package() {
        let text = "# Pose comment\nPoint position\nQuaternion orientation\n================================================================================\nMSG: geometry_msgs/Point\nfloat64 x\nfloat64 y\nfloat64 z\n================================================================================\nMSG: geometry_msgs/Quaternion\nfloat64 x\nfloat64 y\nfloat64 z\nfloat64 w\n";
        let schema = parse_concatenated(
            "geometry_msgs/Pose",
            SchemaKind::Message,
            text,
            Dialect::Foxglove,
        )
        .expect("parse should succeed after re-qualifying");
        let root = match &schema.parsed {
            ParsedSchema::Message(m) => m,
            _ => panic!("expected message"),
        };
        assert_eq!(root.fields.len(), 2);
        match &root.fields[0].field_type {
            rw_canonical::FieldType::Complex { type_name } => {
                assert_eq!(type_name, "geometry_msgs/Point");
            }
            other => panic!("expected complex, got {other:?}"),
        }
    }

    #[test]
    fn array_of_bare_type_is_also_qualified() {
        let text = "ConnectedClient[] clients\n================================================================================\nMSG: rosbridge_msgs/ConnectedClient\nstring ip_address\n";
        let schema = parse_concatenated(
            "rosbridge_msgs/ConnectedClients",
            SchemaKind::Message,
            text,
            Dialect::Foxglove,
        )
        .expect("parse should succeed");
        let root = match &schema.parsed {
            ParsedSchema::Message(m) => m,
            _ => panic!("expected message"),
        };
        match &root.fields[0].field_type {
            rw_canonical::FieldType::Array { element, .. } => match element.as_ref() {
                rw_canonical::FieldType::Complex { type_name } => {
                    assert_eq!(type_name, "rosbridge_msgs/ConnectedClient");
                }
                other => panic!("expected complex element, got {other:?}"),
            },
            other => panic!("expected array, got {other:?}"),
        }
    }

    #[test]
    fn no_dependencies_when_root_only() {
        let text = "uint32 seq\nstring frame_id\n";
        let split = split_concatenated(text);
        assert!(split.dependencies.is_empty());
    }

    #[test]
    fn bare_header_token_resolves_to_std_msgs_header() {
        let text = "Header header\nfloat32 value\n================================================================================\nMSG: std_msgs/Header\nuint32 seq\nstring frame_id\ntime stamp\n";
        let schema = parse_concatenated(
            "example_msgs/SampleState",
            SchemaKind::Message,
            text,
            Dialect::Foxglove,
        )
        .expect("parse should succeed using std_msgs/Header for bare Header");
        let root = match &schema.parsed {
            ParsedSchema::Message(m) => m,
            _ => panic!("expected message"),
        };
        match &root.fields[0].field_type {
            rw_canonical::FieldType::Complex { type_name } => {
                assert_eq!(type_name, "std_msgs/Header");
            }
            other => panic!("expected complex Header, got {other:?}"),
        }
    }

    #[test]
    fn action_definition_parts_assemble_goal_result_feedback() {
        let separator = "=".repeat(80);
        let send_goal_request = format!(
            "unique_identifier_msgs/UUID goal_id\nexample_interfaces/action/Fibonacci_Goal goal\n{separator}\nMSG: unique_identifier_msgs/UUID\nuint8[16] uuid\n{separator}\nMSG: example_interfaces/action/Fibonacci_Goal\nint32 order"
        );
        let get_result_response = format!(
            "int8 status\nexample_interfaces/action/Fibonacci_Result result\n{separator}\nMSG: example_interfaces/action/Fibonacci_Result\nint32[] sequence"
        );
        let feedback_topic = format!(
            "unique_identifier_msgs/UUID goal_id\nexample_interfaces/action/Fibonacci_Feedback feedback\n{separator}\nMSG: unique_identifier_msgs/UUID\nuint8[16] uuid\n{separator}\nMSG: example_interfaces/action/Fibonacci_Feedback\nint32[] partial_sequence"
        );

        let mut parts = ActionDefinitionParts::default();
        parts.set_goal_from_send_goal_request(&send_goal_request);
        parts.set_result_from_get_result_response(&get_result_response);
        parts.set_feedback_from_feedback_topic(&feedback_topic);

        assert_eq!(
            parts.to_action_definition().as_deref(),
            Some("int32 order\n---\nint32[] sequence\n---\nint32[] partial_sequence")
        );
    }

    #[test]
    fn action_definition_needs_at_least_the_goal_and_parses_with_empty_result_feedback() {
        let mut parts = ActionDefinitionParts::default();
        assert_eq!(parts.to_action_definition(), None);

        let separator = "=".repeat(80);
        let flattened_send_goal = format!(
            "unique_identifier_msgs/UUID goal_id\nint32 order\n{separator}\nMSG: unique_identifier_msgs/UUID\nuint8[16] uuid"
        );
        parts.set_goal_from_send_goal_request(&flattened_send_goal);

        let text = parts.to_action_definition().expect("goal present");
        assert_eq!(text, "int32 order\n---\n\n---\n");
        let parsed = parse_concatenated(
            "example_interfaces/Fibonacci",
            SchemaKind::Action,
            &text,
            Dialect::Ros2,
        );
        assert!(
            parsed.is_ok(),
            "empty result/feedback action parse failed: {parsed:?}"
        );
    }
}
