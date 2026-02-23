use crate::modules::time_entries::adapters::outbound::projections::{
    TimeEntryProjectionRepository, WatermarkRepository,
};
use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::projections::{Mutation, apply};
use std::sync::Arc;

#[derive(Clone)]
pub struct Projector<TRepository, TWatermarkRepository>
where
    TRepository: TimeEntryProjectionRepository + Send + Sync + 'static,
    TWatermarkRepository: WatermarkRepository + Send + Sync + 'static,
{
    pub name: String,
    pub repository: Arc<TRepository>,
    pub watermark_repository: Arc<TWatermarkRepository>,
}

impl<TRepository, TWatermarkRepository> Projector<TRepository, TWatermarkRepository>
where
    TRepository: TimeEntryProjectionRepository + Send + Sync + 'static,
    TWatermarkRepository: WatermarkRepository + Send + Sync + 'static,
{
    pub fn new(
        name: impl Into<String>,
        repository: Arc<TRepository>,
        watermark: Arc<TWatermarkRepository>,
    ) -> Self {
        Self {
            name: name.into(),
            repository,
            watermark_repository: watermark,
        }
    }

    pub async fn apply_one(
        &self,
        stream_id: &str,
        version: i64,
        event: &TimeEntryEvent,
    ) -> anyhow::Result<()> {
        for mutation in apply(stream_id, version, event) {
            match mutation {
                Mutation::Upsert(row) => self.repository.upsert(row).await?,
            }
        }
        self.watermark_repository
            .set(&self.name, &format!("{stream_id}:{version}"))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod time_entry_projector_runner_tests {
    use super::*;
    use crate::modules::time_entries::adapters::outbound::projections_in_memory::InMemoryProjections;
    use crate::modules::time_entries::core::events::v1::time_entry_registered::TimeEntryRegisteredV1;
    use crate::tests::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;
    use rstest::{fixture, rstest};

    #[fixture]
    fn before_each() -> (TimeEntryRegisteredV1, InMemoryProjections) {
        let event = make_time_entry_registered_v1_event();
        let repository = InMemoryProjections::new();
        (event, repository)
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_apply_mutations_to_the_repository(
        before_each: (TimeEntryRegisteredV1, InMemoryProjections),
    ) {
        let (event, store) = before_each;
        let st = Arc::new(store);
        let projector = Projector::new("projector-name".to_string(), st.clone(), st.clone());
        projector
            .apply_one(
                "time-entries-0001",
                0,
                &TimeEntryEvent::TimeEntryRegisteredV1(event),
            )
            .await
            .expect("apply_one failed");
        assert_eq!(
            st.get("projector-name").await.unwrap(),
            Some(String::from("time-entries-0001:0"))
        );
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_if_the_repository_is_offline(
        before_each: (TimeEntryRegisteredV1, InMemoryProjections),
    ) {
        let (event, mut store) = before_each;
        store.toggle_offline();
        let st = Arc::new(store);
        let projector = Projector::new("projector-name".to_string(), st.clone(), st.clone());
        let result = projector
            .apply_one(
                "time-entries-0001",
                0,
                &TimeEntryEvent::TimeEntryRegisteredV1(event),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Projections repository offline")
        );
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_if_the_watermark_repository_is_offline(
        before_each: (TimeEntryRegisteredV1, InMemoryProjections),
    ) {
        let (event, store) = before_each;
        let mut watermark_repository = InMemoryProjections::new();
        watermark_repository.toggle_offline();
        let wm = Arc::new(watermark_repository);
        let projector = Projector::new("projector-name".to_string(), Arc::new(store), wm);
        let result = projector
            .apply_one(
                "time-entries-0001",
                0,
                &TimeEntryEvent::TimeEntryRegisteredV1(event),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Watermark repository offline")
        );
    }
}
