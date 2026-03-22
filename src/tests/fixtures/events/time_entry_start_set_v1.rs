use crate::modules::time_entries::core::events::v1::time_entry_start_set::TimeEntryStartSetV1;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct TimeEntryStartSetV1Dto {
    time_entry_id: String,
    started_at: i64,
    updated_at: i64,
    updated_by: String,
}

pub fn make_time_entry_start_set_v1_event() -> TimeEntryStartSetV1 {
    let json_str =
        fs::read_to_string("./src/tests/fixtures/events/json/start_set_event_v1.json").unwrap();
    let dto: TimeEntryStartSetV1Dto = serde_json::from_str(&json_str).unwrap();
    TimeEntryStartSetV1 {
        time_entry_id: dto.time_entry_id,
        started_at: dto.started_at,
        updated_at: dto.updated_at,
        updated_by: dto.updated_by,
    }
}

#[cfg(test)]
mod time_entry_start_set_v1_fixture_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn fixture_loads_from_json() {
        let event = make_time_entry_start_set_v1_event();
        assert_eq!(event.time_entry_id, "te-fixed-0001");
        assert_eq!(event.started_at, 1700000000000);
        assert_eq!(event.updated_at, 1700000000000);
        assert_eq!(event.updated_by, "user-fixed-0001");
    }
}
