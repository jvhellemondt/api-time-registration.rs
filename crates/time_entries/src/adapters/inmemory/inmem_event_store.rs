// In memory implementation of the EventStore port.
//
// Purpose
// - Support command handler tests and local development without a database.
//
// Responsibilities
// - Store events per stream in memory.
// - Enforce optimistic concurrency by checking the expected version.

