use async_graphql::{Context, ID, Object, Result as GqlResult};
use chrono::Utc;
use uuid::Uuid;

use crate::modules::tags::use_cases::create_tag::command::{CreateTag, pick_pastel_color};
use crate::shared::infrastructure::request_context::RequestContext;
use crate::shell::state::AppState;

#[cfg(test)]
mod create_tag_graphql_inbound_tests {
    use async_graphql::{EmptySubscription, Schema};

    use crate::shared::infrastructure::request_context::RequestContext;
    use crate::shell::graphql::{MutationRoot, QueryRoot};
    use crate::tests::fixtures::tags::make_test_app_state;

    fn make_schema_from_state(
        state: crate::shell::state::AppState,
    ) -> Schema<QueryRoot, MutationRoot, EmptySubscription> {
        Schema::build(
            QueryRoot::default(),
            MutationRoot::default(),
            EmptySubscription,
        )
        .data(state)
        .finish()
    }

    fn req_ctx() -> RequestContext {
        RequestContext {
            user_id: "u-1".to_string(),
            tenant_id: "tenant-test".to_string(),
        }
    }

    #[tokio::test]
    async fn returns_id_on_success() {
        let schema = make_schema_from_state(make_test_app_state());
        let result = schema
            .execute(
                async_graphql::Request::new(r#"mutation { createTag(name: "my-tag") }"#)
                    .data(req_ctx()),
            )
            .await;
        assert!(result.errors.is_empty());
        // ID is a non-empty string returned as a JSON string value
        let data = result.data.to_string();
        assert!(data.contains("createTag"));
        assert!(data.len() > r#"{"createTag":""}"#.len());
    }

    #[tokio::test]
    async fn returns_id_with_optional_color_and_description() {
        let schema = make_schema_from_state(make_test_app_state());
        let result = schema
            .execute(
                async_graphql::Request::new(
                    r##"mutation { createTag(name: "my-tag", color: "#ff0000", description: "desc") }"##,
                )
                .data(req_ctx()),
            )
            .await;
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn returns_error_when_event_store_offline() {
        let state = make_test_app_state();
        state.tag_event_store.toggle_offline();
        let schema = make_schema_from_state(state);
        let result = schema
            .execute(
                async_graphql::Request::new(r#"mutation { createTag(name: "my-tag") }"#)
                    .data(req_ctx()),
            )
            .await;
        assert!(!result.errors.is_empty());
    }
}

#[derive(Default)]
pub struct CreateTagMutation;

#[Object]
impl CreateTagMutation {
    async fn create_tag(
        &self,
        context: &Context<'_>,
        name: String,
        color: Option<String>,
        description: Option<String>,
    ) -> GqlResult<ID> {
        let req_ctx = context
            .data::<RequestContext>()
            .map_err(|_| async_graphql::Error::new("Unauthorized"))?;
        let state = context.data_unchecked::<AppState>();
        let tag_id = Uuid::now_v7();
        let stream_id = format!("Tag-{tag_id}");
        let resolved_color = color.unwrap_or_else(|| pick_pastel_color().to_string());

        let command = CreateTag {
            tag_id: tag_id.to_string(),
            tenant_id: req_ctx.tenant_id.clone(),
            name,
            color: resolved_color,
            description,
            created_at: Utc::now().timestamp_millis(),
            created_by: req_ctx.user_id.clone(),
        };

        state
            .create_tag_handler
            .handle(&stream_id, command)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(ID(tag_id.to_string()))
    }
}
