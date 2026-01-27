// Pure decision function for registration.
//
// Purpose
// - Validate the command against the current state and produce domain events on success.
//
// Responsibilities
// - Enforce rules: end time must be after start time, tag count must be within limits.
// - If state is None, emit TimeEntryRegisteredV1. If state is already registered, return an error.
// - Never perform input or output.

