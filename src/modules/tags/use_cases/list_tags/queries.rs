use crate::modules::tags::use_cases::list_tags::projection::{ListTagsState, TagView};
use crate::shared::infrastructure::projection_store::ProjectionStore;

#[derive(Clone)]
pub struct ListTagsQueryHandler<TStore>
where
    TStore: ProjectionStore<ListTagsState> + Send + Sync + 'static,
{
    store: TStore,
}

impl<TStore> ListTagsQueryHandler<TStore>
where
    TStore: ProjectionStore<ListTagsState> + Send + Sync + 'static,
{
    pub fn new(store: TStore) -> Self {
        Self { store }
    }

    pub async fn list_all(&self) -> anyhow::Result<Vec<TagView>> {
        let state = self.store.state().await?.unwrap_or_default();
        let mut items: Vec<_> = state.rows.into_values().map(TagView::from).collect();
        items.sort_by(|a, b| a.tag_id.cmp(&b.tag_id));
        Ok(items)
    }
}

#[cfg(test)]
mod list_tags_query_handler_tests {
    use super::*;
    use crate::modules::tags::use_cases::list_tags::projection::{ListTagsState, TagRow};
    use crate::shared::infrastructure::projection_store::in_memory::InMemoryProjectionStore;
    use rstest::rstest;

    fn make_row(tag_id: &str, name: &str) -> TagRow {
        TagRow {
            tag_id: tag_id.to_string(),
            tenant_id: "ten1".to_string(),
            name: name.to_string(),
            color: "#FFB3BA".to_string(),
            description: None,
            last_event_id: None,
        }
    }

    async fn store_with_rows(rows: Vec<TagRow>) -> InMemoryProjectionStore<ListTagsState> {
        let store = InMemoryProjectionStore::<ListTagsState>::new();
        let mut state = ListTagsState::default();
        for row in rows {
            state.rows.insert(row.tag_id.clone(), row);
        }
        store.save(state, 1).await.unwrap();
        store
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_return_empty_list_when_no_tags() {
        let store = InMemoryProjectionStore::<ListTagsState>::new();
        let handler = ListTagsQueryHandler::new(store);
        let result = handler.list_all().await.unwrap();
        assert!(result.is_empty());
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_return_all_tags_sorted_by_tag_id() {
        let rows = vec![make_row("t2", "Beta"), make_row("t1", "Alpha")];
        let store = store_with_rows(rows).await;
        let handler = ListTagsQueryHandler::new(store);
        let result = handler.list_all().await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].tag_id, "t1");
        assert_eq!(result[1].tag_id, "t2");
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_include_description_in_view() {
        let mut row = make_row("t1", "Work");
        row.description = Some("Client work".to_string());
        let store = store_with_rows(vec![row]).await;
        let handler = ListTagsQueryHandler::new(store);
        let result = handler.list_all().await.unwrap();
        assert_eq!(result[0].description, Some("Client work".to_string()));
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_propagate_store_error() {
        let mut store = InMemoryProjectionStore::<ListTagsState>::new();
        store.toggle_offline();
        let handler = ListTagsQueryHandler::new(store);
        let result = handler.list_all().await;
        assert!(result.is_err());
    }
}
