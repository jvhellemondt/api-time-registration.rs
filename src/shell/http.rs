use axum::{
    Router,
    routing::{get, post},
};

use crate::modules::time_entries::use_cases::list_time_entries_by_user::inbound::http as list_http;
use crate::modules::time_entries::use_cases::register_time_entry::inbound::http as register_http;
use crate::shell::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/register-time-entry", post(register_http::handle))
        .route("/list-time-entries", get(list_http::handle))
        .with_state(state)
}
