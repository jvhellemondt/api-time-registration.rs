use async_graphql::{Context, Object, Result as GqlResult};
use chrono::Utc;

use crate::modules::tags::use_cases::set_tag_description::command::SetTagDescription;
use crate::shell::state::AppState;

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
        let state = context.data_unchecked::<AppState>();
        let stream_id = format!("Tag-{tag_id}");
        let command = SetTagDescription {
            tag_id: tag_id.clone(),
            tenant_id: "tenant-hardcoded".to_string(),
            description,
            set_at: Utc::now().timestamp_millis(),
            set_by: "user-from-auth".to_string(),
        };

        state
            .set_tag_description_handler
            .handle(&stream_id, command)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(true)
    }
}
