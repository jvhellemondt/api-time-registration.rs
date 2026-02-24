pub mod shared {
    pub mod core {
        pub mod primitives;
    }
    pub mod infrastructure {
        pub mod event_store;
        pub mod intent_outbox;
    }
}

pub mod modules {
    pub mod time_entries {
        pub mod core {
            pub mod events;
            pub mod evolve;
            pub mod intents;
            pub mod projections;
            pub mod state;
        }
        pub mod use_cases {
            pub mod register_time_entry {
                pub mod command;
                pub mod decide;
                pub mod decision;
                pub mod handler;
                pub mod inbound {
                    pub mod graphql;
                    pub mod http;
                }
            }
            pub mod list_time_entries_by_user {
                pub mod handler;
                pub mod inbound {
                    pub mod graphql;
                }
                pub mod projection;
                pub mod queries_port;
            }
        }
        pub mod adapters {
            pub mod outbound {
                pub mod event_store;
                pub mod intent_outbox;
                pub mod projections;
                pub mod projections_in_memory;
            }
        }
    }
}

pub mod shell;

#[cfg(test)]
pub mod tests {
    pub mod fixtures;

    pub mod e2e {
        pub mod list_time_entries_by_user_tests;
    }
}
