// In memory projection repository, and watermark repository.
//
// Purpose
// - Exercise projectors without a database.
//
// Responsibilities
// - Store read model rows in a map keyed by identifiers.
// - Track the last processed event per projector.

use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::application::projector::repository::{TimeEntryProjectionRepository, WatermarkRepository};
use crate::core::time_entry::projector::model::TimeEntryRow;

#[derive(Default)]
pub struct InMemoryProjections {
    rows: RwLock<HashMap<(String, String), TimeEntryRow>>,
    watermark: RwLock<HashMap<String, String>>,
}
impl InMemoryProjections {
    pub fn new() -> Self { Self::default() }
}

#[async_trait::async_trait]
impl TimeEntryProjectionRepository for InMemoryProjections {
    async fn upsert(&self, row: TimeEntryRow) -> anyhow::Result<()> {
        let mut guard = self.rows.write().await;
        guard.insert((row.user_id.clone(), row.time_entry_id.clone()), row);
        Ok(())
    }
}
#[async_trait::async_trait]
impl WatermarkRepository for InMemoryProjections {
    async fn get(&self, name: &str) -> anyhow::Result<Option<String>> {
        Ok(self.watermark.read().await.get(name).cloned())
    }
    async fn set(&self, name: &str, last: &str) -> anyhow::Result<()> {
        self.watermark.write().await.insert(name.to_string(), last.to_string());
        Ok(())
    }
}

#[cfg(test)]
pub mod time_entry_in_memory_projections_tests {
    use rstest::rstest;
    use crate::test_support::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;
    use super::*;

    #[rstest]
    #[tokio::test]
    async fn it_should_add_the_time_entry_row_to_the_repository() {
        let event = make_time_entry_registered_v1_event();
        let row = TimeEntryRow {
            time_entry_id: event.time_entry_id.clone(),
            user_id: event.user_id.clone(),
            start_time: event.start_time,
            end_time: event.end_time,
            tags: event.tags.clone(),
            description: event.description.clone(),
            created_at: event.created_at,
            created_by: event.created_by.clone(),
            updated_at: event.created_at,
            updated_by: event.created_by.clone(),
            deleted_at: None,
            last_event_id: None,
        };

        let repository = InMemoryProjections::new();
        repository.upsert(row.clone()).await.expect("InMemoryProjections > upsert failed");

        assert_eq!(repository.rows.read().await.len(), 1);
        assert_eq!(repository.rows.read().await.get(&(event.user_id.clone(), event.time_entry_id.clone())).unwrap(), &row);
    }


    #[rstest]
    #[tokio::test]
    async fn it_should_set_the_watermark_and_confirm_its_set() {
        let repository = InMemoryProjections::new();
        repository.set("projector-name", "event-id").await.expect("InMemoryProjections > set failed");
        assert_eq!(repository.get("projector-name").await.unwrap(), Some(String::from("event-id")));
        // assert_eq!(repository.watermark.read().await.get("projector-name").unwrap(), "event-id");
    }
}
