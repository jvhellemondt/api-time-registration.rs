// TimeEntryState is the canonical domain state after folding events.
//
// Suggested structure (to implement later)
// - None
// - Registered { time_entry_id, user_id, start_time, end_time, tags, description, created_at,
//                created_by, updated_at, updated_by, deleted_at, last_event_id }
//
// Boundaries
// - This file must not perform input or output.
// - Keep it framework-free.
//
// Testing guidance
// - Use the evolve function to produce states from events and assert expected fields.

