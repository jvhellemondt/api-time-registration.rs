use async_graphql::{Context, Object, Result as GqlResult};

use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::TimeEntryView;
use crate::shell::state::AppState;

#[derive(async_graphql::SimpleObject, Clone)]
pub struct GqlTimeEntry {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub created_by: String,
    pub updated_at: i64,
    pub updated_by: String,
    pub deleted_at: Option<i64>,
}

impl From<TimeEntryView> for GqlTimeEntry {
    fn from(v: TimeEntryView) -> Self {
        Self {
            time_entry_id: v.time_entry_id,
            user_id: v.user_id,
            start_time: v.start_time,
            end_time: v.end_time,
            tags: v.tags,
            description: v.description,
            created_at: v.created_at,
            created_by: v.created_by,
            updated_at: v.updated_at,
            updated_by: v.updated_by,
            deleted_at: v.deleted_at,
        }
    }
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn list_time_entries_by_user_id(
        &self,
        context: &Context<'_>,
        user_id: String,
        offset: Option<i64>,
        limit: Option<i64>,
        sort_desc: Option<bool>,
    ) -> GqlResult<Vec<GqlTimeEntry>> {
        let state = context.data_unchecked::<AppState>();
        let list = state
            .queries
            .list_by_user_id(
                &user_id,
                offset.unwrap_or(0).max(0) as u64,
                limit.unwrap_or(20).max(0) as u64,
                sort_desc.unwrap_or(true),
            )
            .await?;
        Ok(list.into_iter().map(Into::into).collect())
    }
}
