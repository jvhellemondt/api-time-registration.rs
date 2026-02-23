use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::TimeEntryRow;
use async_trait::async_trait;

#[async_trait]
pub trait TimeEntryProjectionRepository: Send + Sync {
    async fn upsert(&self, row: TimeEntryRow) -> anyhow::Result<()>;
}

#[async_trait]
pub trait WatermarkRepository: Send + Sync {
    async fn get(&self, name: &str) -> anyhow::Result<Option<String>>;
    async fn set(&self, name: &str, last: &str) -> anyhow::Result<()>;
}
