use async_graphql::{Context, Enum, Object, Result as GqlResult};

use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::{
    TimeEntryStatus, TimeEntryView,
};
use crate::shell::state::AppState;

#[derive(Debug, Enum, Copy, Clone, Eq, PartialEq)]
pub enum GqlTimeEntryStatus {
    Draft,
    Registered,
}

impl From<TimeEntryStatus> for GqlTimeEntryStatus {
    fn from(s: TimeEntryStatus) -> Self {
        match s {
            TimeEntryStatus::Draft => GqlTimeEntryStatus::Draft,
            TimeEntryStatus::Registered => GqlTimeEntryStatus::Registered,
        }
    }
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct GqlTimeEntry {
    pub time_entry_id: String,
    pub user_id: String,
    pub started_at: Option<i64>,
    pub ended_at: Option<i64>,
    pub status: GqlTimeEntryStatus,
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
            started_at: v.started_at,
            ended_at: v.ended_at,
            status: v.status.into(),
            created_at: v.created_at,
            created_by: v.created_by,
            updated_at: v.updated_at,
            updated_by: v.updated_by,
            deleted_at: v.deleted_at,
        }
    }
}

#[derive(Default)]
pub struct TimeEntryQueries;

#[Object]
impl TimeEntryQueries {
    async fn list_time_entries_by_user_id(
        &self,
        context: &Context<'_>,
        user_id: String,
        offset: Option<i64>,
        limit: Option<i64>,
        sort_desc: Option<bool>,
    ) -> GqlResult<Vec<GqlTimeEntry>> {
        let state = context.data_unchecked::<AppState>();
        let list: Vec<TimeEntryView> = state
            .list_time_entries_handler
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

#[cfg(test)]
mod list_time_entries_graphql_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn it_should_convert_draft_status_to_gql() {
        let gql: GqlTimeEntryStatus = TimeEntryStatus::Draft.into();
        assert_eq!(gql, GqlTimeEntryStatus::Draft);
    }

    #[rstest]
    fn it_should_convert_registered_status_to_gql() {
        let gql: GqlTimeEntryStatus = TimeEntryStatus::Registered.into();
        assert_eq!(gql, GqlTimeEntryStatus::Registered);
    }

    #[rstest]
    fn it_should_convert_time_entry_view_to_gql() {
        let view = TimeEntryView {
            time_entry_id: "te-0001".to_string(),
            user_id: "user-0001".to_string(),
            started_at: Some(1_000),
            ended_at: Some(2_000),
            status: TimeEntryStatus::Registered,
            created_at: 0,
            created_by: "user-0001".to_string(),
            updated_at: 0,
            updated_by: "user-0001".to_string(),
            deleted_at: None,
        };
        let gql = GqlTimeEntry::from(view);
        assert_eq!(gql.time_entry_id, "te-0001");
        assert_eq!(gql.started_at, Some(1_000));
        assert_eq!(gql.ended_at, Some(2_000));
        assert_eq!(gql.status, GqlTimeEntryStatus::Registered);
    }
}
