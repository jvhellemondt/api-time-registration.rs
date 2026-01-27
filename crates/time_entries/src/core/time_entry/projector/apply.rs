// Translate a domain event into read model mutations.
//
// Purpose
// - Build an upsert for registration and minimal patches for future change events.
//
// Responsibilities
// - Calculate last_event_id as a stable identifier like "stream_id:version".
// - Return a list of mutations to be persisted by the application runner.

