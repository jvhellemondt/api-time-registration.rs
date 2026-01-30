// Registration command handler orchestrates the writing flow.
//
// Responsibilities
// - Load past events from the event store and fold them into state.
// - Call the decider with the command and current time.
// - Append new events with optimistic concurrency.
// - Enqueue domain events into the domain outbox for publishing.

use crate::application::errors::ApplicationError;
use crate::core::ports::{DomainOutbox, EventStore, OutboxRow};
use crate::core::time_entry::decider::register::command::RegisterTimeEntry;
use crate::core::time_entry::decider::register::decide::decide_register;
use crate::core::time_entry::event::TimeEntryEvent;
use crate::core::time_entry::evolve::evolve;
use crate::core::time_entry::state::TimeEntryState;
use std::marker::PhantomData;

pub struct TimeEntryRegisteredCommandHandler<'a, TEventStore, TOutbox>
where
    TEventStore: EventStore<TimeEntryEvent> + Sync + 'a,
    TOutbox: DomainOutbox + Sync + 'a,
{
    topic: &'a str,
    event_store: &'a TEventStore,
    outbox: &'a TOutbox,
    _pd: PhantomData<&'a ()>,
}

impl<'a, TEventStore, TOutbox> TimeEntryRegisteredCommandHandler<'a, TEventStore, TOutbox>
where
    TEventStore: EventStore<TimeEntryEvent> + Sync + 'a,
    TOutbox: DomainOutbox + Sync + 'a,
{
    pub fn new(topic: &'a str, event_store: &'a TEventStore, outbox: &'a TOutbox) -> Self {
        Self {
            topic,
            event_store,
            outbox,
            _pd: PhantomData,
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
        let new_events = decide_register(&state, command)
            .map_err(|e| ApplicationError::Domain(e.to_string()))?;

        self.event_store
            .append(stream_id, stream.version, &new_events)
            .await
            .map_err(ApplicationError::VersionConflict)?;

        let mut stream_version = stream.version;
        for event in &new_events {
            stream_version += 1;
            match event {
                TimeEntryEvent::TimeEntryRegisteredV1(body) => {
                    let row = OutboxRow {
                        topic: self.topic.to_string(),
                        event_type: "TimeEntryRegistered".to_string(),
                        event_version: 1,
                        stream_id: stream_id.to_string(),
                        stream_version,
                        occurred_at: body.created_at,
                        payload: serde_json::to_value(body).unwrap(),
                    };
                    self.outbox
                        .enqueue(row)
                        .await
                        .map_err(ApplicationError::Outbox)?
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod time_entry_register_time_entry_tests {
    // Integration tests for the registration decider and the evolve function.
    //
    // Responsibilities when you add code
    // - Assert validation rules (end time after start time).
    // - Assert the happy path emits the expected event.
    // - Assert the evolve function produces the registered state.

    use crate::adapters::in_memory::in_memory_domain_outbox::InMemoryDomainOutbox;
    use crate::adapters::in_memory::in_memory_event_store::InMemoryEventStore;
    use crate::application::command_handlers::register_handler::TimeEntryRegisteredCommandHandler;
    use crate::application::errors::ApplicationError;
    use crate::core::ports::{DomainOutbox, EventStore, EventStoreError, OutboxError, OutboxRow};
    use crate::core::time_entry::decider::register::command::RegisterTimeEntry;
    use crate::core::time_entry::decider::register::decide::DecideError;
    use crate::core::time_entry::event::TimeEntryEvent;
    use crate::test_support::fixtures::commands::register_time_entry::RegisterTimeEntryBuilder;
    use crate::test_support::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;
    use rstest::{fixture, rstest};
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
        let handler = TimeEntryRegisteredCommandHandler::new(TOPIC, &event_store, &outbox);
        handler
            .handle(stream_id, command)
            .await
            .expect("Integration test::register_decide > handle failed");
        let stream = event_store
            .load(stream_id)
            .await
            .expect("Integration test::register_decide > event_store load failed");
        assert_eq!(stream.events.len(), 1);
    }

    #[rstest]
    #[tokio::test]
    async fn handle_register_fails_if_time_entry_exists(before_each: BeforeEachReturn) {
        let (stream_id, command, event_store, outbox) = before_each;
        let handler = TimeEntryRegisteredCommandHandler::new(TOPIC, &event_store, &outbox);
        handler
            .handle(stream_id, command.clone())
            .await
            .expect("Integration test::register_decide > handle failed");
        let handle_result = handler.handle(stream_id, command).await;
        assert!(handle_result.is_err());
        assert_eq!(
            handle_result.unwrap_err().to_string(),
            ApplicationError::Domain(DecideError::AlreadyExists.to_string()).to_string()
        );
    }

    #[rstest]
    #[tokio::test]
    async fn handle_register_fails_if_event_store_is_offline(before_each: BeforeEachReturn) {
        let (stream_id, command, mut event_store, outbox) = before_each;
        event_store.toggle_offline();
        let handler = TimeEntryRegisteredCommandHandler::new(TOPIC, &event_store, &outbox);
        let handle_result = handler.handle(stream_id, command).await;
        assert!(handle_result.is_err());
        assert_eq!(
            handle_result.unwrap_err().to_string(),
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
        let handler1 = TimeEntryRegisteredCommandHandler::new(TOPIC, &event_store, &outbox);
        let handler2 = TimeEntryRegisteredCommandHandler::new(TOPIC, &event_store, &outbox);
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
    async fn handle_register_fails_if_outbox_has_a_duplicate_entry_error(before_each: BeforeEachReturn) {
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
        outbox
            .enqueue(row)
            .await
            .expect("Integration test::register_decide > enqueue failed");

        let handler = TimeEntryRegisteredCommandHandler::new(TOPIC, &event_store, &outbox);
        let handle_result = handler.handle(stream_id, command).await;
        assert!(handle_result.is_err());
        assert!(matches!(
            handle_result,
            Err(ApplicationError::Outbox(OutboxError::Duplicate {
                stream_id: _,
                stream_version: 1,
            }))
        ));
    }
}
