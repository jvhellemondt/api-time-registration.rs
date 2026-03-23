use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use serde::Deserialize;

use crate::modules::tags::use_cases::set_tag_color::command::SetTagColor;
use crate::modules::tags::use_cases::set_tag_color::handler::ApplicationError;
use crate::shared::infrastructure::request_context::RequestContext;
use crate::shell::state::AppState;

#[derive(Deserialize)]
pub struct SetTagColorBody {
    pub color: String,
}

pub async fn handle(
    State(state): State<AppState>,
    request_ctx: RequestContext,
    Path(tag_id): Path<String>,
    body: Result<Json<SetTagColorBody>, JsonRejection>,
) -> impl IntoResponse {
    let Json(body) = match body {
        Ok(b) => b,
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };

    let stream_id = format!("Tag-{tag_id}");
    let command = SetTagColor {
        tag_id: tag_id.clone(),
        tenant_id: request_ctx.tenant_id,
        color: body.color,
        set_at: Utc::now().timestamp_millis(),
        set_by: request_ctx.user_id,
    };

    match state
        .set_tag_color_handler
        .handle(&stream_id, command)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(ApplicationError::Domain(_)) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod set_tag_color_http_inbound_tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::patch,
    };
    use tower::ServiceExt;

    use super::handle;
    use crate::shell::state::AppState;
    use crate::tests::fixtures::tags::make_test_app_state;

    fn app(state: AppState) -> Router {
        Router::new()
            .route("/tags/{tag_id}/color", patch(handle))
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
        create_tag(&state, "t-sc-1").await;
        let response = app(state)
            .oneshot(
                Request::patch("/tags/t-sc-1/color")
                    .header("content-type", "application/json")
                    .header("x-user-id", "u-1")
                    .header("x-tenant-id", "tenant-test")
                    .body(Body::from(r##"{"color":"#BAE1FF"}"##))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn it_should_return_422_when_tag_not_found() {
        let response = app(make_test_app_state())
            .oneshot(
                Request::patch("/tags/nonexistent/color")
                    .header("content-type", "application/json")
                    .header("x-user-id", "u-1")
                    .header("x-tenant-id", "tenant-test")
                    .body(Body::from(r##"{"color":"#BAE1FF"}"##))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn it_should_return_422_on_invalid_json() {
        let response = app(make_test_app_state())
            .oneshot(
                Request::patch("/tags/any/color")
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
    async fn it_should_return_500_when_event_store_is_offline() {
        let state = make_test_app_state();
        state.tag_event_store.toggle_offline();
        let response = app(state)
            .oneshot(
                Request::patch("/tags/any/color")
                    .header("content-type", "application/json")
                    .header("x-user-id", "u-1")
                    .header("x-tenant-id", "tenant-test")
                    .body(Body::from(r##"{"color":"#BAE1FF"}"##))
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
                Request::patch("/tags/any/color")
                    .header("content-type", "application/json")
                    .header("x-tenant-id", "tenant-test")
                    .body(Body::from(r##"{"color":"#BAE1FF"}"##))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
