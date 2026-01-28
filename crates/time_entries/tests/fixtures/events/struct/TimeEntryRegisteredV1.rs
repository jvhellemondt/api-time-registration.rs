// Shared test fixture for TimeEntryRegisteredV1.
// This file lives under tests/fixtures/... but is compiled into the crate
// only during tests via include! in src/lib.rs (cfg(test)).

use crate::core::time_entry::event::v1::time_entry_registered::TimeEntryRegisteredV1;

// Builder function returning a canonical event instance for tests.
pub fn make_time_entry_registered_v1_event() -> TimeEntryRegisteredV1 {
    TimeEntryRegisteredV1 {
        time_entry_id: "te-fixed-0001".to_string(),
        user_id: "user-fixed-0001".to_string(),
        start_time: 1_700_000_000_000,
        end_time: 1_700_000_360_000,
        tags: vec!["Work".to_string()],
        description: "This is a test".to_string(),
        created_at: 1_700_000_000_000,
        created_by: "user-fixed-0001".to_string(),
    }
}
