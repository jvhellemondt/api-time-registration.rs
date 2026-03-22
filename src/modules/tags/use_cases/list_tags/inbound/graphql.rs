use async_graphql::{Context, Object, Result as GqlResult};

use crate::modules::tags::use_cases::list_tags::projection::TagView;
use crate::shell::state::AppState;

#[derive(async_graphql::SimpleObject, Clone)]
pub struct GqlTag {
    pub tag_id: String,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

impl From<TagView> for GqlTag {
    fn from(v: TagView) -> Self {
        Self {
            tag_id: v.tag_id,
            name: v.name,
            color: v.color,
            description: v.description,
        }
    }
}

#[derive(Default)]
pub struct ListTagsQuery;

#[Object]
impl ListTagsQuery {
    async fn list_tags(&self, context: &Context<'_>) -> GqlResult<Vec<GqlTag>> {
        let state = context.data_unchecked::<AppState>();
        let tags: Vec<TagView> = state.list_tags_handler.list_all().await?;
        Ok(tags.into_iter().map(Into::into).collect())
    }
}
