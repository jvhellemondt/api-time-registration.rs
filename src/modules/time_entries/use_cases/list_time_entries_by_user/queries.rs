use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::{
    ListTimeEntriesState, TimeEntryView,
};
use crate::shared::infrastructure::projection_store::ProjectionStore;

#[derive(Clone)]
pub struct ListTimeEntriesQueryHandler<TStore>
where
    TStore: ProjectionStore<ListTimeEntriesState> + Send + Sync + 'static,
{
    store: TStore,
}

impl<TStore> ListTimeEntriesQueryHandler<TStore>
where
    TStore: ProjectionStore<ListTimeEntriesState> + Send + Sync + 'static,
{
    pub fn new(store: TStore) -> Self {
        Self { store }
    }
}

impl<TStore> ListTimeEntriesQueryHandler<TStore>
where
    TStore: ProjectionStore<ListTimeEntriesState> + Send + Sync + 'static,
{
    pub async fn list_by_user_id(
        &self,
        user_id: &str,
        offset: u64,
        limit: u64,
        sort_desc: bool,
    ) -> anyhow::Result<Vec<TimeEntryView>> {
        let state = self.store.state().await?.unwrap_or_default();
        let mut items: Vec<_> = state
            .rows
            .values()
            .filter(|row| row.user_id == user_id)
            .cloned()
            .collect();

        items.sort_by_key(|r| r.started_at.unwrap_or(0));
        if sort_desc {
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
mod list_time_entries_query_handler_tests {
    use super::*;
    use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::{
        TimeEntryRow, TimeEntryStatus,
    };
    use crate::shared::infrastructure::projection_store::in_memory::InMemoryProjectionStore;
    use rstest::rstest;

    fn make_row(user_id: &str, te_id: &str, started_at: Option<i64>) -> TimeEntryRow {
        TimeEntryRow {
            time_entry_id: te_id.to_string(),
            user_id: user_id.to_string(),
            started_at,
            ended_at: started_at.map(|s| s + 1000),
            tag_ids: vec![],
            status: if started_at.is_some() {
                TimeEntryStatus::Registered
            } else {
                TimeEntryStatus::Draft
            },
            created_at: 0,
            created_by: "sys".to_string(),
            updated_at: 0,
            updated_by: "sys".to_string(),
            deleted_at: None,
            last_event_id: None,
        }
    }

    async fn store_with_rows(
        rows: Vec<TimeEntryRow>,
    ) -> InMemoryProjectionStore<ListTimeEntriesState> {
        let store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        let mut state = ListTimeEntriesState::default();
        for row in rows {
            state.rows.insert(row.time_entry_id.clone(), row);
        }
        store.save(state, 1).await.unwrap();
        store
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_return_empty_list_when_no_entries() {
        let store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        let handler = ListTimeEntriesQueryHandler::new(store);
        let result = handler.list_by_user_id("u1", 0, 10, true).await.unwrap();
        assert!(result.is_empty());
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_filter_by_user_id() {
        let rows = vec![
            make_row("u1", "te1", Some(1000)),
            make_row("u2", "te2", Some(2000)),
        ];
        let store = store_with_rows(rows).await;
        let handler = ListTimeEntriesQueryHandler::new(store);
        let result = handler.list_by_user_id("u1", 0, 10, false).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].time_entry_id, "te1");
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_sort_descending_by_started_at() {
        let rows = vec![
            make_row("u1", "te1", Some(1000)),
            make_row("u1", "te2", Some(3000)),
            make_row("u1", "te3", Some(2000)),
        ];
        let store = store_with_rows(rows).await;
        let handler = ListTimeEntriesQueryHandler::new(store);
        let result = handler.list_by_user_id("u1", 0, 10, true).await.unwrap();
        assert_eq!(result[0].started_at, Some(3000));
        assert_eq!(result[1].started_at, Some(2000));
        assert_eq!(result[2].started_at, Some(1000));
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_sort_ascending_by_started_at() {
        let rows = vec![
            make_row("u1", "te1", Some(3000)),
            make_row("u1", "te2", Some(1000)),
        ];
        let store = store_with_rows(rows).await;
        let handler = ListTimeEntriesQueryHandler::new(store);
        let result = handler.list_by_user_id("u1", 0, 10, false).await.unwrap();
        assert_eq!(result[0].started_at, Some(1000));
        assert_eq!(result[1].started_at, Some(3000));
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_include_draft_entries_with_no_started_at() {
        let rows = vec![
            make_row("u1", "te1", Some(1000)),
            make_row("u1", "te-draft", None),
        ];
        let store = store_with_rows(rows).await;
        let handler = ListTimeEntriesQueryHandler::new(store);
        let result = handler.list_by_user_id("u1", 0, 10, false).await.unwrap();
        assert_eq!(result.len(), 2);
        // Draft (no started_at → unwrap_or(0)) sorts before registered in ascending
        assert_eq!(result[0].time_entry_id, "te-draft");
        assert_eq!(result[1].time_entry_id, "te1");
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_apply_offset_and_limit() {
        let rows = vec![
            make_row("u1", "te1", Some(1000)),
            make_row("u1", "te2", Some(2000)),
            make_row("u1", "te3", Some(3000)),
        ];
        let store = store_with_rows(rows).await;
        let handler = ListTimeEntriesQueryHandler::new(store);
        let result = handler.list_by_user_id("u1", 1, 1, false).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].started_at, Some(2000));
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_return_empty_when_offset_exceeds_total() {
        let rows = vec![make_row("u1", "te1", Some(1000))];
        let store = store_with_rows(rows).await;
        let handler = ListTimeEntriesQueryHandler::new(store);
        let result = handler.list_by_user_id("u1", 10, 5, false).await.unwrap();
        assert!(result.is_empty());
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_propagate_store_error() {
        let mut store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        store.toggle_offline();
        let handler = ListTimeEntriesQueryHandler::new(store);
        let result = handler.list_by_user_id("u1", 0, 10, false).await;
        assert!(result.is_err());
    }
}
