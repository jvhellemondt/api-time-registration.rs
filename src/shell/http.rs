use axum::{
    Json, Router,
    response::IntoResponse,
    routing::{delete, get, patch, post, put},
};

use crate::modules::tags::use_cases::create_tag::inbound::http as create_tag_http;
use crate::modules::tags::use_cases::delete_tag::inbound::http as delete_tag_http;
use crate::modules::tags::use_cases::list_tags::inbound::http as list_tags_http;
use crate::modules::tags::use_cases::set_tag_color::inbound::http as set_tag_color_http;
use crate::modules::tags::use_cases::set_tag_description::inbound::http as set_tag_description_http;
use crate::modules::tags::use_cases::set_tag_name::inbound::http as set_tag_name_http;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::inbound::http as list_http;
use crate::modules::time_entries::use_cases::set_ended_at::inbound::http as set_ended_at_http;
use crate::modules::time_entries::use_cases::set_started_at::inbound::http as set_started_at_http;
use crate::modules::time_entries::use_cases::set_time_entry_tags::inbound::http as set_time_entry_tags_http;
use crate::shell::state::AppState;

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok"}))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route(
            "/time-entries/{id}/start",
            put(set_started_at_http::handle_put),
        )
        .route("/time-entries/{id}/end", put(set_ended_at_http::handle_put))
        .route(
            "/time-entries/{id}/tags",
            put(set_time_entry_tags_http::handle_put),
        )
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
