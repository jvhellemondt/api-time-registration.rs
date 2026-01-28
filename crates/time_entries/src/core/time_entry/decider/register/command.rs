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
    pub created_at: i64,
    pub created_by: String,
}

#[cfg(test)]
mod time_entry_registered_event_tests {
    use super::*;
    use crate::test_fixtures::make_register_time_entry_command;
    use rstest::{fixture, rstest};

    #[fixture]
    fn register_command() -> RegisterTimeEntry {
        make_register_time_entry_command()
    }

    #[rstest]
    fn it_should_create_the_command(register_command: RegisterTimeEntry) {
        let command = RegisterTimeEntry {
            time_entry_id: register_command.time_entry_id.clone(),
            user_id: register_command.user_id.clone(),
            start_time: register_command.start_time,
            end_time: register_command.end_time,
            tags: register_command.tags.clone(),
            description: register_command.description,
            created_at: register_command.created_at,
            created_by: register_command.created_by,
        };
        assert_eq!(command.time_entry_id, register_command.time_entry_id);
        assert_eq!(command.user_id, register_command.user_id);
        assert_eq!(command.tags, register_command.tags);
    }
}
