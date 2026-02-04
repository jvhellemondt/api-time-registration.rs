// Trait for fetching all time entries by user_id from the projection store.
//
// Purpose
// - Abstract data access so that different storage backends can implement it.

use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimeEntryView {
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
