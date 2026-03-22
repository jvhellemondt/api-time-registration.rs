use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};

use crate::shell::state::AppState;

pub async fn handle(State(state): State<AppState>) -> impl IntoResponse {
    match state.list_tags_handler.list_all().await {
        Ok(tags) => Json(tags).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod list_tags_http_inbound_tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::handle;
    use crate::modules::tags::use_cases::list_tags::projection::{ListTagsState, TagRow};
    use crate::shared::infrastructure::projection_store::ProjectionStore;
    use crate::shell::state::AppState;
    use crate::tests::fixtures::tags::make_test_app_state;

    fn app(state: AppState) -> Router {
        Router::new().route("/tags", get(handle)).with_state(state)
    }

    #[tokio::test]
    async fn it_should_return_200_with_empty_list_when_no_tags_exist() {
        let response = app(make_test_app_state())
            .oneshot(Request::get("/tags").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json, serde_json::json!([]));
    }

    #[tokio::test]
    async fn it_should_return_200_with_tags_from_projection() {
        let state = make_test_app_state();
        let mut projection_state = ListTagsState::default();
        projection_state.rows.insert(
            "lt-1".to_string(),
            TagRow {
                tag_id: "lt-1".to_string(),
                tenant_id: "tenant-hardcoded".to_string(),
                name: "Work".to_string(),
                color: "#FFB3BA".to_string(),
                description: Some("Client work".to_string()),
                deleted: false,
                last_event_id: None,
            },
        );
        state
            .tag_projection_store
            .save(projection_state, 1)
            .await
            .unwrap();

        let response = app(state)
            .oneshot(Request::get("/tags").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 1);
        assert_eq!(json[0]["tag_id"], "lt-1");
        assert_eq!(json[0]["name"], "Work");
        assert_eq!(json[0]["description"], "Client work");
    }

    #[tokio::test]
    async fn it_should_return_500_when_projection_store_is_offline() {
        let mut state = make_test_app_state();
        state.tag_projection_store.toggle_offline();
        let response = app(state)
            .oneshot(Request::get("/tags").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
