// This module groups time entry domain components in 2018 style.
//
// Structure
// - state.rs: domain state
// - event.rs + event/: root event enum and versioned payloads
// - evolve.rs: pure state transitions
// - decider/: pure decision logic per command intent
// - projector/: mapping from events to read model mutations
//
// Import pattern
// - Use `pub mod state;` etc. in this file once you add code. For now, see files in core/time_entry/.

pub mod event;
pub mod evolve;
pub mod state;
pub mod decider {
    pub mod register {
        pub mod command;
        pub mod decide;
    }
}
pub mod projector {
    pub mod model;
    pub mod apply;
}
