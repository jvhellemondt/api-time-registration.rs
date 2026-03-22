use crate::modules::time_entries::core::events::v1::time_entry_registered::TimeEntryRegisteredV1;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct TimeEntryRegisteredV1Dto {
    time_entry_id: String,
    occurred_at: i64,
}

pub fn make_time_entry_registered_v1_event() -> TimeEntryRegisteredV1 {
    let json_str =
        fs::read_to_string("./src/tests/fixtures/events/json/registered_event_v1.json").unwrap();
    let dto: TimeEntryRegisteredV1Dto = serde_json::from_str(&json_str).unwrap();
    TimeEntryRegisteredV1 {
        time_entry_id: dto.time_entry_id,
        occurred_at: dto.occurred_at,
    }
}

#[cfg(test)]
mod time_entry_registered_v1_fixture_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn fixture_loads_from_json() {
        let event = make_time_entry_registered_v1_event();
        assert_eq!(event.time_entry_id, "te-fixed-0001");
        assert_eq!(event.occurred_at, 1700000000000);
    }
}
