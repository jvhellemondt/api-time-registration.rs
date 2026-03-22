use crate::modules::tags::core::events::TagEvent;
use crate::modules::tags::core::evolve::evolve;
use crate::modules::tags::core::state::TagState;
use crate::modules::tags::use_cases::delete_tag::command::DeleteTag;
use crate::modules::tags::use_cases::delete_tag::decide::decide_delete;
use crate::modules::tags::use_cases::delete_tag::decision::{DecideError, Decision};
use crate::shared::infrastructure::event_store::{EventStore, EventStoreError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    VersionConflict(#[from] EventStoreError),

    #[error("domain error: {0}")]
    Domain(DecideError),
}

#[derive(Debug, Clone)]
pub struct DeleteTagHandler<TEventStore>
where
    TEventStore: EventStore<TagEvent> + Send + Sync + 'static,
{
    event_store: TEventStore,
}

impl<TEventStore> DeleteTagHandler<TEventStore>
where
    TEventStore: EventStore<TagEvent> + Send + Sync + 'static,
{
    pub fn new(event_store: TEventStore) -> Self {
        Self { event_store }
    }

    pub async fn handle(
        &self,
        stream_id: &str,
        command: DeleteTag,
    ) -> Result<(), ApplicationError> {
        let stream = self
            .event_store
            .load(stream_id)
            .await
            .map_err(ApplicationError::VersionConflict)?;

        let state = stream.events.iter().cloned().fold(TagState::None, evolve);

        match decide_delete(&state, command) {
            Decision::Accepted { events } => {
                self.event_store
                    .append(stream_id, stream.version, &events)
                    .await
                    .map_err(ApplicationError::VersionConflict)?;
                Ok(())
            }
            Decision::Rejected { reason } => Err(ApplicationError::Domain(reason)),
        }
    }
}

#[cfg(test)]
mod delete_tag_handler_tests {
    use super::*;
    use crate::modules::tags::use_cases::create_tag::command::CreateTag;
    use crate::modules::tags::use_cases::create_tag::handler::CreateTagHandler;
    use crate::shared::infrastructure::event_store::EventStoreError;
    use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
    use rstest::{fixture, rstest};

    const STREAM_ID: &str = "Tag-t1";

    async fn create_tag(event_store: InMemoryEventStore<TagEvent>) {
        let handler = CreateTagHandler::new(event_store);
        handler
            .handle(
                STREAM_ID,
                CreateTag {
                    tag_id: "t1".to_string(),
                    tenant_id: "ten1".to_string(),
                    name: "Work".to_string(),
                    color: "#FFB3BA".to_string(),
                    description: None,
                    created_at: 1000,
                    created_by: "u1".to_string(),
                },
            )
            .await
            .unwrap();
    }

    #[fixture]
    fn delete_cmd() -> DeleteTag {
        DeleteTag {
            tag_id: "t1".to_string(),
            tenant_id: "ten1".to_string(),
            deleted_at: 2000,
            deleted_by: "u1".to_string(),
        }
    }

    #[rstest]
    #[tokio::test]
    async fn handle_delete_appends_event(delete_cmd: DeleteTag) {
        let event_store = InMemoryEventStore::<TagEvent>::new();
        create_tag(event_store.clone()).await;
        let handler = DeleteTagHandler::new(event_store.clone());
        handler
            .handle(STREAM_ID, delete_cmd)
            .await
            .expect("handle failed");
        let stream = event_store.load(STREAM_ID).await.unwrap();
        assert_eq!(stream.events.len(), 2);
    }

    #[rstest]
    #[tokio::test]
    async fn handle_delete_fails_if_tag_not_found(delete_cmd: DeleteTag) {
        let event_store = InMemoryEventStore::<TagEvent>::new();
        let handler = DeleteTagHandler::new(event_store);
        let result = handler.handle(STREAM_ID, delete_cmd).await;
        assert!(matches!(
            result,
            Err(ApplicationError::Domain(DecideError::TagNotFound))
        ));
    }

    #[rstest]
    #[tokio::test]
    async fn handle_delete_fails_if_already_deleted(delete_cmd: DeleteTag) {
        let event_store = InMemoryEventStore::<TagEvent>::new();
        create_tag(event_store.clone()).await;
        let handler = DeleteTagHandler::new(event_store);
        handler.handle(STREAM_ID, delete_cmd.clone()).await.unwrap();
        let result = handler.handle(STREAM_ID, delete_cmd).await;
        assert!(matches!(
            result,
            Err(ApplicationError::Domain(DecideError::TagAlreadyDeleted))
        ));
    }

    #[rstest]
    #[tokio::test]
    async fn handle_delete_fails_if_event_store_is_offline(delete_cmd: DeleteTag) {
        let event_store = InMemoryEventStore::<TagEvent>::new();
        event_store.toggle_offline();
        let handler = DeleteTagHandler::new(event_store);
        let result = handler.handle(STREAM_ID, delete_cmd).await;
        assert!(matches!(
            result,
            Err(ApplicationError::VersionConflict(EventStoreError::Backend(
                _
            )))
        ));
    }

    #[rstest]
    #[tokio::test]
    async fn handle_delete_fails_on_version_conflict(delete_cmd: DeleteTag) {
        use tokio::join;
        let event_store = InMemoryEventStore::<TagEvent>::new();
        create_tag(event_store.clone()).await;
        event_store.set_delay_append_ms(10);
        let h1 = DeleteTagHandler::new(event_store.clone());
        let h2 = DeleteTagHandler::new(event_store);
        let (r1, r2) = join!(
            h1.handle(STREAM_ID, delete_cmd.clone()),
            h2.handle(STREAM_ID, delete_cmd)
        );
        assert!(r1.is_ok() ^ r2.is_ok(), "exactly one should succeed");
        let err = r1.err().or(r2.err()).unwrap();
        assert!(matches!(
            err,
            ApplicationError::VersionConflict(EventStoreError::VersionMismatch { .. })
        ));
    }
}
