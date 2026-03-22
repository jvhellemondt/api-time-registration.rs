use axum::{
    Json, Router,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};

use crate::modules::tags::use_cases::create_tag::inbound::http as create_tag_http;
use crate::modules::tags::use_cases::delete_tag::inbound::http as delete_tag_http;
use crate::modules::tags::use_cases::list_tags::inbound::http as list_tags_http;
use crate::modules::tags::use_cases::set_tag_color::inbound::http as set_tag_color_http;
use crate::modules::tags::use_cases::set_tag_description::inbound::http as set_tag_description_http;
use crate::modules::tags::use_cases::set_tag_name::inbound::http as set_tag_name_http;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::inbound::http as list_http;
use crate::modules::time_entries::use_cases::register_time_entry::inbound::http as register_http;
use crate::shell::state::AppState;

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok"}))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/register-time-entry", post(register_http::handle))
        .route("/list-time-entries", get(list_http::handle))
        .route("/tags", get(list_tags_http::handle))
        .route("/tags", post(create_tag_http::handle))
        .route("/tags/{tag_id}", delete(delete_tag_http::handle))
        .route("/tags/{tag_id}/name", patch(set_tag_name_http::handle))
        .route("/tags/{tag_id}/color", patch(set_tag_color_http::handle))
        .route(
            "/tags/{tag_id}/description",
            patch(set_tag_description_http::handle),
        )
        .with_state(state)
}
