use async_graphql::{Context, Object, Result as GqlResult};
use chrono::Utc;

use crate::modules::tags::use_cases::delete_tag::command::DeleteTag;
use crate::shell::state::AppState;

#[derive(Default)]
pub struct DeleteTagMutation;

#[Object]
impl DeleteTagMutation {
    async fn delete_tag(&self, context: &Context<'_>, tag_id: String) -> GqlResult<bool> {
        let state = context.data_unchecked::<AppState>();
        let stream_id = format!("Tag-{tag_id}");
        let command = DeleteTag {
            tag_id: tag_id.clone(),
            tenant_id: "tenant-hardcoded".to_string(),
            deleted_at: Utc::now().timestamp_millis(),
            deleted_by: "user-from-auth".to_string(),
        };

        state
            .delete_tag_handler
            .handle(&stream_id, command)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(true)
    }
}
