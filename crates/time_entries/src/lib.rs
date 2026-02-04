// Crate entry point. Re-export modules so tests and binaries can import them easily.
//
// Responsibilities
// - Only declare and expose modules. No business logic here.
//
// How it is used
// - Tests import modules from this crate root to reach the code under test.

pub mod core {
    pub mod ports;
    pub mod time_entry;
}

pub mod application {
    pub mod errors;
    pub mod command_handlers {
        pub mod register_handler;
    }
    pub mod query_handlers {
        pub mod time_entries_queries;
    }
    pub mod projector {
        pub mod repository;
        pub mod runner;
    }
}

pub mod adapters {
    pub mod in_memory {
        pub mod in_memory_domain_outbox;
        pub mod in_memory_event_store;
        pub mod in_memory_projections;
    }
    pub mod mappers {
        pub mod time_entry_row_to_time_entry_view;
    }
}

// pub mod shell;

#[cfg(test)]
pub mod tests {
    pub mod fixtures;

    pub mod e2e {
        pub mod list_time_entries_by_user_tests;
    }
}
