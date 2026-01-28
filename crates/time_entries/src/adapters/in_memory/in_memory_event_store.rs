// In memory implementation of the EventStore port.
//
// Purpose
// - Support command handler tests and local development without a database.
//
// Responsibilities
// - Store events per stream in memory.
// - Enforce optimistic concurrency by checking the expected version.

use crate::core::ports::{EventStore, EventStoreError, LoadedStream};
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct InMemoryEventStore<Event: Clone + Send + Sync + 'static> {
    inner: RwLock<HashMap<String, Vec<Event>>>,
}
impl<Event: Clone + Send + Sync + 'static> InMemoryEventStore<Event> {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}
#[async_trait::async_trait]
impl<Event> EventStore<Event> for InMemoryEventStore<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    async fn load(&self, id: &str) -> Result<LoadedStream<Event>, EventStoreError> {
        let guard = self.inner.read().await;
        let events = guard.get(id).cloned().unwrap_or_default();
        Ok(LoadedStream {
            events,
            version: guard.get(id).map(|v| v.len()).unwrap_or(0) as i64,
        })
    }
    async fn append(&self, stream_id: &str, expected_version: i64, new_events: &[Event]) -> Result<(), EventStoreError> {
        let mut g = self.inner.write().await;
        let entry = g.entry(stream_id.to_string()).or_default();
        let actual = entry.len() as i64;
        if actual != expected_version {
            return Err(EventStoreError::VersionMismatch { expected: expected_version, actual });
        }
        entry.extend_from_slice(new_events);
        Ok(())
    }
}

#[cfg(test)]
mod time_entry_in_memory_event_store_tests {
    use super::*;
    use rstest::rstest;
    use crate::test_fixtures::DomainEvent;

    #[rstest]
    #[tokio::test]
    async fn it_should_append_and_load_an_event() {
        let store = InMemoryEventStore::<DomainEvent>::new();
        let event = DomainEvent { event_type: "test" };
        store
            .append("1", 0, &vec![event])
            .await
            .expect("expected to append to the event_store");
        let stream = store
            .load("1")
            .await
            .expect("expected to load from the event_store");
        assert_eq!(stream.version, 1);
        let stream_events = stream.events;
        assert_eq!(stream_events.len(), 1);
        assert_eq!(stream_events.get(0).unwrap().event_type, "test");
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_append_and_load_multiple_events() {
        let store = InMemoryEventStore::<DomainEvent>::new();
        let events = vec![
            DomainEvent {
                event_type: "test_1",
            },
            DomainEvent {
                event_type: "test_2",
            },
            DomainEvent {
                event_type: "test_3",
            },
        ];
        store
            .append("1", 0, &events)
            .await
            .expect("expected to append to the event_store");
        let stream = store
            .load("1")
            .await
            .expect("expected to load from the event_store");
        assert_eq!(stream.version, 3);
        let stream_events = stream.events;
        assert_eq!(stream_events.len(), 3);
        assert_eq!(stream_events.get(0).unwrap().event_type, "test_1");
        assert_eq!(stream_events.get(1).unwrap().event_type, "test_2");
        assert_eq!(stream_events.get(2).unwrap().event_type, "test_3");
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_to_append_if_the_wrong_version_is_expected() {
        let store = InMemoryEventStore::<DomainEvent>::new();
        let event = DomainEvent { event_type: "test" };
        let store_result= store
            .append("1", 1, &vec![event])
            .await;
        assert!(store_result.is_err());
        match store_result {
            Err(EventStoreError::VersionMismatch { expected, actual }) => {
                assert_eq!(actual, 0);
                assert_eq!(expected, 1);
            },
            _ => panic!("expected VersionMismatch error"),
        }
    }
}
