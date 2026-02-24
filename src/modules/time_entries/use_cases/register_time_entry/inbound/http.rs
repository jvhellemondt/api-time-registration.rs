use axum::{
    Json, extract::State, extract::rejection::JsonRejection, http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::time_entries::use_cases::register_time_entry::command::RegisterTimeEntry;
use crate::modules::time_entries::use_cases::register_time_entry::handler::ApplicationError;
use crate::shell::state::AppState;

#[derive(Deserialize)]
pub struct RegisterTimeEntryBody {
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
}

#[derive(Serialize)]
pub struct RegisterTimeEntryResponse {
    pub time_entry_id: String,
}

pub async fn handle(
    State(state): State<AppState>,
    body: Result<Json<RegisterTimeEntryBody>, JsonRejection>,
) -> impl IntoResponse {
    let Json(body) = match body {
        Ok(b) => b,
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };

    let time_entry_id = Uuid::now_v7();
    let stream_id = format!("TimeEntry-{time_entry_id}");

    let command = RegisterTimeEntry {
        time_entry_id: time_entry_id.to_string(),
        user_id: body.user_id,
        start_time: body.start_time,
        end_time: body.end_time,
        tags: body.tags,
        description: body.description,
        created_at: Utc::now().timestamp_millis(),
        created_by: "user-from-auth".into(),
    };

    match state.register_handler.handle(&stream_id, command).await {
        Ok(()) => (
            StatusCode::CREATED,
            Json(RegisterTimeEntryResponse {
                time_entry_id: time_entry_id.to_string(),
            }),
        )
            .into_response(),
        Err(ApplicationError::Domain(_)) => StatusCode::CONFLICT.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod register_time_entry_http_inbound_tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::post,
    };
    use http_body_util::BodyExt;
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::modules::time_entries::adapters::outbound::projections_in_memory::InMemoryProjections;
    use crate::modules::time_entries::core::events::TimeEntryEvent;
    use crate::modules::time_entries::use_cases::list_time_entries_by_user::handler::Projector;
    use crate::modules::time_entries::use_cases::register_time_entry::handler::RegisterTimeEntryHandler;
    use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
    use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
    use crate::shell::state::AppState;

    use super::handle;

    fn make_test_state() -> AppState {
        let event_store = Arc::new(InMemoryEventStore::<TimeEntryEvent>::new());
        let outbox = Arc::new(InMemoryDomainOutbox::new());
        let projections = Arc::new(InMemoryProjections::new());
        let projector = Arc::new(Projector::new(
            "test",
            projections.clone(),
            projections.clone(),
        ));
        let register_handler = Arc::new(RegisterTimeEntryHandler::new(
            "time-entries",
            event_store.clone(),
            outbox,
        ));
        AppState {
            queries: projections,
            register_handler,
            event_store,
            projector,
        }
    }

    fn make_offline_event_store_state() -> AppState {
        let mut event_store = InMemoryEventStore::<TimeEntryEvent>::new();
        event_store.toggle_offline();
        let event_store = Arc::new(event_store);
        let outbox = Arc::new(InMemoryDomainOutbox::new());
        let projections = Arc::new(InMemoryProjections::new());
        let projector = Arc::new(Projector::new(
            "test",
            projections.clone(),
            projections.clone(),
        ));
        let register_handler = Arc::new(RegisterTimeEntryHandler::new(
            "time-entries",
            event_store.clone(),
            outbox,
        ));
        AppState {
            queries: projections,
            register_handler,
            event_store,
            projector,
        }
    }

    fn app(state: AppState) -> Router {
        Router::new()
            .route("/register-time-entry", post(handle))
            .with_state(state)
    }

    #[tokio::test]
    async fn it_should_return_201_with_time_entry_id_on_valid_request() {
        let body = r#"{"user_id":"u-1","start_time":1000,"end_time":2000,"tags":["Work"],"description":"test"}"#;

        let response = app(make_test_state())
            .oneshot(
                Request::post("/register-time-entry")
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
    async fn it_should_return_409_when_domain_rejects_invalid_interval() {
        // end_time < start_time triggers DecideError::InvalidInterval -> ApplicationError::Domain -> 409
        let body =
            r#"{"user_id":"u-1","start_time":2000,"end_time":1000,"tags":[],"description":"test"}"#;

        let response = app(make_test_state())
            .oneshot(
                Request::post("/register-time-entry")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn it_should_return_422_on_invalid_json() {
        let response = app(make_test_state())
            .oneshot(
                Request::post("/register-time-entry")
                    .header("content-type", "application/json")
                    .body(Body::from("not-json"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn it_should_return_500_when_event_store_is_offline() {
        let body =
            r#"{"user_id":"u-1","start_time":1000,"end_time":2000,"tags":[],"description":"test"}"#;

        let response = app(make_offline_event_store_state())
            .oneshot(
                Request::post("/register-time-entry")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
