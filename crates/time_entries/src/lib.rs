// Crate entry point. Re-export modules so tests and binaries can import them easily.
//
// Responsibilities
// - Only declare and expose modules. No business logic here.
//
// How it is used
// - Tests import modules from this crate root to reach the code under test.

pub mod core {
    pub mod time_entry;
}
// pub mod application;
// pub mod adapters;
// pub mod shell;

