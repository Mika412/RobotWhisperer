use rw_canonical::{FieldType, ParsedSchema, SchemaKind};
use rw_schema_ros2 as ros2;

pub use ros2::{ParseError, ParseResult};

pub fn parse(kind: SchemaKind, source: &str) -> ParseResult<ParsedSchema> {
    let rewritten = rewrite_bare_header_source(source);
    ros2::parse(kind, &rewritten)
}

fn rewrite_bare_header_source(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    for line in source.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("Header") {
            let after = rest.chars().next();
            if after.map(|c| c.is_whitespace()).unwrap_or(false) {
                let indent_len = line.len() - trimmed.len();
                out.push_str(&line[..indent_len]);
                out.push_str("std_msgs/Header");
                out.push_str(rest);
                out.push('\n');
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[allow(dead_code)]
fn rewrite_field_type(ty: &mut FieldType) {
    match ty {
        FieldType::Complex { type_name } if type_name == "Header" => {
            *type_name = "std_msgs/Header".into();
        }
        FieldType::Array { element, .. } => rewrite_field_type(element),
        _ => {}
    }
}

pub use rw_schema_ros2::collect_dependencies;

#[cfg(test)]
mod tests {
    use super::*;
    use rw_canonical::ParsedSchema;

    #[test]
    fn bare_header_resolves_to_std_msgs() {
        let parsed = parse(SchemaKind::Message, "Header header\nfloat32 value\n").unwrap();
        match parsed {
            ParsedSchema::Message(message) => assert!(matches!(
                message.fields[0].field_type,
                FieldType::Complex { ref type_name } if type_name == "std_msgs/Header"
            )),
            _ => panic!(),
        }
    }
}
