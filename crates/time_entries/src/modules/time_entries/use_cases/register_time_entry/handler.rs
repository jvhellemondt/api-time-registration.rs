use crate::modules::time_entries::adapters::outbound::intent_outbox::dispatch_intents;
use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::evolve::evolve;
use crate::modules::time_entries::core::state::TimeEntryState;
use crate::modules::time_entries::use_cases::register_time_entry::command::RegisterTimeEntry;
use crate::modules::time_entries::use_cases::register_time_entry::decide::decide_register;
use crate::modules::time_entries::use_cases::register_time_entry::decision::Decision;
use crate::shared::infrastructure::event_store::{EventStore, EventStoreError};
use crate::shared::infrastructure::intent_outbox::{DomainOutbox, OutboxError};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    VersionConflict(#[from] EventStoreError),

    #[error(transparent)]
    Outbox(#[from] OutboxError),

    #[error("domain rejected: {0}")]
    Domain(String),

    #[error("unexpected: {0}")]
    Unexpected(String),
}

pub struct RegisterTimeEntryHandler<TEventStore, TOutbox>
where
    TEventStore: EventStore<TimeEntryEvent> + Send + Sync + 'static,
    TOutbox: DomainOutbox + Send + Sync + 'static,
{
    topic: String,
    event_store: Arc<TEventStore>,
    outbox: Arc<TOutbox>,
}

impl<TEventStore, TOutbox> RegisterTimeEntryHandler<TEventStore, TOutbox>
where
    TEventStore: EventStore<TimeEntryEvent> + Send + Sync + 'static,
    TOutbox: DomainOutbox + Send + Sync + 'static,
{
    pub fn new(
        topic: impl Into<String>,
        event_store: Arc<TEventStore>,
        outbox: Arc<TOutbox>,
    ) -> Self {
        Self {
            topic: topic.into(),
            event_store,
            outbox,
        }
    }

    pub async fn handle(
        &self,
        stream_id: &str,
        command: RegisterTimeEntry,
    ) -> Result<(), ApplicationError> {
        let stream = self
            .event_store
            .load(stream_id)
            .await
            .map_err(ApplicationError::VersionConflict)?;

        let state = stream
            .events
            .iter()
            .cloned()
            .fold(TimeEntryState::None, evolve);

        match decide_register(&state, command) {
            Decision::Accepted { events, intents } => {
                self.event_store
                    .append(stream_id, stream.version, &events)
                    .await
                    .map_err(ApplicationError::VersionConflict)?;
                dispatch_intents(
                    &*self.outbox,
                    stream_id,
                    stream.version,
                    &self.topic,
                    intents,
                )
                .await
                .map_err(ApplicationError::Outbox)?;
                Ok(())
            }
            Decision::Rejected { reason } => Err(ApplicationError::Domain(reason.to_string())),
        }
    }
}

#[cfg(test)]
mod time_entry_register_handler_tests {
    use crate::modules::time_entries::core::events::TimeEntryEvent;
    use crate::modules::time_entries::use_cases::register_time_entry::command::RegisterTimeEntry;
    use crate::modules::time_entries::use_cases::register_time_entry::decision::DecideError;
    use crate::modules::time_entries::use_cases::register_time_entry::handler::{
        ApplicationError, RegisterTimeEntryHandler,
    };
    use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
    use crate::shared::infrastructure::event_store::{EventStore, EventStoreError};
    use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
    use crate::shared::infrastructure::intent_outbox::{DomainOutbox, OutboxError, OutboxRow};
    use crate::tests::fixtures::commands::register_time_entry::RegisterTimeEntryBuilder;
    use crate::tests::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;
    use rstest::{fixture, rstest};
    use std::sync::Arc;
    use tokio::join;

    const TOPIC: &str = "time-entries";

    type BeforeEachReturn = (
        &'static str,
        RegisterTimeEntry,
        InMemoryEventStore<TimeEntryEvent>,
        InMemoryDomainOutbox,
    );

    #[fixture]
    fn before_each() -> BeforeEachReturn {
        let stream_id = "time-entries-0001";
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new();
        let outbox = InMemoryDomainOutbox::new();
        let command = RegisterTimeEntryBuilder::new().build();
        (stream_id, command, event_store, outbox)
    }

    #[rstest]
    #[tokio::test]
    async fn handle_register_appends_and_enqueues(before_each: BeforeEachReturn) {
        let (stream_id, command, event_store, outbox) = before_each;
        let es = Arc::new(event_store);
        let ob = Arc::new(outbox);
        let handler = RegisterTimeEntryHandler::new(TOPIC, es.clone(), ob);
        handler
            .handle(stream_id, command)
            .await
            .expect("handle failed");
        let stream = es.load(stream_id).await.expect("load failed");
        assert_eq!(stream.events.len(), 1);
    }

    #[rstest]
    #[tokio::test]
    async fn handle_register_fails_if_time_entry_exists(before_each: BeforeEachReturn) {
        let (stream_id, command, event_store, outbox) = before_each;
        let handler = RegisterTimeEntryHandler::new(TOPIC, Arc::new(event_store), Arc::new(outbox));
        handler
            .handle(stream_id, command.clone())
            .await
            .expect("first handle failed");
        let result = handler.handle(stream_id, command).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            ApplicationError::Domain(DecideError::AlreadyExists.to_string()).to_string()
        );
    }

    #[rstest]
    #[tokio::test]
    async fn handle_register_fails_if_event_store_is_offline(before_each: BeforeEachReturn) {
        let (stream_id, command, mut event_store, outbox) = before_each;
        event_store.toggle_offline();
        let handler = RegisterTimeEntryHandler::new(TOPIC, Arc::new(event_store), Arc::new(outbox));
        let result = handler.handle(stream_id, command).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            ApplicationError::VersionConflict(EventStoreError::Backend(
                "Event store offline".into()
            ))
            .to_string()
        );
    }

    #[rstest]
    #[tokio::test]
    async fn handle_register_fails_if_event_store_has_a_mismatching_version(
        before_each: BeforeEachReturn,
    ) {
        let (stream_id, command, event_store, outbox) = before_each;
        event_store.set_delay_append_ms(10);
        let es = Arc::new(event_store);
        let ob = Arc::new(outbox);
        let handler1 = RegisterTimeEntryHandler::new(TOPIC, es.clone(), ob.clone());
        let handler2 = RegisterTimeEntryHandler::new(TOPIC, es, ob);
        let (result1, result2) = join!(
            handler1.handle(stream_id, command.clone()),
            handler2.handle(stream_id, command)
        );
        assert!(
            result1.is_ok() ^ result2.is_ok(),
            "exactly one should fail with conflict"
        );
        let err = result1.err().or(result2.err()).unwrap();
        match err {
            ApplicationError::VersionConflict(EventStoreError::VersionMismatch {
                expected,
                actual,
            }) => {
                assert_eq!(expected, 0);
                assert_eq!(actual, 1);
            }
            e => panic!("unexpected error: {e:?}"),
        }
    }

    #[rstest]
    #[tokio::test]
    async fn handle_register_fails_if_outbox_has_a_duplicate_entry_error(
        before_each: BeforeEachReturn,
    ) {
        let (stream_id, command, event_store, outbox) = before_each;
        let event = make_time_entry_registered_v1_event();
        let row = OutboxRow {
            topic: TOPIC.to_string(),
            event_type: "TimeEntryRegistered".to_string(),
            event_version: 1,
            stream_id: stream_id.to_string(),
            stream_version: 1,
            occurred_at: event.created_at,
            payload: serde_json::to_value(event).unwrap(),
        };
        outbox.enqueue(row).await.expect("pre-enqueue failed");
        let handler = RegisterTimeEntryHandler::new(TOPIC, Arc::new(event_store), Arc::new(outbox));
        let result = handler.handle(stream_id, command).await;
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ApplicationError::Outbox(OutboxError::Duplicate {
                stream_id: _,
                stream_version: 1,
            }))
        ));
    }
}
