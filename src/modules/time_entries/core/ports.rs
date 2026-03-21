use crate::shared::core::primitives::Tag;

/// Port consumed by the register_time_entry inbound adapter to resolve tag IDs
/// to full Tag values before building the command. Defined here (time_entries
/// core) so the shell can inject a concrete implementation without creating a
/// cross-module dependency between time_entries and tags.
#[async_trait::async_trait]
pub trait TagLookupPort: Send + Sync {
    async fn resolve(&self, tenant_id: &str, tag_ids: &[String]) -> Result<Vec<Tag>, TagLookupError>;
}

#[derive(Debug, thiserror::Error)]
pub enum TagLookupError {
    #[error("tag not found: {0}")]
    TagNotFound(String),
    #[error("tag lookup unavailable")]
    Unavailable,
}
