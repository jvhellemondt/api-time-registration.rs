// In memory projection repository, and watermark repository.
//
// Purpose
// - Exercise projectors without a database.
//
// Responsibilities
// - Store read model rows in a map keyed by identifiers.
// - Track the last processed event per projector.

use crate::application::projector::repository::{
    TimeEntryProjectionRepository, WatermarkRepository,
};
use crate::application::query_handlers::time_entries_queries::{
    TimeEntryQueries, TimeEntryView,
};
use crate::core::time_entry::projector::model::TimeEntryRow;
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct InMemoryProjections {
    rows: RwLock<HashMap<(String, String), TimeEntryRow>>,
    watermark: RwLock<HashMap<String, String>>,
    is_offline: bool,
}
impl InMemoryProjections {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle_offline(&mut self) {
        self.is_offline = !self.is_offline;
    }
}

#[async_trait::async_trait]
impl TimeEntryProjectionRepository for InMemoryProjections {
    async fn upsert(&self, row: TimeEntryRow) -> anyhow::Result<()> {
        if self.is_offline {
            return Err(anyhow::anyhow!("Projections repository offline"));
        }

        let mut guard = self.rows.write().await;
        guard.insert((row.user_id.clone(), row.time_entry_id.clone()), row);
        Ok(())
    }
}
#[async_trait::async_trait]
impl WatermarkRepository for InMemoryProjections {
    async fn get(&self, name: &str) -> anyhow::Result<Option<String>> {
        if self.is_offline {
            return Err(anyhow::anyhow!("Watermark repository offline"));
        }

        Ok(self.watermark.read().await.get(name).cloned())
    }
    async fn set(&self, name: &str, last: &str) -> anyhow::Result<()> {
        if self.is_offline {
            return Err(anyhow::anyhow!("Watermark repository offline"));
        }

        self.watermark
            .write()
            .await
            .insert(name.to_string(), last.to_string());
        Ok(())
    }
}

#[async_trait::async_trait]
impl TimeEntryQueries for InMemoryProjections {
    async fn list_by_user_id(
        &self,
        user_id: &str,
        offset: u64,
        limit: u64,
        sort_by_start_time_desc: bool,
    ) -> anyhow::Result<Vec<TimeEntryView>> {
        let guard = self.rows.read().await;

        let mut items: Vec<TimeEntryRow> = guard
            .iter()
            .filter(|((uid, _), _)| uid == user_id)
            .map(|(_, row)| row.clone())
            .collect();

        items.sort_by_key(|r| r.start_time);
        if sort_by_start_time_desc {
            items.reverse();
        }

        let start = offset as usize;
        let end = start.saturating_add(limit as usize).min(items.len());
        if start >= items.len() {
            return Ok(Vec::new());
        }
        Ok(items[start..end]
            .iter()
            .cloned()
            .map(TimeEntryView::from)
            .collect())
    }
}

#[cfg(test)]
pub mod time_entry_in_memory_projections_tests {
    use super::*;
    use crate::core::time_entry::event::v1::time_entry_registered::TimeEntryRegisteredV1;
    use crate::tests::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;
    use rstest::{fixture, rstest};

    #[fixture]
    fn before_each() -> (TimeEntryRegisteredV1, TimeEntryRow, InMemoryProjections) {
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
        (event, row, repository)
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_add_the_time_entry_row_to_the_repository(
        before_each: (TimeEntryRegisteredV1, TimeEntryRow, InMemoryProjections),
    ) {
        let (event, row, repository) = before_each;
        repository
            .upsert(row.clone())
            .await
            .expect("InMemoryProjections > upsert failed");

        assert_eq!(repository.rows.read().await.len(), 1);
        assert_eq!(
            repository
                .rows
                .read()
                .await
                .get(&(event.user_id.clone(), event.time_entry_id.clone()))
                .unwrap(),
            &row
        );
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_set_the_watermark_and_confirm_its_set(
        before_each: (TimeEntryRegisteredV1, TimeEntryRow, InMemoryProjections),
    ) {
        let (_, _, repository) = before_each;
        repository
            .set("projector-name", "event-id")
            .await
            .expect("InMemoryProjections > set failed");
        assert_eq!(
            repository.get("projector-name").await.unwrap(),
            Some(String::from("event-id"))
        );
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_if_the_projections_repository_is_offline(
        before_each: (TimeEntryRegisteredV1, TimeEntryRow, InMemoryProjections),
    ) {
        let (_, row, mut repository) = before_each;
        repository.toggle_offline();
        let result = repository.upsert(row).await;
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
    async fn it_should_fail_to_set_if_the_watermark_repository_is_offline(
        before_each: (TimeEntryRegisteredV1, TimeEntryRow, InMemoryProjections),
    ) {
        let (_, _, mut repository) = before_each;
        repository.toggle_offline();
        let result = repository.set("projector-name", "event-id").await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Watermark repository offline")
        );
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_to_get_if_the_watermark_repository_is_offline(
        before_each: (TimeEntryRegisteredV1, TimeEntryRow, InMemoryProjections),
    ) {
        let (_, _, mut repository) = before_each;
        repository.toggle_offline();
        let result = repository.get("projector-name").await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Watermark repository offline")
        );
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_list_all_time_entries_stored(
        before_each: (TimeEntryRegisteredV1, TimeEntryRow, InMemoryProjections),
    ) {
        let (event, row, repository) = before_each;
        repository
            .upsert(row.clone())
            .await
            .expect("InMemoryProjections > upsert failed");

        let time_entry_list = repository
            .list_by_user_id(&event.user_id, 0, 10, false)
            .await
            .unwrap();
        let view = row.into();
        assert_eq!(time_entry_list.len(), 1);
        assert_eq!(time_entry_list[0], view);
    }
}
