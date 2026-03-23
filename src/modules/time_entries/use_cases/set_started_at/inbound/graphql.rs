use async_graphql::{Context, Object, Result as GqlResult};
use chrono::Utc;
use uuid::{Uuid, Version};

use crate::modules::time_entries::use_cases::set_started_at::command::SetStartedAt;
use crate::shared::infrastructure::request_context::RequestContext;
use crate::shell::state::AppState;

#[cfg(test)]
mod set_started_at_graphql_inbound_tests {
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

    fn valid_v7_id() -> String {
        uuid::Uuid::now_v7().to_string()
    }

    #[tokio::test]
    async fn returns_true_on_valid_input() {
        let te_id = valid_v7_id();
        let schema = make_schema_from_state(make_test_app_state());
        let result = schema
            .execute(
                async_graphql::Request::new(format!(
                    r#"mutation {{ setStartedAt(timeEntryId: "{te_id}", startedAt: 1000) }}"#
                ))
                .data(req_ctx()),
            )
            .await;
        assert!(result.errors.is_empty());
        assert_eq!(result.data.to_string(), "{setStartedAt: true}");
    }

    #[tokio::test]
    async fn returns_error_on_non_v7_uuid() {
        let v4_id = "550e8400-e29b-41d4-a716-446655440000";
        let schema = make_schema_from_state(make_test_app_state());
        let result = schema
            .execute(
                async_graphql::Request::new(format!(
                    r#"mutation {{ setStartedAt(timeEntryId: "{v4_id}", startedAt: 1000) }}"#
                ))
                .data(req_ctx()),
            )
            .await;
        assert!(!result.errors.is_empty());
    }

    #[tokio::test]
    async fn returns_error_on_domain_rejection() {
        use crate::tests::fixtures::commands::set_ended_at::SetEndedAtBuilder;

        let state = make_test_app_state();
        let te_id = valid_v7_id();
        let stream_id = format!("TimeEntry-{te_id}");

        // Seed ended_at=1000; then started_at=2000 creates an invalid interval
        state
            .set_ended_at_handler
            .handle(
                &stream_id,
                SetEndedAtBuilder::new()
                    .time_entry_id(te_id.clone())
                    .ended_at(1_000)
                    .build(),
            )
            .await
            .unwrap();

        let schema = make_schema_from_state(state);
        let result = schema
            .execute(
                async_graphql::Request::new(format!(
                    r#"mutation {{ setStartedAt(timeEntryId: "{te_id}", startedAt: 2000) }}"#
                ))
                .data(req_ctx()),
            )
            .await;
        assert!(!result.errors.is_empty());
    }

    #[tokio::test]
    async fn returns_error_when_event_store_offline() {
        let state = make_test_app_state();
        state.event_store.toggle_offline();
        let te_id = valid_v7_id();
        let schema = make_schema_from_state(state);
        let result = schema
            .execute(
                async_graphql::Request::new(format!(
                    r#"mutation {{ setStartedAt(timeEntryId: "{te_id}", startedAt: 1000) }}"#
                ))
                .data(req_ctx()),
            )
            .await;
        assert!(!result.errors.is_empty());
    }
}

#[derive(Default)]
pub struct SetStartedAtMutation;

#[Object]
impl SetStartedAtMutation {
    async fn set_started_at(
        &self,
        context: &Context<'_>,
        time_entry_id: String,
        started_at: i64,
    ) -> GqlResult<bool> {
        Uuid::parse_str(&time_entry_id)
            .ok()
            .filter(|u| u.get_version() == Some(Version::SortRand))
            .ok_or_else(|| async_graphql::Error::new("time_entry_id must be a valid UUID v7"))?;

        let req_ctx = context
            .data::<RequestContext>()
            .map_err(|_| async_graphql::Error::new("Unauthorized"))?;
        let state = context.data_unchecked::<AppState>();
        let stream_id = format!("TimeEntry-{time_entry_id}");

        let command = SetStartedAt {
            time_entry_id,
            user_id: req_ctx.user_id.clone(),
            started_at,
            updated_at: Utc::now().timestamp_millis(),
            updated_by: req_ctx.user_id.clone(),
        };

        state
            .set_started_at_handler
            .handle(&stream_id, command)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(true)
    }
}
