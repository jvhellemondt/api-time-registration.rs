use crate::modules::tags::core::events::TagEvent;
use crate::modules::tags::core::evolve::evolve;
use crate::modules::tags::core::state::TagState;
use crate::modules::tags::use_cases::create_tag::command::CreateTag;
use crate::modules::tags::use_cases::create_tag::decide::decide_create;
use crate::modules::tags::use_cases::create_tag::decision::{DecideError, Decision};
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
pub struct CreateTagHandler<TEventStore>
where
    TEventStore: EventStore<TagEvent> + Send + Sync + 'static,
{
    event_store: TEventStore,
}

impl<TEventStore> CreateTagHandler<TEventStore>
where
    TEventStore: EventStore<TagEvent> + Send + Sync + 'static,
{
    pub fn new(event_store: TEventStore) -> Self {
        Self { event_store }
    }

    pub async fn handle(
        &self,
        stream_id: &str,
        command: CreateTag,
    ) -> Result<(), ApplicationError> {
        let stream = self
            .event_store
            .load(stream_id)
            .await
            .map_err(ApplicationError::VersionConflict)?;

        let state = stream.events.iter().cloned().fold(TagState::None, evolve);

        match decide_create(&state, command) {
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
mod create_tag_handler_tests {
    use super::*;
    use crate::shared::infrastructure::event_store::EventStoreError;
    use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
    use rstest::{fixture, rstest};
    use tokio::join;

    type Setup = (&'static str, CreateTag, InMemoryEventStore<TagEvent>);

    #[fixture]
    fn setup() -> Setup {
        let stream_id = "Tag-t1";
        let event_store = InMemoryEventStore::<TagEvent>::new();
        let command = CreateTag {
            tag_id: "t1".to_string(),
            tenant_id: "ten1".to_string(),
            name: "Work".to_string(),
            color: "#FFB3BA".to_string(),
            description: None,
            created_at: 1000,
            created_by: "u1".to_string(),
        };
        (stream_id, command, event_store)
    }

    #[rstest]
    #[tokio::test]
    async fn handle_create_appends_event(setup: Setup) {
        let (stream_id, command, event_store) = setup;
        let handler = CreateTagHandler::new(event_store.clone());
        handler
            .handle(stream_id, command)
            .await
            .expect("handle failed");
        let stream = event_store.load(stream_id).await.unwrap();
        assert_eq!(stream.events.len(), 1);
    }

    #[rstest]
    #[tokio::test]
    async fn handle_create_fails_if_tag_already_exists(setup: Setup) {
        let (stream_id, command, event_store) = setup;
        let handler = CreateTagHandler::new(event_store);
        handler.handle(stream_id, command.clone()).await.unwrap();
        let result = handler.handle(stream_id, command).await;
        assert!(matches!(
            result,
            Err(ApplicationError::Domain(DecideError::TagAlreadyExists))
        ));
    }

    #[rstest]
    #[tokio::test]
    async fn handle_create_fails_if_event_store_is_offline(setup: Setup) {
        let (stream_id, command, event_store) = setup;
        event_store.toggle_offline();
        let handler = CreateTagHandler::new(event_store);
        let result = handler.handle(stream_id, command).await;
        assert!(matches!(
            result,
            Err(ApplicationError::VersionConflict(EventStoreError::Backend(
                _
            )))
        ));
    }

    #[rstest]
    #[tokio::test]
    async fn handle_create_fails_on_version_conflict(setup: Setup) {
        let (stream_id, command, event_store) = setup;
        event_store.set_delay_append_ms(10);
        let h1 = CreateTagHandler::new(event_store.clone());
        let h2 = CreateTagHandler::new(event_store);
        let (r1, r2) = join!(
            h1.handle(stream_id, command.clone()),
            h2.handle(stream_id, command)
        );
        assert!(r1.is_ok() ^ r2.is_ok(), "exactly one should succeed");
        let err = r1.err().or(r2.err()).unwrap();
        assert!(matches!(
            err,
            ApplicationError::VersionConflict(EventStoreError::VersionMismatch { .. })
        ));
    }
}
