use crate::modules::time_entries::core::events::v1::time_entry_end_set::TimeEntryEndSetV1;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct TimeEntryEndSetV1Dto {
    time_entry_id: String,
    ended_at: i64,
    updated_at: i64,
    updated_by: String,
}

pub fn make_time_entry_end_set_v1_event() -> TimeEntryEndSetV1 {
    let json_str =
        fs::read_to_string("./src/tests/fixtures/events/json/end_set_event_v1.json").unwrap();
    let dto: TimeEntryEndSetV1Dto = serde_json::from_str(&json_str).unwrap();
    TimeEntryEndSetV1 {
        time_entry_id: dto.time_entry_id,
        ended_at: dto.ended_at,
        updated_at: dto.updated_at,
        updated_by: dto.updated_by,
    }
}

#[cfg(test)]
mod time_entry_end_set_v1_fixture_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn fixture_loads_from_json() {
        let event = make_time_entry_end_set_v1_event();
        assert_eq!(event.time_entry_id, "te-fixed-0001");
        assert_eq!(event.ended_at, 1700000360000);
        assert_eq!(event.updated_at, 1700000000000);
        assert_eq!(event.updated_by, "user-fixed-0001");
    }
}
