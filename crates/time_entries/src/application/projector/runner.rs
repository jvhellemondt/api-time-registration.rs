// Projector runner consumes a stream of events, translates them into mutations,
// persists them using a repository, and advances the watermark.
//
// Purpose
// - Guarantee idempotent application of events and safe recovery on failure.

use crate::application::projector::repository::{
    TimeEntryProjectionRepository, WatermarkRepository,
};
use crate::core::time_entry::event::TimeEntryEvent;
use crate::core::time_entry::projector::apply::{apply, Mutation};

pub struct Projector<'a, TRepository, TWatermarkRepository>
where
    TRepository: TimeEntryProjectionRepository,
    TWatermarkRepository: WatermarkRepository,
{
    pub name: String,
    pub repository: &'a TRepository,
    pub watermark_repository: &'a TWatermarkRepository,
}
impl<'a, TRepository, TWatermarkRepository> Projector<'a, TRepository, TWatermarkRepository>
where
    TRepository: TimeEntryProjectionRepository,
    TWatermarkRepository: WatermarkRepository,
{
    pub fn new(
        name: String,
        repository: &'a TRepository,
        watermark: &'a TWatermarkRepository,
    ) -> Self {
        Self {
            name,
            repository,
            watermark_repository: watermark,
        }
    }
}

impl<'a, TRepository, TWatermarkRepository> Projector<'a, TRepository, TWatermarkRepository>
where
    TRepository: TimeEntryProjectionRepository,
    TWatermarkRepository: WatermarkRepository,
{
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
    use crate::adapters::in_memory::in_memory_projections::InMemoryProjections;
    use crate::core::time_entry::event::v1::time_entry_registered::TimeEntryRegisteredV1;
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
    async fn it_should_apply_mutations_to_the_repository(before_each: (TimeEntryRegisteredV1, InMemoryProjections)) {
        let (event, store) = before_each;
        let projector = Projector::new("projector-name".to_string(), &store, &store);
        projector
            .apply_one(
                "time-entries-0001",
                0,
                &TimeEntryEvent::TimeEntryRegisteredV1(event),
            )
            .await
            .expect("InMemoryProjections > upsert failed");
        assert_eq!(
            store.get("projector-name").await.unwrap(),
            Some(String::from("time-entries-0001:0"))
        );
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_if_the_repository_is_offline(before_each: (TimeEntryRegisteredV1, InMemoryProjections)) {
        let (event, mut store) = before_each;
        store.toggle_offline();
        let projector = Projector::new("projector-name".to_string(), &store, &store);
        let result = projector
            .apply_one(
                "time-entries-0001",
                0,
                &TimeEntryEvent::TimeEntryRegisteredV1(event),
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Projections repository offline"));
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_if_the_watermark_repository_is_offline(before_each: (TimeEntryRegisteredV1, InMemoryProjections)) {
        let (event, store) = before_each;
        let mut watermark_repository = InMemoryProjections::new();
        watermark_repository.toggle_offline();
        let projector = Projector::new("projector-name".to_string(), &store, &watermark_repository);
        let result = projector
            .apply_one(
                "time-entries-0001",
                0,
                &TimeEntryEvent::TimeEntryRegisteredV1(event),
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Watermark repository offline"));
    }
}
