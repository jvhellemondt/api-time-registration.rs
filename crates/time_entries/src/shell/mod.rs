// Composition root for the time_entries bounded context.
//
// Responsibilities (when implemented):
// - Read config from environment.
// - Instantiate concrete infrastructure implementations.
// - Wire implementations into use case handlers.
// - Spawn background workers (projector runner, intent relay runner, event relay runner).
// - Expose the HTTP router to time_entries_api.

pub mod workers;
