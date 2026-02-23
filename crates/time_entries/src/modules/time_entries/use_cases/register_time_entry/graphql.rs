use async_graphql::{Context, ID, Object, Result as GqlResult};
use chrono::Utc;
use uuid::Uuid;

use crate::modules::time_entries::use_cases::register_time_entry::command::RegisterTimeEntry;
use crate::shared::infrastructure::event_store::EventStore;
use crate::shell::state::AppState;

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn register_time_entry(
        &self,
        context: &Context<'_>,
        user_id: String,
        start_time: i64,
        end_time: i64,
        tags: Vec<String>,
        description: String,
    ) -> GqlResult<ID> {
        let time_entry_id = Uuid::now_v7();
        let state = context.data_unchecked::<AppState>();

        let command = RegisterTimeEntry {
            time_entry_id: time_entry_id.to_string(),
            user_id,
            start_time,
            end_time,
            tags,
            description,
            created_at: Utc::now().timestamp_millis(),
            created_by: "user-from-auth".into(),
        };

        let stream_id = format!("TimeEntry-{time_entry_id}");

        state
            .register_handler
            .handle(&stream_id, command)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Inline projection so queries see the new row immediately
        let loaded = state
            .event_store
            .load(&stream_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        if let Some(last) = loaded.events.last() {
            state
                .projector
                .apply_one(&stream_id, loaded.version, last)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }

        Ok(ID(time_entry_id.to_string()))
    }
}
