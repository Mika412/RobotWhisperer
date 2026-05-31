use crate::ids::CollectionId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    pub id: CollectionId,
    pub parent_id: Option<CollectionId>,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn collection_round_trips_through_json() {
        let collection = Collection {
            id: 7,
            parent_id: Some(1),
            name: "Demo".into(),
            created_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        };

        let json = serde_json::to_string(&collection).unwrap();
        let decoded: Collection = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, collection);
    }
}
