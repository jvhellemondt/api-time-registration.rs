use async_graphql::{Context, ID, Object, Result as GqlResult};
use chrono::Utc;
use uuid::Uuid;

use crate::modules::time_entries::use_cases::set_ended_at::command::SetEndedAt;
use crate::shell::state::AppState;

#[derive(Default)]
pub struct SetEndedAtMutation;

#[Object]
impl SetEndedAtMutation {
    async fn create_with_ended_at(
        &self,
        context: &Context<'_>,
        user_id: String,
        ended_at: i64,
    ) -> GqlResult<ID> {
        let state = context.data_unchecked::<AppState>();
        let time_entry_id = Uuid::now_v7().to_string();
        let stream_id = format!("TimeEntry-{time_entry_id}");

        let command = SetEndedAt {
            time_entry_id: time_entry_id.clone(),
            user_id,
            ended_at,
            updated_at: Utc::now().timestamp_millis(),
            updated_by: "user-from-auth".to_string(),
        };

        state
            .set_ended_at_handler
            .handle(&stream_id, command)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(ID(time_entry_id))
    }

    async fn set_ended_at(
        &self,
        context: &Context<'_>,
        time_entry_id: String,
        user_id: String,
        ended_at: i64,
    ) -> GqlResult<bool> {
        let state = context.data_unchecked::<AppState>();
        let stream_id = format!("TimeEntry-{time_entry_id}");

        let command = SetEndedAt {
            time_entry_id,
            user_id,
            ended_at,
            updated_at: Utc::now().timestamp_millis(),
            updated_by: "user-from-auth".to_string(),
        };

        state
            .set_ended_at_handler
            .handle(&stream_id, command)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(true)
    }
}
