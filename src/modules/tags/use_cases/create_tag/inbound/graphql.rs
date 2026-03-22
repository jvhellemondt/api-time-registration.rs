use async_graphql::{Context, ID, Object, Result as GqlResult};
use chrono::Utc;
use uuid::Uuid;

use crate::modules::tags::use_cases::create_tag::command::{CreateTag, pick_pastel_color};
use crate::shell::state::AppState;

#[derive(Default)]
pub struct CreateTagMutation;

#[Object]
impl CreateTagMutation {
    async fn create_tag(
        &self,
        context: &Context<'_>,
        name: String,
        color: Option<String>,
        description: Option<String>,
    ) -> GqlResult<ID> {
        let state = context.data_unchecked::<AppState>();
        let tag_id = Uuid::now_v7();
        let stream_id = format!("Tag-{tag_id}");
        let resolved_color = color.unwrap_or_else(|| pick_pastel_color().to_string());

        let command = CreateTag {
            tag_id: tag_id.to_string(),
            tenant_id: "tenant-hardcoded".to_string(),
            name,
            color: resolved_color,
            description,
            created_at: Utc::now().timestamp_millis(),
            created_by: "user-from-auth".to_string(),
        };

        state
            .create_tag_handler
            .handle(&stream_id, command)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(ID(tag_id.to_string()))
    }
}
