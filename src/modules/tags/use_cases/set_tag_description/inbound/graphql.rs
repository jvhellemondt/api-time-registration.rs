use async_graphql::{Context, Object, Result as GqlResult};
use chrono::Utc;

use crate::modules::tags::use_cases::set_tag_description::command::SetTagDescription;
use crate::shared::infrastructure::request_context::RequestContext;
use crate::shell::state::AppState;

#[cfg(test)]
mod set_tag_description_graphql_inbound_tests {
    use async_graphql::{EmptySubscription, Schema};

    use crate::modules::tags::use_cases::create_tag::command::{CreateTag, pick_pastel_color};
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

    async fn seed_tag(state: &crate::shell::state::AppState) -> String {
        let tag_id = uuid::Uuid::now_v7().to_string();
        let stream_id = format!("Tag-{tag_id}");
        state
            .create_tag_handler
            .handle(
                &stream_id,
                CreateTag {
                    tag_id: tag_id.clone(),
                    tenant_id: "tenant-hardcoded".to_string(),
                    name: "test-tag".to_string(),
                    color: pick_pastel_color().to_string(),
                    description: None,
                    created_at: 0,
                    created_by: "user-from-auth".to_string(),
                },
            )
            .await
            .unwrap();
        tag_id
    }

    #[tokio::test]
    async fn returns_true_on_success() {
        let state = make_test_app_state();
        let tag_id = seed_tag(&state).await;
        let schema = make_schema_from_state(state);
        let result = schema
            .execute(
                async_graphql::Request::new(format!(
                    r#"mutation {{ setTagDescription(tagId: "{tag_id}", description: "a description") }}"#
                ))
                .data(req_ctx()),
            )
            .await;
        assert!(result.errors.is_empty());
        assert_eq!(result.data.to_string(), "{setTagDescription: true}");
    }

    #[tokio::test]
    async fn returns_true_when_description_is_null() {
        let state = make_test_app_state();
        let tag_id = seed_tag(&state).await;
        let schema = make_schema_from_state(state);
        let result = schema
            .execute(
                async_graphql::Request::new(format!(
                    r#"mutation {{ setTagDescription(tagId: "{tag_id}") }}"#
                ))
                .data(req_ctx()),
            )
            .await;
        assert!(result.errors.is_empty());
        assert_eq!(result.data.to_string(), "{setTagDescription: true}");
    }

    #[tokio::test]
    async fn returns_error_when_event_store_offline() {
        let state = make_test_app_state();
        state.tag_event_store.toggle_offline();
        let schema = make_schema_from_state(state);
        let result = schema
            .execute(
                async_graphql::Request::new(
                    r#"mutation { setTagDescription(tagId: "some-id", description: "desc") }"#,
                )
                .data(req_ctx()),
            )
            .await;
        assert!(!result.errors.is_empty());
    }
}

#[derive(Default)]
pub struct SetTagDescriptionMutation;

#[Object]
impl SetTagDescriptionMutation {
    async fn set_tag_description(
        &self,
        context: &Context<'_>,
        tag_id: String,
        description: Option<String>,
    ) -> GqlResult<bool> {
        let req_ctx = context
            .data::<RequestContext>()
            .map_err(|_| async_graphql::Error::new("Unauthorized"))?;
        let state = context.data_unchecked::<AppState>();
        let stream_id = format!("Tag-{tag_id}");
        let command = SetTagDescription {
            tag_id: tag_id.clone(),
            tenant_id: req_ctx.tenant_id.clone(),
            description,
            set_at: Utc::now().timestamp_millis(),
            set_by: req_ctx.user_id.clone(),
        };

        state
            .set_tag_description_handler
            .handle(&stream_id, command)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(true)
    }
}
