use crate::modules::time_entries::adapters::outbound::intent_outbox::dispatch_intents;
use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::evolve::evolve;
use crate::modules::time_entries::core::state::TimeEntryState;
use crate::modules::time_entries::use_cases::set_ended_at::command::SetEndedAt;
use crate::modules::time_entries::use_cases::set_ended_at::decide::decide_set_ended_at;
use crate::modules::time_entries::use_cases::set_ended_at::decision::{DecideError, Decision};
use crate::shared::infrastructure::event_store::{EventStore, EventStoreError};
use crate::shared::infrastructure::intent_outbox::{DomainOutbox, OutboxError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    VersionConflict(#[from] EventStoreError),

    #[error(transparent)]
    Outbox(#[from] OutboxError),

    #[error("domain rejected: {0}")]
    Domain(DecideError),

    #[error("unexpected: {0}")]
    Unexpected(String),
}

#[derive(Debug, Clone)]
pub struct SetEndedAtHandler<TEventStore, TOutbox>
where
    TEventStore: EventStore<TimeEntryEvent> + Send + Sync + 'static,
    TOutbox: DomainOutbox + Send + Sync + 'static,
{
    topic: String,
    event_store: TEventStore,
    outbox: TOutbox,
}

impl<TEventStore, TOutbox> SetEndedAtHandler<TEventStore, TOutbox>
where
    TEventStore: EventStore<TimeEntryEvent> + Send + Sync + 'static,
    TOutbox: DomainOutbox + Send + Sync + 'static,
{
    pub fn new(topic: impl Into<String>, event_store: TEventStore, outbox: TOutbox) -> Self {
        Self {
            topic: topic.into(),
            event_store,
            outbox,
        }
    }

    pub async fn handle(
        &self,
        stream_id: &str,
        command: SetEndedAt,
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

        match decide_set_ended_at(&state, command) {
            Decision::Accepted { events, intents } => {
                let events_len = events.len();
                self.event_store
                    .append(stream_id, stream.version, &events)
                    .await
                    .map_err(ApplicationError::VersionConflict)?;
                dispatch_intents(
                    &self.outbox,
                    stream_id,
                    stream.version,
                    events_len,
                    &self.topic,
                    intents,
                )
                .await
                .map_err(ApplicationError::Outbox)?;
                Ok(())
            }
            Decision::Rejected { reason } => Err(ApplicationError::Domain(reason)),
        }
    }
}

#[cfg(test)]
mod set_ended_at_handler_tests {
    use crate::modules::time_entries::core::events::TimeEntryEvent;
    use crate::modules::time_entries::use_cases::set_ended_at::decision::DecideError;
    use crate::modules::time_entries::use_cases::set_ended_at::handler::{
        ApplicationError, SetEndedAtHandler,
    };
    use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
    use crate::shared::infrastructure::event_store::{EventStore, EventStoreError};
    use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
    use crate::shared::infrastructure::intent_outbox::{DomainOutbox, OutboxError, OutboxRow};
    use crate::tests::fixtures::commands::set_ended_at::SetEndedAtBuilder;
    use rstest::{fixture, rstest};
    use tokio::join;

    const TOPIC: &str = "time-entries";

    type BeforeEachReturn = (
        &'static str,
        InMemoryEventStore<TimeEntryEvent>,
        InMemoryDomainOutbox,
    );

    #[fixture]
    fn before_each() -> BeforeEachReturn {
        let stream_id = "TimeEntry-te-fixed-0001";
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new();
        let outbox = InMemoryDomainOutbox::new();
        (stream_id, event_store, outbox)
    }

    #[rstest]
    #[tokio::test]
    async fn handle_set_ended_at_creates_draft_on_new_stream(before_each: BeforeEachReturn) {
        let (stream_id, event_store, outbox) = before_each;
        let handler = SetEndedAtHandler::new(TOPIC, event_store.clone(), outbox);
        handler
            .handle(stream_id, SetEndedAtBuilder::new().build())
            .await
            .expect("handle failed");
        let stream = event_store.load(stream_id).await.expect("load failed");
        // Initiated + EndSet = 2 events
        assert_eq!(stream.events.len(), 2);
    }

    #[rstest]
    #[tokio::test]
    async fn handle_set_ended_at_on_existing_draft_emits_end_set(before_each: BeforeEachReturn) {
        let (stream_id, event_store, outbox) = before_each;
        let handler = SetEndedAtHandler::new(TOPIC, event_store.clone(), outbox);
        // First call creates draft
        handler
            .handle(stream_id, SetEndedAtBuilder::new().build())
            .await
            .unwrap();
        // Second call updates ended_at
        handler
            .handle(
                stream_id,
                SetEndedAtBuilder::new().ended_at(1_700_000_400_000).build(),
            )
            .await
            .unwrap();
        let stream = event_store.load(stream_id).await.unwrap();
        // Initiated, EndSet, EndSet = 3 events
        assert_eq!(stream.events.len(), 3);
    }

    #[rstest]
    #[tokio::test]
    async fn handle_set_ended_at_rejects_invalid_interval(before_each: BeforeEachReturn) {
        let (stream_id, event_store, outbox) = before_each;
        let handler = SetEndedAtHandler::new(TOPIC, event_store.clone(), outbox.clone());
        // Create a draft with started_at via set_started_at first
        use crate::modules::time_entries::use_cases::set_started_at::handler::SetStartedAtHandler;
        use crate::tests::fixtures::commands::set_started_at::SetStartedAtBuilder;
        SetStartedAtHandler::new(TOPIC, event_store.clone(), outbox)
            .handle(
                stream_id,
                SetStartedAtBuilder::new().started_at(5_000).build(),
            )
            .await
            .unwrap();
        // Now set ended_at <= started_at
        let result = handler
            .handle(stream_id, SetEndedAtBuilder::new().ended_at(3_000).build())
            .await;
        assert!(matches!(
            result,
            Err(ApplicationError::Domain(DecideError::InvalidInterval))
        ));
    }

    #[rstest]
    #[tokio::test]
    async fn handle_set_ended_at_fails_if_event_store_is_offline(before_each: BeforeEachReturn) {
        let (stream_id, event_store, outbox) = before_each;
        event_store.toggle_offline();
        let handler = SetEndedAtHandler::new(TOPIC, event_store, outbox);
        let result = handler
            .handle(stream_id, SetEndedAtBuilder::new().build())
            .await;
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
    async fn handle_set_ended_at_fails_on_version_conflict(before_each: BeforeEachReturn) {
        let (stream_id, event_store, outbox) = before_each;
        event_store.set_delay_append_ms(10);
        let es = event_store;
        let ob = outbox;
        let handler1 = SetEndedAtHandler::new(TOPIC, es.clone(), ob.clone());
        let handler2 = SetEndedAtHandler::new(TOPIC, es, ob);
        let (result1, result2) = join!(
            handler1.handle(stream_id, SetEndedAtBuilder::new().build()),
            handler2.handle(stream_id, SetEndedAtBuilder::new().build())
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
                assert_eq!(actual, 2);
            }
            e => panic!("unexpected error: {e:?}"),
        }
    }

    #[rstest]
    #[tokio::test]
    async fn handle_set_ended_at_fails_if_outbox_has_duplicate(before_each: BeforeEachReturn) {
        let (stream_id, event_store, outbox) = before_each;
        // Create a draft with started_at first (Initiated v1, StartSet v2)
        use crate::modules::time_entries::use_cases::set_started_at::handler::SetStartedAtHandler;
        use crate::tests::fixtures::commands::set_started_at::SetStartedAtBuilder;
        SetStartedAtHandler::new(TOPIC, event_store.clone(), outbox.clone())
            .handle(
                stream_id,
                SetStartedAtBuilder::new()
                    .started_at(1_700_000_000_000)
                    .build(),
            )
            .await
            .unwrap();
        // Pre-seed outbox at version 4 (stream v2, 2 more events appended → v4)
        outbox
            .enqueue(OutboxRow {
                topic: TOPIC.to_string(),
                event_type: "TimeEntryRegistered".to_string(),
                event_version: 1,
                stream_id: stream_id.to_string(),
                stream_version: 4,
                occurred_at: 0,
                payload: serde_json::json!({}),
            })
            .await
            .unwrap();
        let handler = SetEndedAtHandler::new(TOPIC, event_store, outbox);
        let result = handler
            .handle(
                stream_id,
                SetEndedAtBuilder::new().ended_at(1_700_000_360_000).build(),
            )
            .await;
        assert!(matches!(
            result,
            Err(ApplicationError::Outbox(OutboxError::Duplicate { .. }))
        ));
    }
}
