use thiserror::Error;
use crate::core::ports::{EventStoreError, OutboxError};

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    VersionConflict(#[from] EventStoreError),

    #[error(transparent)]
    Outbox(#[from] OutboxError),

    #[error("domain rejected: {0}")]
    Domain(String),

    #[error("unexpected: {0}")]
    Unexpected(String),
}
