use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::{Uuid, Version};

use crate::modules::time_entries::use_cases::set_time_entry_tags::command::SetTimeEntryTags;
use crate::shell::state::AppState;

#[derive(Deserialize)]
pub struct SetTimeEntryTagsBody {
    pub user_id: String,
    pub tag_ids: Vec<String>,
}

/// PUT /time-entries/{id}/tags — sets/replaces tags on a time entry (creates if new)
pub async fn handle_put(
    State(state): State<AppState>,
    Path(time_entry_id): Path<String>,
    body: Result<Json<SetTimeEntryTagsBody>, JsonRejection>,
) -> impl IntoResponse {
    let is_valid_v7 = Uuid::parse_str(&time_entry_id)
        .ok()
        .filter(|u| u.get_version() == Some(Version::SortRand))
        .is_some();
    if !is_valid_v7 {
        return StatusCode::UNPROCESSABLE_ENTITY.into_response();
    }

    let Json(body) = match body {
        Ok(b) => b,
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };

    let stream_id = format!("TimeEntry-{time_entry_id}");

    let command = SetTimeEntryTags {
        time_entry_id: time_entry_id.clone(),
        user_id: body.user_id,
        tag_ids: body.tag_ids,
        updated_at: Utc::now().timestamp_millis(),
        updated_by: "user-from-auth".to_string(),
    };

    match state
        .set_time_entry_tags_handler
        .handle(&stream_id, command)
        .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod set_time_entry_tags_http_inbound_tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::put,
    };
    use tower::ServiceExt;

    use super::handle_put;
    use crate::shell::state::AppState;
    use crate::tests::fixtures::tags::make_test_app_state;

    fn make_test_state() -> AppState {
        make_test_app_state()
    }

    fn make_offline_state() -> AppState {
        let state = make_test_app_state();
        state.event_store.toggle_offline();
        state
    }

    fn app(state: AppState) -> Router {
        Router::new()
            .route("/time-entries/{id}/tags", put(handle_put))
            .with_state(state)
    }

    fn valid_v7_id() -> String {
        uuid::Uuid::now_v7().to_string()
    }

    #[tokio::test]
    async fn put_returns_200_on_valid_request() {
        let te_id = valid_v7_id();
        let body = r#"{"user_id":"u-1","tag_ids":["tag-1","tag-2"]}"#;
        let response = app(make_test_state())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/time-entries/{te_id}/tags"))
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn put_returns_200_with_empty_tags() {
        let te_id = valid_v7_id();
        let body = r#"{"user_id":"u-1","tag_ids":[]}"#;
        let response = app(make_test_state())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/time-entries/{te_id}/tags"))
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn put_returns_422_on_non_uuid() {
        let body = r#"{"user_id":"u-1","tag_ids":["tag-1"]}"#;
        let response = app(make_test_state())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/time-entries/not-a-uuid/tags")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn put_returns_422_on_non_v7_uuid() {
        let v4_id = "550e8400-e29b-41d4-a716-446655440000";
        let body = r#"{"user_id":"u-1","tag_ids":["tag-1"]}"#;
        let response = app(make_test_state())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/time-entries/{v4_id}/tags"))
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn put_returns_422_on_invalid_json() {
        let te_id = valid_v7_id();
        let response = app(make_test_state())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/time-entries/{te_id}/tags"))
                    .header("content-type", "application/json")
                    .body(Body::from("not-json"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn put_returns_500_when_event_store_offline() {
        let te_id = valid_v7_id();
        let body = r#"{"user_id":"u-1","tag_ids":["tag-1"]}"#;
        let response = app(make_offline_state())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/time-entries/{te_id}/tags"))
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
