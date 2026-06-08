use sha2::{Digest, Sha256};

use crate::id::SchemaId;

pub fn canonical_schema_id(definition: &str) -> SchemaId {
    let canonical = canonicalize(definition);
    SchemaId::new(hex_digest(canonical.as_bytes()))
}

pub fn canonical_schema_id_with_deps<I>(root: &str, dependencies: I) -> SchemaId
where
    I: IntoIterator<Item = (String, String)>,
{
    let mut hasher = Sha256::new();
    hasher.update(canonicalize(root).as_bytes());
    for (name, text) in dependencies {
        hasher.update(b"\n====\n");
        hasher.update(name.as_bytes());
        hasher.update(b"\n");
        hasher.update(canonicalize(&text).as_bytes());
    }
    SchemaId::new(hex_encode(&hasher.finalize()))
}

fn canonicalize(definition: &str) -> String {
    let mut output = String::with_capacity(definition.len());
    for raw_line in definition.lines() {
        let stripped = strip_comment(raw_line).trim();
        if stripped.is_empty() {
            continue;
        }
        let normalised = collapse_whitespace_and_normalise(stripped);
        output.push_str(&normalised);
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

fn collapse_whitespace_and_normalise(line: &str) -> String {
    let mut tokens = line.split_whitespace();
    let mut out = String::new();
    if let Some(first) = tokens.next() {
        out.push_str(&normalise_type_token(first));
        for token in tokens {
            out.push(' ');
            out.push_str(token);
        }
    }
    out
}

fn normalise_type_token(token: &str) -> String {
    let (base, suffix) = split_array_suffix(token);
    let normalised_base = normalise_complex(base);
    match suffix {
        Some(s) => format!("{normalised_base}{s}"),
        None => normalised_base,
    }
}

fn split_array_suffix(token: &str) -> (&str, Option<&str>) {
    if let Some(open) = token.rfind('[') {
        if token.ends_with(']') {
            return (&token[..open], Some(&token[open..]));
        }
    }
    (token, None)
}

fn normalise_complex(token: &str) -> String {
    let segments: Vec<&str> = token.split('/').collect();
    if segments.len() == 3 && matches!(segments[1], "msg" | "srv" | "action") {
        format!("{}/{}", segments[0], segments[2])
    } else {
        token.to_string()
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(TABLE[(byte >> 4) as usize] as char);
        out.push(TABLE[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_and_whitespace_are_ignored() {
        let a = "# leading comment\nuint32  seq   # inline\nstring   frame_id\n";
        let b = "uint32 seq\nstring frame_id\n";
        assert_eq!(canonical_schema_id(a), canonical_schema_id(b));
    }

    #[test]
    fn ros2_msg_segment_canonicalises_to_short_form() {
        let with_msg = "geometry_msgs/msg/Point position\n";
        let without = "geometry_msgs/Point position\n";
        assert_eq!(canonical_schema_id(with_msg), canonical_schema_id(without));
    }

    #[test]
    fn array_suffix_does_not_block_canonicalisation() {
        let a = "float32[16] data\n";
        let b = "float32[16]   data    # trailing\n";
        assert_eq!(canonical_schema_id(a), canonical_schema_id(b));
    }

    #[test]
    fn dependencies_change_the_id() {
        let root = "std_msgs/Header header\nfloat32 value\n";
        let dep_a = ("std_msgs/Header".to_string(), "uint32 seq\n".to_string());
        let dep_b = (
            "std_msgs/Header".to_string(),
            "uint32 seq\nstring frame_id\n".to_string(),
        );
        let id_a = canonical_schema_id_with_deps(root, std::iter::once(dep_a));
        let id_b = canonical_schema_id_with_deps(root, std::iter::once(dep_b));
        assert_ne!(id_a, id_b);
    }

    #[test]
    fn id_is_64_hex_chars() {
        let id = canonical_schema_id("uint32 seq\n");
        assert_eq!(id.as_str().len(), 64);
        assert!(id.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }
}
