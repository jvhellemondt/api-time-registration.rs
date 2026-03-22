use crate::modules::tags::core::events::TagEvent;
use crate::modules::tags::core::evolve::evolve;
use crate::modules::tags::core::state::TagState;
use crate::modules::tags::use_cases::set_tag_color::command::SetTagColor;
use crate::modules::tags::use_cases::set_tag_color::decide::decide_set_color;
use crate::modules::tags::use_cases::set_tag_color::decision::{DecideError, Decision};
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
pub struct SetTagColorHandler<TEventStore>
where
    TEventStore: EventStore<TagEvent> + Send + Sync + 'static,
{
    event_store: TEventStore,
}

impl<TEventStore> SetTagColorHandler<TEventStore>
where
    TEventStore: EventStore<TagEvent> + Send + Sync + 'static,
{
    pub fn new(event_store: TEventStore) -> Self {
        Self { event_store }
    }

    pub async fn handle(
        &self,
        stream_id: &str,
        command: SetTagColor,
    ) -> Result<(), ApplicationError> {
        let stream = self
            .event_store
            .load(stream_id)
            .await
            .map_err(ApplicationError::VersionConflict)?;

        let state = stream.events.iter().cloned().fold(TagState::None, evolve);

        match decide_set_color(&state, command) {
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
mod set_tag_color_handler_tests {
    use super::*;
    use crate::modules::tags::use_cases::create_tag::command::CreateTag;
    use crate::modules::tags::use_cases::create_tag::handler::CreateTagHandler;
    use crate::shared::infrastructure::event_store::EventStoreError;
    use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
    use rstest::{fixture, rstest};

    const STREAM_ID: &str = "Tag-t1";

    async fn create_tag(event_store: InMemoryEventStore<TagEvent>) {
        CreateTagHandler::new(event_store)
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
    fn cmd() -> SetTagColor {
        SetTagColor {
            tag_id: "t1".to_string(),
            tenant_id: "ten1".to_string(),
            color: "#BAE1FF".to_string(),
            set_at: 2000,
            set_by: "u1".to_string(),
        }
    }

    #[rstest]
    #[tokio::test]
    async fn handle_set_color_appends_event(cmd: SetTagColor) {
        let event_store = InMemoryEventStore::<TagEvent>::new();
        create_tag(event_store.clone()).await;
        let handler = SetTagColorHandler::new(event_store.clone());
        handler.handle(STREAM_ID, cmd).await.expect("handle failed");
        let stream = event_store.load(STREAM_ID).await.unwrap();
        assert_eq!(stream.events.len(), 2);
    }

    #[rstest]
    #[tokio::test]
    async fn handle_set_color_fails_if_tag_not_found(cmd: SetTagColor) {
        let handler = SetTagColorHandler::new(InMemoryEventStore::<TagEvent>::new());
        let result = handler.handle(STREAM_ID, cmd).await;
        assert!(matches!(
            result,
            Err(ApplicationError::Domain(DecideError::TagNotFound))
        ));
    }

    #[rstest]
    #[tokio::test]
    async fn handle_set_color_fails_if_tag_deleted(cmd: SetTagColor) {
        use crate::modules::tags::use_cases::delete_tag::command::DeleteTag;
        use crate::modules::tags::use_cases::delete_tag::handler::DeleteTagHandler;
        let event_store = InMemoryEventStore::<TagEvent>::new();
        create_tag(event_store.clone()).await;
        DeleteTagHandler::new(event_store.clone())
            .handle(
                STREAM_ID,
                DeleteTag {
                    tag_id: "t1".to_string(),
                    tenant_id: "ten1".to_string(),
                    deleted_at: 2000,
                    deleted_by: "u1".to_string(),
                },
            )
            .await
            .unwrap();
        let result = SetTagColorHandler::new(event_store)
            .handle(STREAM_ID, cmd)
            .await;
        assert!(matches!(
            result,
            Err(ApplicationError::Domain(DecideError::TagDeleted))
        ));
    }

    #[rstest]
    #[tokio::test]
    async fn handle_set_color_fails_if_event_store_is_offline(cmd: SetTagColor) {
        let event_store = InMemoryEventStore::<TagEvent>::new();
        event_store.toggle_offline();
        let result = SetTagColorHandler::new(event_store)
            .handle(STREAM_ID, cmd)
            .await;
        assert!(matches!(
            result,
            Err(ApplicationError::VersionConflict(EventStoreError::Backend(
                _
            )))
        ));
    }

    #[rstest]
    #[tokio::test]
    async fn handle_set_color_fails_on_version_conflict(cmd: SetTagColor) {
        use tokio::join;
        let event_store = InMemoryEventStore::<TagEvent>::new();
        create_tag(event_store.clone()).await;
        event_store.set_delay_append_ms(10);
        let h1 = SetTagColorHandler::new(event_store.clone());
        let h2 = SetTagColorHandler::new(event_store);
        let (r1, r2) = join!(h1.handle(STREAM_ID, cmd.clone()), h2.handle(STREAM_ID, cmd));
        assert!(r1.is_ok() ^ r2.is_ok(), "exactly one should succeed");
        let err = r1.err().or(r2.err()).unwrap();
        assert!(matches!(
            err,
            ApplicationError::VersionConflict(EventStoreError::VersionMismatch { .. })
        ));
    }
}
