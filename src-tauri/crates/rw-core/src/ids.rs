pub type RequestId = i64;
pub type CollectionId = i64;
pub type ConnectionId = i64;

pub type SessionId = String;
pub type SubscriptionHandle = String;
pub type ActionGoalHandle = String;

pub fn new_handle() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_are_unique_uuid_v4() {
        let first = new_handle();
        let second = new_handle();

        assert_ne!(first, second);
        assert_eq!(first.len(), 36);
        let parsed = uuid::Uuid::parse_str(&first).expect("handle parses as UUID");
        assert_eq!(parsed.get_version_num(), 4);
    }
}
