pub mod shared {
    pub mod core {
        pub mod primitives;
    }
    pub mod infrastructure {
        pub mod event_store;
        pub mod intent_outbox;
        pub mod projection_store;
        pub mod request_context;
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
            pub mod set_started_at {
                pub mod command;
                pub mod decide;
                pub mod decision;
                pub mod handler;
                pub mod inbound {
                    pub mod graphql;
                    pub mod http;
                }
            }
            pub mod set_ended_at {
                pub mod command;
                pub mod decide;
                pub mod decision;
                pub mod handler;
                pub mod inbound {
                    pub mod graphql;
                    pub mod http;
                }
            }
            pub mod list_time_entries {
                pub mod inbound {
                    pub mod graphql;
                    pub mod http;
                }
                pub mod projection;
                pub mod projector;
                pub mod queries;
            }
            pub mod set_time_entry_tags {
                pub mod command;
                pub mod decide;
                pub mod decision;
                pub mod handler;
                pub mod inbound {
                    pub mod graphql;
                    pub mod http;
                }
            }
        }
        pub mod adapters {
            pub mod outbound {
                pub mod event_store;
                pub mod intent_outbox;
            }
        }
    }
    pub mod tags {
        pub mod core {
            pub mod events;
            pub mod evolve;
            pub mod projections;
            pub mod state;
        }
        pub mod use_cases {
            pub mod create_tag {
                pub mod command;
                pub mod decide;
                pub mod decision;
                pub mod handler;
                pub mod inbound {
                    pub mod graphql;
                    pub mod http;
                }
            }
            pub mod delete_tag {
                pub mod command;
                pub mod decide;
                pub mod decision;
                pub mod handler;
                pub mod inbound {
                    pub mod graphql;
                    pub mod http;
                }
            }
            pub mod set_tag_name {
                pub mod command;
                pub mod decide;
                pub mod decision;
                pub mod handler;
                pub mod inbound {
                    pub mod graphql;
                    pub mod http;
                }
            }
            pub mod set_tag_color {
                pub mod command;
                pub mod decide;
                pub mod decision;
                pub mod handler;
                pub mod inbound {
                    pub mod graphql;
                    pub mod http;
                }
            }
            pub mod set_tag_description {
                pub mod command;
                pub mod decide;
                pub mod decision;
                pub mod handler;
                pub mod inbound {
                    pub mod graphql;
                    pub mod http;
                }
            }
            pub mod list_tags {
                pub mod inbound {
                    pub mod graphql;
                    pub mod http;
                }
                pub mod projection;
                pub mod projector;
                pub mod queries;
            }
        }
        pub mod adapters {
            pub mod outbound {
                pub mod event_store;
            }
        }
    }
}

pub mod shell;

#[cfg(test)]
pub mod tests {
    pub mod fixtures;

    pub mod e2e {
        pub mod list_time_entries_tests;
    }
}
