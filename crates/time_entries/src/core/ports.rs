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
    async fn append(&self, stream_id: &str, expected_version: i64, new_events: &[Event]) -> Result<(), EventStoreError>;
}

#[async_trait]
pub trait DomainOutbox: Send + Sync {
    async fn enqueue(&self, topic: &str, payload: &serde_json::Value) -> anyhow::Result<()>;
}
