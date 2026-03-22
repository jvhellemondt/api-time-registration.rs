use async_graphql::{Context, Object, Result as GqlResult};
use chrono::Utc;

use crate::modules::tags::use_cases::set_tag_name::command::SetTagName;
use crate::shell::state::AppState;

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
        let state = context.data_unchecked::<AppState>();
        let stream_id = format!("Tag-{tag_id}");
        let command = SetTagName {
            tag_id: tag_id.clone(),
            tenant_id: "tenant-hardcoded".to_string(),
            name,
            set_at: Utc::now().timestamp_millis(),
            set_by: "user-from-auth".to_string(),
        };

        state
            .set_tag_name_handler
            .handle(&stream_id, command)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(true)
    }
}
