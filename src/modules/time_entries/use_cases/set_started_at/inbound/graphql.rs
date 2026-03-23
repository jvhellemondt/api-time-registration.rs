use async_graphql::{Context, Object, Result as GqlResult};
use chrono::Utc;
use uuid::{Uuid, Version};

use crate::modules::time_entries::use_cases::set_started_at::command::SetStartedAt;
use crate::shell::state::AppState;

#[derive(Default)]
pub struct SetStartedAtMutation;

#[Object]
impl SetStartedAtMutation {
    async fn set_started_at(
        &self,
        context: &Context<'_>,
        time_entry_id: String,
        user_id: String,
        started_at: i64,
    ) -> GqlResult<bool> {
        Uuid::parse_str(&time_entry_id)
            .ok()
            .filter(|u| u.get_version() == Some(Version::SortRand))
            .ok_or_else(|| async_graphql::Error::new("time_entry_id must be a valid UUID v7"))?;

        let state = context.data_unchecked::<AppState>();
        let stream_id = format!("TimeEntry-{time_entry_id}");

        let command = SetStartedAt {
            time_entry_id,
            user_id,
            started_at,
            updated_at: Utc::now().timestamp_millis(),
            updated_by: "user-from-auth".to_string(),
        };

        state
            .set_started_at_handler
            .handle(&stream_id, command)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(true)
    }
}
