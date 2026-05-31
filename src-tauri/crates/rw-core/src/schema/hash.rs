use std::collections::{BTreeMap, HashSet};

use sha2::{Digest, Sha256};

use crate::schema::parser;
use crate::schema::SchemaKind;
use crate::{CoreError, CoreResult};

pub fn canonicalize(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    for line in source.lines() {
        let stripped = strip_comment(line);
        let trimmed = stripped.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        output.push_str(trimmed);
        output.push('\n');
    }
    output
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(position) => &line[..position],
        None => line,
    }
}

pub fn canonical_hash<'a, F>(
    kind: SchemaKind,
    source: &str,
    resolve_dependency: F,
) -> CoreResult<String>
where
    F: FnMut(&str) -> Option<&'a str>,
{
    canonical_hash_with_package(kind, source, None, resolve_dependency)
}

pub fn canonical_hash_with_package<'a, F>(
    kind: SchemaKind,
    source: &str,
    default_package: Option<&str>,
    mut resolve_dependency: F,
) -> CoreResult<String>
where
    F: FnMut(&str) -> Option<&'a str>,
{
    let parsed = parser::parse_with_package(kind, source, default_package)?;
    let direct_deps = parser::collect_dependencies(&parsed);

    let mut visited: HashSet<String> = HashSet::new();
    let mut ordered: Vec<(String, String)> = Vec::new();
    let mut stack: Vec<String> = direct_deps.iter().rev().cloned().collect();

    while let Some(name) = stack.pop() {
        if !visited.insert(name.clone()) {
            continue;
        }
        let body = match resolve_dependency(&name) {
            Some(text) => text,
            None => {
                return Err(CoreError::Schema(format!(
                    "schema dependency '{name}' is not registered; register it before computing the hash"
                )));
            }
        };
        let dep_package = name.split('/').next().filter(|segment| !segment.is_empty());
        let dep_parsed = parser::parse_with_package(SchemaKind::Message, body, dep_package)?;
        let nested_deps = parser::collect_dependencies(&dep_parsed);
        for nested in nested_deps.into_iter().rev() {
            if !visited.contains(&nested) {
                stack.push(nested);
            }
        }
        ordered.push((name, canonicalize(body)));
    }

    ordered.sort_by(|left, right| left.0.cmp(&right.0));

    let primary = canonicalize(source);
    let mut hasher = Sha256::new();
    hasher.update(primary.as_bytes());
    for (_, dep_text) in &ordered {
        hasher.update(b"\n---\n");
        hasher.update(dep_text.as_bytes());
    }
    Ok(hex_encode(&hasher.finalize()))
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(*byte >> 4) as usize] as char);
        out.push(DIGITS[(*byte & 0xF) as usize] as char);
    }
    out
}

pub fn hash_isolated(kind: SchemaKind, source: &str) -> CoreResult<String> {
    let dep_map: BTreeMap<String, &str> = BTreeMap::new();
    canonical_hash(kind, source, |name| dep_map.get(name).copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_strips_comments_and_blanks() {
        let input = "# header line\nfloat32 x   \n\n\n# trailing\nfloat32 y\n";
        assert_eq!(canonicalize(input), "float32 x\nfloat32 y\n");
    }

    #[test]
    fn whitespace_only_changes_dont_change_hash() {
        let a = "float32 x\nfloat32 y\n";
        let b = "float32 x   \n\n\nfloat32 y\n";
        assert_eq!(
            hash_isolated(SchemaKind::Message, a).unwrap(),
            hash_isolated(SchemaKind::Message, b).unwrap()
        );
    }

    #[test]
    fn comment_only_edits_dont_change_hash() {
        let a = "float32 x\nfloat32 y\n";
        let b = "# explaining x\nfloat32 x\nfloat32 y # explanation\n";
        assert_eq!(
            hash_isolated(SchemaKind::Message, a).unwrap(),
            hash_isolated(SchemaKind::Message, b).unwrap()
        );
    }

    #[test]
    fn field_rename_changes_hash() {
        let a = "float32 x\nfloat32 y\n";
        let b = "float32 px\nfloat32 py\n";
        assert_ne!(
            hash_isolated(SchemaKind::Message, a).unwrap(),
            hash_isolated(SchemaKind::Message, b).unwrap()
        );
    }

    #[test]
    fn same_text_same_hash() {
        let text = "float32 x\nfloat32 y\n";
        let first = hash_isolated(SchemaKind::Message, text).unwrap();
        let second = hash_isolated(SchemaKind::Message, text).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn dependencies_change_hash() {
        let header = "uint32 seq\n";
        let with_dep = "std_msgs/Header header\nfloat32 x\n";
        let resolver_one = |name: &str| {
            if name == "std_msgs/Header" {
                Some(header)
            } else {
                None
            }
        };
        let hash_one = canonical_hash(SchemaKind::Message, with_dep, resolver_one).unwrap();

        let updated_header = "uint32 seq2\n";
        let resolver_two = |name: &str| {
            if name == "std_msgs/Header" {
                Some(updated_header)
            } else {
                None
            }
        };
        let hash_two = canonical_hash(SchemaKind::Message, with_dep, resolver_two).unwrap();

        assert_ne!(hash_one, hash_two);
    }

    #[test]
    fn missing_dependency_errors() {
        let with_dep = "std_msgs/Header header\nfloat32 x\n";
        let result = canonical_hash(SchemaKind::Message, with_dep, |_| None::<&str>);
        assert!(matches!(result, Err(CoreError::Schema(_))));
    }
}
