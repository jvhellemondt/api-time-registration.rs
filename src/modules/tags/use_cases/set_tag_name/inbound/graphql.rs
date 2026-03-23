use async_graphql::{Context, Object, Result as GqlResult};
use chrono::Utc;

use crate::modules::tags::use_cases::set_tag_name::command::SetTagName;
use crate::shared::infrastructure::request_context::RequestContext;
use crate::shell::state::AppState;

#[cfg(test)]
mod set_tag_name_graphql_inbound_tests {
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
                    name: "original-name".to_string(),
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
                    r#"mutation {{ setTagName(tagId: "{tag_id}", name: "new-name") }}"#
                ))
                .data(req_ctx()),
            )
            .await;
        assert!(result.errors.is_empty());
        assert_eq!(result.data.to_string(), "{setTagName: true}");
    }

    #[tokio::test]
    async fn returns_error_when_event_store_offline() {
        let state = make_test_app_state();
        state.tag_event_store.toggle_offline();
        let schema = make_schema_from_state(state);
        let result = schema
            .execute(
                async_graphql::Request::new(
                    r#"mutation { setTagName(tagId: "some-id", name: "new-name") }"#,
                )
                .data(req_ctx()),
            )
            .await;
        assert!(!result.errors.is_empty());
    }
}

#[derive(Default)]
pub struct SetTagNameMutation;

#[Object]
impl SetTagNameMutation {
    async fn set_tag_name(
        &self,
        context: &Context<'_>,
        tag_id: String,
        name: String,
    ) -> GqlResult<bool> {
        let req_ctx = context
            .data::<RequestContext>()
            .map_err(|_| async_graphql::Error::new("Unauthorized"))?;
        let state = context.data_unchecked::<AppState>();
        let stream_id = format!("Tag-{tag_id}");
        let command = SetTagName {
            tag_id: tag_id.clone(),
            tenant_id: req_ctx.tenant_id.clone(),
            name,
            set_at: Utc::now().timestamp_millis(),
            set_by: req_ctx.user_id.clone(),
        };

        state
            .set_tag_name_handler
            .handle(&stream_id, command)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(true)
    }
}
