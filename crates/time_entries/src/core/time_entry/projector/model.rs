// Read model row for a single time entry and last_event_id for idempotency.
//
// Purpose
// - Represent how a time entry is stored for fast reads in the projection store.
//
// Responsibilities
// - Map from event fields where possible.
// - Include last_event_id so idempotent upserts and patches are possible.

