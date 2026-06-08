use super::collection::Collection;
use super::connection::Connection;
use super::request::Request;
use crate::schema::SchemaDefinition;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const WORKSPACE_FORMAT: &str = "robot-whisperer/workspace";
pub const WORKSPACE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Workspace {
    #[serde(default)]
    pub connections: Vec<Connection>,
    #[serde(default)]
    pub collections: Vec<Collection>,
    #[serde(default)]
    pub requests: Vec<Request>,
    #[serde(default)]
    pub schemas: Vec<SchemaDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFile {
    #[serde(rename = "_comment", default, skip_deserializing)]
    pub comment: String,
    pub format: String,
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    #[serde(flatten)]
    pub workspace: Workspace,
}

impl PartialEq for WorkspaceFile {
    fn eq(&self, other: &Self) -> bool {
        self.format == other.format
            && self.version == other.version
            && self.exported_at == other.exported_at
            && self.workspace == other.workspace
    }
}

impl WorkspaceFile {
    pub fn new(workspace: Workspace, exported_at: DateTime<Utc>) -> Self {
        Self {
            comment: "Robot Whisperer workspace export. Edit with care. \
                      Format must remain \"robot-whisperer/workspace\". \
                      Version is the schema version this file targets."
                .to_string(),
            format: WORKSPACE_FORMAT.to_string(),
            version: WORKSPACE_VERSION,
            exported_at,
            workspace,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn workspace_file_round_trips_through_json() {
        let exported = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let file = WorkspaceFile::new(Workspace::default(), exported);

        let json = serde_json::to_string(&file).unwrap();
        let decoded: WorkspaceFile = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, file);
        assert_eq!(decoded.format, WORKSPACE_FORMAT);
        assert_eq!(decoded.version, WORKSPACE_VERSION);
    }

    #[test]
    fn workspace_file_json_uses_flat_envelope() {
        let exported = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let file = WorkspaceFile::new(Workspace::default(), exported);
        let json = serde_json::to_value(&file).unwrap();

        let object = json.as_object().expect("workspace file is object");
        assert!(object.contains_key("format"));
        assert!(object.contains_key("version"));
        assert!(object.contains_key("exported_at"));
        assert!(object.contains_key("connections"));
        assert!(object.contains_key("collections"));
        assert!(object.contains_key("requests"));
        assert!(object.contains_key("schemas"));
    }
}
