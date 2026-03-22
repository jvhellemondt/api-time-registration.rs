use async_graphql::{Context, Object, Result as GqlResult};
use chrono::Utc;

use crate::modules::tags::use_cases::set_tag_color::command::SetTagColor;
use crate::shell::state::AppState;

#[derive(Default)]
pub struct SetTagColorMutation;

#[Object]
impl SetTagColorMutation {
    async fn set_tag_color(
        &self,
        context: &Context<'_>,
        tag_id: String,
        color: String,
    ) -> GqlResult<bool> {
        let state = context.data_unchecked::<AppState>();
        let stream_id = format!("Tag-{tag_id}");
        let command = SetTagColor {
            tag_id: tag_id.clone(),
            tenant_id: "tenant-hardcoded".to_string(),
            color,
            set_at: Utc::now().timestamp_millis(),
            set_by: "user-from-auth".to_string(),
        };

        state
            .set_tag_color_handler
            .handle(&stream_id, command)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(true)
    }
}
