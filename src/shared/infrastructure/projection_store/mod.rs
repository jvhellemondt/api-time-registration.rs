use async_trait::async_trait;

#[async_trait]
pub trait ProjectionStore<P: Clone + Send + Sync + 'static>: Send + Sync {
    async fn state(&self) -> anyhow::Result<Option<P>>;
    async fn checkpoint(&self) -> anyhow::Result<u64>;
    async fn schema_version(&self) -> anyhow::Result<Option<u32>>;
    async fn save(&self, state: P, checkpoint: u64) -> anyhow::Result<()>;
    async fn save_schema_version(&self, version: u32) -> anyhow::Result<()>;
    async fn clear(&self) -> anyhow::Result<()>;
}

pub mod in_memory;
