use async_trait::async_trait;
use serde_json::Value as Json;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct OutboxRow {
    pub topic: String,
    pub event_type: String,
    pub event_version: i32,
    pub stream_id: String,
    pub stream_version: i64,
    pub occurred_at: i64,
    pub payload: Json,
}

#[derive(Debug, Error)]
pub enum OutboxError {
    #[error("duplicate outbox row for stream {stream_id} v{stream_version}")]
    Duplicate {
        stream_id: String,
        stream_version: i64,
    },

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("transient backend error: {0}")]
    Transient(String),

    #[error("backend error: {0}")]
    Backend(String),
}

#[async_trait]
pub trait DomainOutbox: Send + Sync {
    async fn enqueue(&self, row: OutboxRow) -> Result<(), OutboxError>;
}

pub mod in_memory;
