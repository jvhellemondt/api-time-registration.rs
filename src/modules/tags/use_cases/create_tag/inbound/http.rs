use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::tags::use_cases::create_tag::command::{CreateTag, pick_pastel_color};
use crate::modules::tags::use_cases::create_tag::decision::DecideError;
use crate::modules::tags::use_cases::create_tag::handler::ApplicationError;
use crate::shell::state::AppState;

#[derive(Deserialize)]
pub struct CreateTagBody {
    pub tag_id: Option<String>,
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct CreateTagResponse {
    pub tag_id: String,
}

pub async fn handle(
    State(state): State<AppState>,
    body: Result<Json<CreateTagBody>, JsonRejection>,
) -> impl IntoResponse {
    let Json(body) = match body {
        Ok(b) => b,
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };

    let tag_id = body
        .tag_id
        .as_deref()
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::now_v7);
    let stream_id = format!("Tag-{tag_id}");
    let color = body
        .color
        .unwrap_or_else(|| pick_pastel_color().to_string());

    let command = CreateTag {
        tag_id: tag_id.to_string(),
        tenant_id: "tenant-hardcoded".to_string(),
        name: body.name,
        color,
        description: body.description,
        created_at: Utc::now().timestamp_millis(),
        created_by: "user-from-auth".to_string(),
    };

    match state.create_tag_handler.handle(&stream_id, command).await {
        Ok(()) => (
            StatusCode::CREATED,
            Json(CreateTagResponse {
                tag_id: tag_id.to_string(),
            }),
        )
            .into_response(),
        Err(ApplicationError::Domain(DecideError::TagAlreadyExists)) => {
            StatusCode::CONFLICT.into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod create_tag_http_inbound_tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::post,
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::handle;
    use crate::shell::state::AppState;
    use crate::tests::fixtures::tags::make_test_app_state;

    fn app(state: AppState) -> Router {
        Router::new().route("/tags", post(handle)).with_state(state)
    }

    #[tokio::test]
    async fn it_should_return_201_with_tag_id() {
        let body = r##"{"name":"Work","color":"#FFB3BA"}"##;
        let response = app(make_test_app_state())
            .oneshot(
                Request::post("/tags")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json.get("tag_id").is_some());
    }

    #[tokio::test]
    async fn it_should_return_201_with_random_pastel_when_no_color_given() {
        use crate::modules::tags::use_cases::create_tag::command::PASTEL_COLORS;
        let body = r#"{"name":"Work"}"#;
        let state = make_test_app_state();
        let response = app(state.clone())
            .oneshot(
                Request::post("/tags")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        // Ensure all possible pastels are valid hex strings
        for p in PASTEL_COLORS {
            assert!(p.starts_with('#'));
        }
    }

    #[tokio::test]
    async fn it_should_return_201_with_description() {
        let body = r##"{"name":"Work","color":"#FFB3BA","description":"Client work"}"##;
        let response = app(make_test_app_state())
            .oneshot(
                Request::post("/tags")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn it_should_return_409_on_duplicate() {
        let state = make_test_app_state();
        let known_id = uuid::Uuid::now_v7().to_string();
        let body = format!(r##"{{"tag_id":"{known_id}","name":"Work","color":"#FFB3BA"}}"##);
        // First create succeeds
        let resp1 = app(state.clone())
            .oneshot(
                Request::post("/tags")
                    .header("content-type", "application/json")
                    .body(Body::from(body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp1.status(), StatusCode::CREATED);
        // Second create with the same tag_id → 409
        let resp2 = app(state)
            .oneshot(
                Request::post("/tags")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp2.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn it_should_return_422_on_invalid_json() {
        let response = app(make_test_app_state())
            .oneshot(
                Request::post("/tags")
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
        let state = make_test_app_state();
        state.tag_event_store.toggle_offline();
        let body = r##"{"name":"Work","color":"#FFB3BA"}"##;
        let response = app(state)
            .oneshot(
                Request::post("/tags")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
