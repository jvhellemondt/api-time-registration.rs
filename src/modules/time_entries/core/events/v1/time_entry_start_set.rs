#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TimeEntryStartSetV1 {
    pub time_entry_id: String,
    pub started_at: i64,
    pub updated_at: i64,
    pub updated_by: String,
}

#[cfg(test)]
mod time_entry_start_set_event_tests {
    use super::*;
    use crate::tests::fixtures::events::time_entry_start_set_v1::make_time_entry_start_set_v1_event;
    use rstest::{fixture, rstest};
    use std::fs;

    #[fixture]
    fn start_set_event() -> TimeEntryStartSetV1 {
        make_time_entry_start_set_v1_event()
    }

    #[rstest]
    fn it_should_create_the_start_set_event(start_set_event: TimeEntryStartSetV1) {
        assert_eq!(start_set_event.time_entry_id, "te-fixed-0001");
        assert_eq!(start_set_event.started_at, 1_700_000_000_000i64);
        assert_eq!(start_set_event.updated_at, 1_700_000_000_000i64);
        assert_eq!(start_set_event.updated_by, "user-fixed-0001");
    }

    #[fixture]
    fn golden_start_set_event_json() -> serde_json::Value {
        let s = fs::read_to_string("./src/tests/fixtures/events/json/time_entry_start_set_v1.json")
            .unwrap();
        serde_json::from_str(&s).unwrap()
    }

    #[rstest]
    fn it_serializes_start_set_event_stable(
        start_set_event: TimeEntryStartSetV1,
        golden_start_set_event_json: serde_json::Value,
    ) {
        let json = serde_json::to_value(&start_set_event).unwrap();
        assert_eq!(json, golden_start_set_event_json);
    }
}
