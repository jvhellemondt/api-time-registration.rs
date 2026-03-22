use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::time_entries::use_cases::set_started_at::command::SetStartedAt;
use crate::modules::time_entries::use_cases::set_started_at::handler::ApplicationError;
use crate::shell::state::AppState;

#[derive(Deserialize)]
pub struct SetStartedAtBody {
    pub user_id: String,
    pub started_at: i64,
}

#[derive(Serialize)]
pub struct SetStartedAtResponse {
    pub time_entry_id: String,
}

/// POST /time-entries/start — creates a new time entry draft with started_at
pub async fn handle_post(
    State(state): State<AppState>,
    body: Result<Json<SetStartedAtBody>, JsonRejection>,
) -> impl IntoResponse {
    let Json(body) = match body {
        Ok(b) => b,
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };

    let time_entry_id = Uuid::now_v7().to_string();
    let stream_id = format!("TimeEntry-{time_entry_id}");

    let command = SetStartedAt {
        time_entry_id: time_entry_id.clone(),
        user_id: body.user_id,
        started_at: body.started_at,
        updated_at: Utc::now().timestamp_millis(),
        updated_by: "user-from-auth".to_string(),
    };

    match state
        .set_started_at_handler
        .handle(&stream_id, command)
        .await
    {
        Ok(()) => (
            StatusCode::CREATED,
            Json(SetStartedAtResponse { time_entry_id }),
        )
            .into_response(),
        Err(ApplicationError::Domain(_)) => StatusCode::CONFLICT.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// PUT /time-entries/{id}/start — sets/updates started_at on an existing entry
pub async fn handle_put(
    State(state): State<AppState>,
    Path(time_entry_id): Path<String>,
    body: Result<Json<SetStartedAtBody>, JsonRejection>,
) -> impl IntoResponse {
    let Json(body) = match body {
        Ok(b) => b,
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };

    let stream_id = format!("TimeEntry-{time_entry_id}");

    let command = SetStartedAt {
        time_entry_id: time_entry_id.clone(),
        user_id: body.user_id,
        started_at: body.started_at,
        updated_at: Utc::now().timestamp_millis(),
        updated_by: "user-from-auth".to_string(),
    };

    match state
        .set_started_at_handler
        .handle(&stream_id, command)
        .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(ApplicationError::Domain(_)) => StatusCode::CONFLICT.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod set_started_at_http_inbound_tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::{post, put},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::{handle_post, handle_put};
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
            .route("/time-entries/start", post(handle_post))
            .route("/time-entries/{id}/start", put(handle_put))
            .with_state(state)
    }

    #[tokio::test]
    async fn post_returns_201_with_time_entry_id() {
        let body = r#"{"user_id":"u-1","started_at":1000}"#;
        let response = app(make_test_state())
            .oneshot(
                Request::post("/time-entries/start")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json.get("time_entry_id").is_some());
    }

    #[tokio::test]
    async fn put_returns_200_on_valid_request() {
        let state = make_test_state();
        // First create a draft via POST
        let post_body = r#"{"user_id":"u-1","started_at":1000}"#;
        let post_response = app(state.clone())
            .oneshot(
                Request::post("/time-entries/start")
                    .header("content-type", "application/json")
                    .body(Body::from(post_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = post_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let post_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let te_id = post_json["time_entry_id"].as_str().unwrap().to_string();

        // Now PUT to update started_at
        let put_body = r#"{"user_id":"u-1","started_at":2000}"#;
        let response = app(state)
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/time-entries/{te_id}/start"))
                    .header("content-type", "application/json")
                    .body(Body::from(put_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn post_returns_409_on_invalid_interval() {
        // Create a draft with ended_at, then try to set started_at >= ended_at
        let state = make_test_app_state();
        // Create draft via set_ended_at first, then set started_at to invalid
        use crate::modules::time_entries::use_cases::set_ended_at::handler::SetEndedAtHandler;
        use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
        use crate::tests::fixtures::commands::set_ended_at::SetEndedAtBuilder;
        let te_id = "te-conflict-test";
        let stream_id = format!("TimeEntry-{te_id}");
        SetEndedAtHandler::new("t", state.event_store.clone(), InMemoryDomainOutbox::new())
            .handle(
                &stream_id,
                SetEndedAtBuilder::new()
                    .time_entry_id(te_id.to_string())
                    .ended_at(1_000)
                    .build(),
            )
            .await
            .unwrap();

        // Now try to PUT started_at >= ended_at (1000)
        let body = r#"{"user_id":"u-1","started_at":2000}"#;
        let response = app(state)
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/time-entries/{te_id}/start"))
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn post_returns_422_on_invalid_json() {
        let response = app(make_test_state())
            .oneshot(
                Request::post("/time-entries/start")
                    .header("content-type", "application/json")
                    .body(Body::from("not-json"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn put_returns_422_on_invalid_json() {
        let response = app(make_test_state())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/time-entries/some-id/start")
                    .header("content-type", "application/json")
                    .body(Body::from("not-json"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn post_returns_500_when_event_store_offline() {
        let body = r#"{"user_id":"u-1","started_at":1000}"#;
        let response = app(make_offline_state())
            .oneshot(
                Request::post("/time-entries/start")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn put_returns_500_when_event_store_offline() {
        let body = r#"{"user_id":"u-1","started_at":1000}"#;
        let response = app(make_offline_state())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/time-entries/some-id/start")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
