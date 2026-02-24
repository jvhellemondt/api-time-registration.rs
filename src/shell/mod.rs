// Composition root for the time_entries bounded context.
//
// Responsibilities (when implemented):
// - Read config from environment.
// - Instantiate concrete infrastructure implementations.
// - Wire implementations into use case handlers.
// - Spawn background workers (projector runner, intent relay runner, event relay runner).

pub mod graphql;
pub mod http;
pub mod state;
pub mod workers;
