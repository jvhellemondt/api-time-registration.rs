use super::ProjectionStore;
use tokio::sync::RwLock;

struct Inner<P> {
    state: Option<P>,
    checkpoint: u64,
    schema_version: Option<u32>,
}

pub struct InMemoryProjectionStore<P: Clone + Send + Sync + 'static> {
    inner: RwLock<Inner<P>>,
    is_offline: bool,
}

impl<P: Clone + Send + Sync + 'static> Default for InMemoryProjectionStore<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Clone + Send + Sync + 'static> InMemoryProjectionStore<P> {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Inner {
                state: None,
                checkpoint: 0,
                schema_version: None,
            }),
            is_offline: false,
        }
    }

    pub fn toggle_offline(&mut self) {
        self.is_offline = !self.is_offline;
    }
}

#[async_trait::async_trait]
impl<P: Clone + Send + Sync + 'static> ProjectionStore<P> for InMemoryProjectionStore<P> {
    async fn state(&self) -> anyhow::Result<Option<P>> {
        if self.is_offline {
            return Err(anyhow::anyhow!("Projection store offline"));
        }
        Ok(self.inner.read().await.state.clone())
    }

    async fn checkpoint(&self) -> anyhow::Result<u64> {
        if self.is_offline {
            return Err(anyhow::anyhow!("Projection store offline"));
        }
        Ok(self.inner.read().await.checkpoint)
    }

    async fn schema_version(&self) -> anyhow::Result<Option<u32>> {
        if self.is_offline {
            return Err(anyhow::anyhow!("Projection store offline"));
        }
        Ok(self.inner.read().await.schema_version)
    }

    async fn save(&self, state: P, checkpoint: u64) -> anyhow::Result<()> {
        if self.is_offline {
            return Err(anyhow::anyhow!("Projection store offline"));
        }
        let mut inner = self.inner.write().await;
        inner.state = Some(state);
        inner.checkpoint = checkpoint;
        Ok(())
    }

    async fn save_schema_version(&self, version: u32) -> anyhow::Result<()> {
        if self.is_offline {
            return Err(anyhow::anyhow!("Projection store offline"));
        }
        self.inner.write().await.schema_version = Some(version);
        Ok(())
    }

    async fn clear(&self) -> anyhow::Result<()> {
        if self.is_offline {
            return Err(anyhow::anyhow!("Projection store offline"));
        }
        let mut inner = self.inner.write().await;
        inner.state = None;
        inner.checkpoint = 0;
        Ok(())
    }
}

#[cfg(test)]
mod in_memory_projection_store_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[tokio::test]
    async fn it_should_return_none_state_and_zero_checkpoint_initially() {
        let store = InMemoryProjectionStore::<String>::new();
        assert_eq!(store.state().await.unwrap(), None);
        assert_eq!(store.checkpoint().await.unwrap(), 0);
        assert_eq!(store.schema_version().await.unwrap(), None);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_save_and_load_state_with_checkpoint() {
        let store = InMemoryProjectionStore::<String>::new();
        store
            .save("hello".to_string(), 42)
            .await
            .expect("save failed");
        assert_eq!(store.state().await.unwrap(), Some("hello".to_string()));
        assert_eq!(store.checkpoint().await.unwrap(), 42);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_save_and_load_schema_version() {
        let store = InMemoryProjectionStore::<String>::new();
        store.save_schema_version(3).await.expect("save failed");
        assert_eq!(store.schema_version().await.unwrap(), Some(3));
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_clear_state_and_reset_checkpoint() {
        let store = InMemoryProjectionStore::<String>::new();
        store.save("data".to_string(), 10).await.unwrap();
        store.clear().await.expect("clear failed");
        assert_eq!(store.state().await.unwrap(), None);
        assert_eq!(store.checkpoint().await.unwrap(), 0);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_return_default_via_default_trait() {
        let store = InMemoryProjectionStore::<String>::default();
        assert_eq!(store.state().await.unwrap(), None);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_all_operations_when_offline() {
        let mut store = InMemoryProjectionStore::<String>::new();
        store.toggle_offline();
        assert!(store.state().await.is_err());
        assert!(store.checkpoint().await.is_err());
        assert!(store.schema_version().await.is_err());
        assert!(store.save("x".to_string(), 1).await.is_err());
        assert!(store.save_schema_version(1).await.is_err());
        assert!(store.clear().await.is_err());
    }
}
