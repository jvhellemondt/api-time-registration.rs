// Command data type for registering a time entry.
//
// Purpose
// - Express user intent to create a time entry with start and end time, tags, and description.
//
// Responsibilities
// - Carry input data for the decider to validate and convert into an event.
// - Be independent of transport layer details (not tied to HTTP or GraphQL).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterTimeEntry {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
    pub created_by: String,
}

#[cfg(test)]
mod time_entry_registered_event_tests {
    use super::*;
    use rstest::{rstest, fixture};
    use crate::core::time_entry::event::v1::time_entry_registered::TimeEntryRegisteredV1;
    use crate::test_fixtures::make_time_entry_registered_v1_event;

    #[fixture]
    fn registered_event() -> TimeEntryRegisteredV1 {
        make_time_entry_registered_v1_event()
    }

    #[rstest]
    fn it_should_create_the_command(registered_event: TimeEntryRegisteredV1) {
        let command = RegisterTimeEntry {
            time_entry_id: registered_event.time_entry_id.clone(),
            user_id: registered_event.user_id.clone(),
            start_time: registered_event.start_time,
            end_time: registered_event.end_time,
            tags: registered_event.tags.clone(),
            description: registered_event.description,
            created_by: registered_event.created_by,
        };
        assert_eq!(command.time_entry_id, registered_event.time_entry_id);
        assert_eq!(command.user_id, registered_event.user_id);
        assert_eq!(command.tags, registered_event.tags);
    }

}
