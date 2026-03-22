use crate::modules::time_entries::core::events::v1::time_entry_initiated::TimeEntryInitiatedV1;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct TimeEntryInitiatedV1Dto {
    time_entry_id: String,
    user_id: String,
    created_at: i64,
    created_by: String,
}

pub fn make_time_entry_initiated_v1_event() -> TimeEntryInitiatedV1 {
    let json_str =
        fs::read_to_string("./src/tests/fixtures/events/json/initiated_event_v1.json").unwrap();
    let dto: TimeEntryInitiatedV1Dto = serde_json::from_str(&json_str).unwrap();
    TimeEntryInitiatedV1 {
        time_entry_id: dto.time_entry_id,
        user_id: dto.user_id,
        created_at: dto.created_at,
        created_by: dto.created_by,
    }
}

#[cfg(test)]
mod time_entry_initiated_v1_fixture_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn fixture_loads_from_json() {
        let event = make_time_entry_initiated_v1_event();
        assert_eq!(event.time_entry_id, "te-fixed-0001");
        assert_eq!(event.user_id, "user-fixed-0001");
        assert_eq!(event.created_at, 1700000000000);
        assert_eq!(event.created_by, "user-fixed-0001");
    }
}
