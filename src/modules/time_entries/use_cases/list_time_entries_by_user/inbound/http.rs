use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

use crate::shell::state::AppState;

#[derive(Deserialize)]
pub struct ListTimeEntriesParams {
    pub user_id: String,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub sort_desc: Option<bool>,
}

pub async fn handle(
    State(state): State<AppState>,
    Query(params): Query<ListTimeEntriesParams>,
) -> impl IntoResponse {
    match state
        .list_time_entries_handler
        .list_by_user_id(
            &params.user_id,
            params.offset.unwrap_or(0),
            params.limit.unwrap_or(20),
            params.sort_desc.unwrap_or(true),
        )
        .await
    {
        Ok(entries) => Json(entries).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod list_time_entries_by_user_http_inbound_tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::handle;
    use crate::modules::time_entries::core::events::TimeEntryEvent;
    use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::ListTimeEntriesState;
    use crate::modules::time_entries::use_cases::list_time_entries_by_user::queries::ListTimeEntriesQueryHandler;
    use crate::modules::time_entries::use_cases::register_time_entry::handler::RegisterTimeEntryHandler;
    use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
    use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
    use crate::shared::infrastructure::projection_store::in_memory::InMemoryProjectionStore;
    use crate::shell::state::AppState;

    fn make_failing_queries_state() -> AppState {
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new();
        event_store.toggle_offline();
        let outbox = InMemoryDomainOutbox::new();
        let mut projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        projection_store.toggle_offline();
        let register_time_entry_handler =
            RegisterTimeEntryHandler::new("time-entries", event_store.clone(), outbox.clone());
        let list_time_entries_handler = ListTimeEntriesQueryHandler::new(projection_store);
        AppState {
            list_time_entries_handler,
            register_time_entry_handler,
            event_store,
            outbox,
        }
    }

    fn make_test_state() -> AppState {
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new();
        event_store.toggle_offline();
        let outbox = InMemoryDomainOutbox::new();
        let projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        let register_time_entry_handler =
            RegisterTimeEntryHandler::new("time-entries", event_store.clone(), outbox.clone());
        let list_time_entries_handler = ListTimeEntriesQueryHandler::new(projection_store);
        AppState {
            list_time_entries_handler,
            register_time_entry_handler,
            event_store,
            outbox,
        }
    }

    fn app(state: AppState) -> Router {
        Router::new()
            .route("/list-time-entries", get(handle))
            .with_state(state)
    }

    #[tokio::test]
    async fn it_should_return_200_with_empty_list_when_no_entries_exist() {
        let response = app(make_test_state())
            .oneshot(
                Request::get("/list-time-entries?user_id=u-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json, serde_json::json!([]));
    }

    #[tokio::test]
    async fn it_should_return_400_when_user_id_is_missing() {
        let response = app(make_test_state())
            .oneshot(
                Request::get("/list-time-entries")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn it_should_return_500_when_queries_fail() {
        let response = app(make_failing_queries_state())
            .oneshot(
                Request::get("/list-time-entries?user_id=u-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
