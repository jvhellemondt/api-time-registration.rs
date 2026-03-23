use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;

use crate::modules::tags::use_cases::delete_tag::command::DeleteTag;
use crate::modules::tags::use_cases::delete_tag::handler::ApplicationError;
use crate::shared::infrastructure::request_context::RequestContext;
use crate::shell::state::AppState;

pub async fn handle(
    State(state): State<AppState>,
    request_ctx: RequestContext,
    Path(tag_id): Path<String>,
) -> impl IntoResponse {
    let stream_id = format!("Tag-{tag_id}");
    let command = DeleteTag {
        tag_id: tag_id.clone(),
        tenant_id: request_ctx.tenant_id,
        deleted_at: Utc::now().timestamp_millis(),
        deleted_by: request_ctx.user_id,
    };

    match state.delete_tag_handler.handle(&stream_id, command).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(ApplicationError::Domain(_)) => StatusCode::CONFLICT.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod delete_tag_http_inbound_tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::delete,
    };
    use tower::ServiceExt;

    use super::handle;
    use crate::shell::state::AppState;
    use crate::tests::fixtures::tags::make_test_app_state;

    fn app(state: AppState) -> Router {
        Router::new()
            .route("/tags/{tag_id}", delete(handle))
            .with_state(state)
    }

    async fn create_tag(state: &AppState, tag_id: &str) {
        use crate::modules::tags::use_cases::create_tag::command::CreateTag;
        state
            .create_tag_handler
            .handle(
                &format!("Tag-{tag_id}"),
                CreateTag {
                    tag_id: tag_id.to_string(),
                    tenant_id: "tenant-hardcoded".to_string(),
                    name: "Work".to_string(),
                    color: "#FFB3BA".to_string(),
                    description: None,
                    created_at: 0,
                    created_by: "u1".to_string(),
                },
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn it_should_return_204_on_success() {
        let state = make_test_app_state();
        create_tag(&state, "t-del-1").await;
        let response = app(state)
            .oneshot(
                Request::delete("/tags/t-del-1")
                    .header("x-user-id", "u-1")
                    .header("x-tenant-id", "tenant-test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn it_should_return_409_when_tag_not_found() {
        let response = app(make_test_app_state())
            .oneshot(
                Request::delete("/tags/nonexistent")
                    .header("x-user-id", "u-1")
                    .header("x-tenant-id", "tenant-test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn it_should_return_409_when_tag_already_deleted() {
        let state = make_test_app_state();
        create_tag(&state, "t-del-2").await;
        app(state.clone())
            .oneshot(
                Request::delete("/tags/t-del-2")
                    .header("x-user-id", "u-1")
                    .header("x-tenant-id", "tenant-test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let response = app(state)
            .oneshot(
                Request::delete("/tags/t-del-2")
                    .header("x-user-id", "u-1")
                    .header("x-tenant-id", "tenant-test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn it_should_return_500_when_event_store_is_offline() {
        let state = make_test_app_state();
        state.tag_event_store.toggle_offline();
        let response = app(state)
            .oneshot(
                Request::delete("/tags/any")
                    .header("x-user-id", "u-1")
                    .header("x-tenant-id", "tenant-test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn it_should_return_401_when_user_id_header_missing() {
        let response = app(make_test_app_state())
            .oneshot(
                Request::delete("/tags/any")
                    .header("x-tenant-id", "tenant-test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
