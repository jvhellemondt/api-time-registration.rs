use super::ProjectionStore;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;

struct InnerState<Projection> {
    state: Option<Projection>,
    checkpoint: u64,
    schema_version: Option<u32>,
}

struct Inner<Projection: Clone + Send + Sync + 'static> {
    state: RwLock<InnerState<Projection>>,
    is_offline: AtomicBool,
    fail_next_save: AtomicBool,
    fail_next_save_schema: AtomicBool,
}

#[derive(Clone)]
pub struct InMemoryProjectionStore<Projection: Clone + Send + Sync + 'static> {
    inner: Arc<Inner<Projection>>,
}

impl<P: Clone + Send + Sync + 'static> Default for InMemoryProjectionStore<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Clone + Send + Sync + 'static> InMemoryProjectionStore<P> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                state: RwLock::new(InnerState {
                    state: None,
                    checkpoint: 0,
                    schema_version: None,
                }),
                is_offline: AtomicBool::new(false),
                fail_next_save: AtomicBool::new(false),
                fail_next_save_schema: AtomicBool::new(false),
            }),
        }
    }

    pub fn toggle_offline(&mut self) {
        self.inner.is_offline.fetch_xor(true, Ordering::SeqCst);
    }

    pub fn is_offline(&self) -> bool {
        self.inner.is_offline.load(Ordering::SeqCst)
    }

    pub fn set_fail_next_save(&self) {
        self.inner.fail_next_save.store(true, Ordering::SeqCst);
    }

    pub fn set_fail_next_save_schema_version(&self) {
        self.inner
            .fail_next_save_schema
            .store(true, Ordering::SeqCst);
    }
}

#[async_trait::async_trait]
impl<P: Clone + Send + Sync + 'static> ProjectionStore<P> for InMemoryProjectionStore<P> {
    async fn state(&self) -> anyhow::Result<Option<P>> {
        if self.is_offline() {
            return Err(anyhow::anyhow!("Projection store offline"));
        }
        Ok(self.inner.state.read().await.state.clone())
    }

    async fn checkpoint(&self) -> anyhow::Result<u64> {
        if self.is_offline() {
            return Err(anyhow::anyhow!("Projection store offline"));
        }
        Ok(self.inner.state.read().await.checkpoint)
    }

    async fn schema_version(&self) -> anyhow::Result<Option<u32>> {
        if self.is_offline() {
            return Err(anyhow::anyhow!("Projection store offline"));
        }
        Ok(self.inner.state.read().await.schema_version)
    }

    async fn save(&self, state: P, checkpoint: u64) -> anyhow::Result<()> {
        if self.is_offline() {
            return Err(anyhow::anyhow!("Projection store offline"));
        }
        if self.inner.fail_next_save.swap(false, Ordering::SeqCst) {
            return Err(anyhow::anyhow!("Injected save failure"));
        }
        let mut inner = self.inner.state.write().await;
        inner.state = Some(state);
        inner.checkpoint = checkpoint;
        Ok(())
    }

    async fn save_schema_version(&self, version: u32) -> anyhow::Result<()> {
        if self.is_offline() {
            return Err(anyhow::anyhow!("Projection store offline"));
        }
        if self
            .inner
            .fail_next_save_schema
            .swap(false, Ordering::SeqCst)
        {
            return Err(anyhow::anyhow!("Injected schema version save failure"));
        }
        self.inner.state.write().await.schema_version = Some(version);
        Ok(())
    }

    async fn clear(&self) -> anyhow::Result<()> {
        if self.is_offline() {
            return Err(anyhow::anyhow!("Projection store offline"));
        }
        let mut inner = self.inner.state.write().await;
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
