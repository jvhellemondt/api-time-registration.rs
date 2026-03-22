#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TimeEntryInitiatedV1 {
    pub time_entry_id: String,
    pub user_id: String,
    pub created_at: i64,
    pub created_by: String,
}

#[cfg(test)]
mod time_entry_initiated_event_tests {
    use super::*;
    use crate::tests::fixtures::events::time_entry_initiated_v1::make_time_entry_initiated_v1_event;
    use rstest::{fixture, rstest};
    use std::fs;

    #[fixture]
    fn initiated_event() -> TimeEntryInitiatedV1 {
        make_time_entry_initiated_v1_event()
    }

    #[rstest]
    fn it_should_create_the_initiated_event(initiated_event: TimeEntryInitiatedV1) {
        assert_eq!(initiated_event.time_entry_id, "te-fixed-0001");
        assert_eq!(initiated_event.user_id, "user-fixed-0001");
        assert_eq!(initiated_event.created_at, 1_700_000_000_000i64);
        assert_eq!(initiated_event.created_by, "user-fixed-0001");
    }

    #[fixture]
    fn golden_initiated_event_json() -> serde_json::Value {
        let s = fs::read_to_string("./src/tests/fixtures/events/json/time_entry_initiated_v1.json")
            .unwrap();
        serde_json::from_str(&s).unwrap()
    }

    #[rstest]
    fn it_serializes_initiated_event_stable(
        initiated_event: TimeEntryInitiatedV1,
        golden_initiated_event_json: serde_json::Value,
    ) {
        let json = serde_json::to_value(&initiated_event).unwrap();
        assert_eq!(json, golden_initiated_event_json);
    }
}
