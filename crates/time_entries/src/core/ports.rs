// Ports define what the core needs from the outside world, without implementing it.
//
// Purpose
// - Describe abstract input and output capabilities as traits (for example: EventStore, DomainOutbox).
//
// Responsibilities
// - Keep the core independent of any database or broker by coding against traits.
//
// Boundaries
// - No concrete input or output here. Adapters implement these traits in the adapters layer.
//
// Testing guidance
// - Provide in memory implementations for tests and local development.

use async_trait::async_trait;
use thiserror::Error;
use serde_json::Value as Json;

#[derive(Debug, Error)]
pub enum EventStoreError {
    #[error("version mismatch: expected {expected}, actual {actual}")]
    VersionMismatch { expected: i64, actual: i64 },

    #[error("backend error: {0}")]
    Backend(String),
}

#[derive(Debug, Clone)]
pub struct LoadedStream<E> {
    pub events: Vec<E>,
    pub version: i64,
}

#[async_trait]
pub trait EventStore<Event: Clone + Send + Sync + 'static>: Send + Sync {
    async fn load(&self, stream_id: &str) -> Result<LoadedStream<Event>, EventStoreError>;
    async fn append(
        &self,
        stream_id: &str,
        expected_version: i64,
        new_events: &[Event],
    ) -> Result<(), EventStoreError>;
}

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
    Duplicate { stream_id: String, stream_version: i64 },

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
