use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SchemaId(String);

impl SchemaId {
    pub fn new(hex: impl Into<String>) -> Self {
        SchemaId(hex.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for SchemaId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for SchemaId {
    fn from(value: &str) -> Self {
        SchemaId(value.to_string())
    }
}

impl From<String> for SchemaId {
    fn from(value: String) -> Self {
        SchemaId(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_id_serializes_as_bare_string() {
        let id = SchemaId::new("deadbeef");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"deadbeef\"");
        let decoded: SchemaId = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, id);
    }
}
