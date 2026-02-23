use async_graphql::{EmptySubscription, Schema};

pub use crate::modules::time_entries::use_cases::list_time_entries_by_user::graphql::QueryRoot;
pub use crate::modules::time_entries::use_cases::register_time_entry::graphql::MutationRoot;
pub use crate::shell::state::AppState;

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;
