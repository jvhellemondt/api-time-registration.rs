// Repository traits for projection persistence and projector watermark tracking.
//
// Purpose
// - TimeEntryProjectionRepository: upsert and patch read model rows.
// - WatermarkRepository: track the last processed event for idempotency.

use async_trait::async_trait;
use crate::core::time_entry::projector::model::TimeEntryRow;

#[async_trait]
pub trait TimeEntryProjectionRepository: Send + Sync {
    async fn upsert(&self, row: TimeEntryRow) -> anyhow::Result<()>;
}

#[async_trait]
pub trait WatermarkRepository: Send + Sync {
    async fn get(&self, name: &str) -> anyhow::Result<Option<String>>;
    async fn set(&self, name: &str, last: &str) -> anyhow::Result<()>;
}
