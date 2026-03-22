#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TimeEntryDeletedV1 {
    pub time_entry_id: String,
    pub deleted_at: i64,
    pub deleted_by: String,
}

#[cfg(test)]
mod time_entry_deleted_event_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn it_should_create_the_deleted_event() {
        let event = TimeEntryDeletedV1 {
            time_entry_id: "te-fixed-0001".to_string(),
            deleted_at: 1_700_000_500_000i64,
            deleted_by: "user-fixed-0001".to_string(),
        };
        assert_eq!(event.time_entry_id, "te-fixed-0001");
        assert_eq!(event.deleted_at, 1_700_000_500_000i64);
        assert_eq!(event.deleted_by, "user-fixed-0001");
    }

    #[rstest]
    fn it_serializes_and_deserializes_roundtrip() {
        let event = TimeEntryDeletedV1 {
            time_entry_id: "te-fixed-0001".to_string(),
            deleted_at: 1_700_000_500_000i64,
            deleted_by: "user-fixed-0001".to_string(),
        };
        let json = serde_json::to_value(&event).unwrap();
        let restored: TimeEntryDeletedV1 = serde_json::from_value(json).unwrap();
        assert_eq!(restored, event);
    }
}
