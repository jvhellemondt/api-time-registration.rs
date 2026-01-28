// Shared test fixture for TimeEntryRegisteredV1.
// This file lives under tests/fixtures/... but is compiled into the crate
// only during tests via include! in src/lib.rs (cfg(test)).

use crate::core::time_entry::event::v1::time_entry_registered::TimeEntryRegisteredV1;

/// Builder function returning a canonical event instance for tests.
pub fn make_time_entry_registered_v1_event() -> TimeEntryRegisteredV1 {
    let command = make_register_time_entry_command();
    TimeEntryRegisteredV1 {
        time_entry_id: command.time_entry_id,
        user_id: command.user_id,
        start_time: command.start_time,
        end_time: command.end_time,
        tags: command.tags,
        description: command.description,
        created_at: command.created_at,
        created_by: command.created_by,
    }
}
