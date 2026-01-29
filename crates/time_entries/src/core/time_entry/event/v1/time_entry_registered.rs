// Event payload: TimeEntryRegisteredV1.
//
// Purpose
// - Record the business fact that a time entry was registered with the minimal fields.
//
// Responsibilities
// - Carry only identifiers and snapshot values needed by the domain today.
//
// Inputs and outputs
// - Inputs: the decider validates values from the command.
// - Outputs: fed into evolving to produce the first registered state and into projectors.
//
// Versioning and evolution
// - Prefer adding fields. For breaking changes, create TimeEntryRegisteredV2 in a new file and add a new variant.
//
// Timestamps
// - All i64 values must use the same epoch unit (milliseconds).

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TimeEntryRegisteredV1 {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub created_by: String,
}

#[cfg(test)]
mod time_entry_registered_event_tests {
    use super::*;
    use rstest::{fixture, rstest};
    use std::fs;
    use crate::test_support::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;

    #[fixture]
    fn registered_event() -> TimeEntryRegisteredV1 {
        make_time_entry_registered_v1_event()
    }

    #[rstest]
    fn it_should_create_the_registered_event(registered_event: TimeEntryRegisteredV1) {
        assert_eq!(registered_event.time_entry_id, "te-fixed-0001");
        assert_eq!(registered_event.user_id, "user-fixed-0001");
        assert_eq!(registered_event.tags, vec!["Work".to_string()]);
    }

    #[fixture]
    fn golden_registered_event_json() -> serde_json::Value {
        let s = fs::read_to_string("./src/test_support/fixtures/events/json/registered_event_v1.json").unwrap();
        serde_json::from_str(&s).unwrap()
    }

    #[rstest]
    fn it_serializes_registered_event_stable(
        registered_event: TimeEntryRegisteredV1,
        golden_registered_event_json: serde_json::Value,
    ) {
        let json = serde_json::to_value(&registered_event).unwrap();
        assert_eq!(json, golden_registered_event_json);
    }
}
