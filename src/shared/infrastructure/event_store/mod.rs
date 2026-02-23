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
    async fn append(
        &self,
        stream_id: &str,
        expected_version: i64,
        new_events: &[Event],
    ) -> Result<(), EventStoreError>;
}

pub mod in_memory;
