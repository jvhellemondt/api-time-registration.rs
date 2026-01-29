// Crate entry point. Re-export modules so tests and binaries can import them easily.
//
// Responsibilities
// - Only declare and expose modules. No business logic here.
//
// How it is used
// - Tests import modules from this crate root to reach the code under test.

pub mod core {
    pub mod time_entry;
    pub mod ports;
}

pub mod application {
    pub mod errors;
    pub mod command_handlers {
        pub mod register_handler;
    }
}

pub mod adapters {
    pub mod in_memory {
        pub mod in_memory_domain_outbox;
        pub mod in_memory_event_store;
    }
}

// pub mod shell;

#[cfg(test)]
pub mod test_support {
    pub mod fixtures;
}
