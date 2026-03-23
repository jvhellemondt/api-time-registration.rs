use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::{Uuid, Version};

use crate::modules::time_entries::use_cases::set_ended_at::command::SetEndedAt;
use crate::modules::time_entries::use_cases::set_ended_at::handler::ApplicationError;
use crate::shared::infrastructure::request_context::RequestContext;
use crate::shell::state::AppState;

#[derive(Deserialize)]
pub struct SetEndedAtBody {
    pub ended_at: i64,
}

/// PUT /time-entries/{id}/end — sets/updates ended_at on an existing entry (creates if new)
pub async fn handle_put(
    State(state): State<AppState>,
    request_ctx: RequestContext,
    Path(time_entry_id): Path<String>,
    body: Result<Json<SetEndedAtBody>, JsonRejection>,
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

    let command = SetEndedAt {
        time_entry_id: time_entry_id.clone(),
        user_id: request_ctx.user_id.clone(),
        ended_at: body.ended_at,
        updated_at: Utc::now().timestamp_millis(),
        updated_by: request_ctx.user_id,
    };

    match state.set_ended_at_handler.handle(&stream_id, command).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(ApplicationError::Domain(_)) => StatusCode::CONFLICT.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod set_ended_at_http_inbound_tests {
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
            .route("/time-entries/{id}/end", put(handle_put))
            .with_state(state)
    }

    fn valid_v7_id() -> String {
        uuid::Uuid::now_v7().to_string()
    }

    #[tokio::test]
    async fn put_returns_200_on_valid_request() {
        let te_id = valid_v7_id();
        let body = r#"{"ended_at":1000}"#;
        let response = app(make_test_state())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/time-entries/{te_id}/end"))
                    .header("content-type", "application/json")
                    .header("x-user-id", "u-1")
                    .header("x-tenant-id", "tenant-test")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn put_returns_409_on_invalid_interval() {
        let state = make_test_app_state();
        use crate::modules::time_entries::use_cases::set_started_at::handler::SetStartedAtHandler;
        use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
        use crate::tests::fixtures::commands::set_started_at::SetStartedAtBuilder;

        let te_id = valid_v7_id();
        let stream_id = format!("TimeEntry-{te_id}");

        // Seed a draft with started_at=5000 via the handler directly
        SetStartedAtHandler::new("t", state.event_store.clone(), InMemoryDomainOutbox::new())
            .handle(
                &stream_id,
                SetStartedAtBuilder::new()
                    .time_entry_id(te_id.clone())
                    .started_at(5_000)
                    .build(),
            )
            .await
            .unwrap();

        // ended_at=3000 < started_at=5000 → invalid interval → 409
        let body = r#"{"ended_at":3000}"#;
        let response = app(state)
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/time-entries/{te_id}/end"))
                    .header("content-type", "application/json")
                    .header("x-user-id", "u-1")
                    .header("x-tenant-id", "tenant-test")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn put_returns_422_on_non_uuid() {
        let body = r#"{"ended_at":1000}"#;
        let response = app(make_test_state())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/time-entries/not-a-uuid/end")
                    .header("content-type", "application/json")
                    .header("x-user-id", "u-1")
                    .header("x-tenant-id", "tenant-test")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn put_returns_422_on_non_v7_uuid() {
        // UUID v4 string (not v7)
        let v4_id = "550e8400-e29b-41d4-a716-446655440000";
        let body = r#"{"ended_at":1000}"#;
        let response = app(make_test_state())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/time-entries/{v4_id}/end"))
                    .header("content-type", "application/json")
                    .header("x-user-id", "u-1")
                    .header("x-tenant-id", "tenant-test")
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
                    .uri(format!("/time-entries/{te_id}/end"))
                    .header("content-type", "application/json")
                    .header("x-user-id", "u-1")
                    .header("x-tenant-id", "tenant-test")
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
        let body = r#"{"ended_at":1000}"#;
        let response = app(make_offline_state())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/time-entries/{te_id}/end"))
                    .header("content-type", "application/json")
                    .header("x-user-id", "u-1")
                    .header("x-tenant-id", "tenant-test")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn put_returns_401_when_user_id_header_missing() {
        let te_id = valid_v7_id();
        let body = r#"{"ended_at":1000}"#;
        let response = app(make_test_state())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/time-entries/{te_id}/end"))
                    .header("content-type", "application/json")
                    .header("x-tenant-id", "tenant-test")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
