use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::TimeEntryView;
use async_trait::async_trait;

#[async_trait]
pub trait TimeEntryQueries {
    async fn list_by_user_id(
        &self,
        user_id: &str,
        offset: u64,
        limit: u64,
        sort_by_start_time_desc: bool,
    ) -> anyhow::Result<Vec<TimeEntryView>>;
}
