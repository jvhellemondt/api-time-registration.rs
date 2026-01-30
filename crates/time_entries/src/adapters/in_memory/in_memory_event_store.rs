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
    is_offline: bool,
}
impl<Event: Clone + Send + Sync + 'static> Default for InMemoryEventStore<Event> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Event: Clone + Send + Sync + 'static> InMemoryEventStore<Event> {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
            is_offline: false,
        }
    }

    pub fn toggle_offline(&mut self) {
        self.is_offline = !self.is_offline;
    }
}
#[async_trait::async_trait]
impl<Event> EventStore<Event> for InMemoryEventStore<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    async fn load(&self, id: &str) -> Result<LoadedStream<Event>, EventStoreError> {
        if self.is_offline {
            return Err(EventStoreError::Backend("Event store offline".to_string()))
        }

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
    use crate::test_support::fixtures::events::domain_event::DomainEvent;

    #[rstest]
    #[tokio::test]
    async fn it_should_initiate_with_new_and_default() {
        let _store_with_new: InMemoryEventStore<DomainEvent> =
            InMemoryEventStore::new();
        let _store_with_default: InMemoryEventStore<DomainEvent> =
            InMemoryEventStore::default();
        let s1 = _store_with_new.load("id").await.unwrap();
        let s2 = _store_with_default.load("id").await.unwrap();
        assert!(s1.events.is_empty() && s1.version == 0);
        assert!(s2.events.is_empty() && s2.version == 0);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_append_and_load_an_event() {
        let store = InMemoryEventStore::<DomainEvent>::new();
        let event = DomainEvent { name: "Teddy Test" };
        store
            .append("1", 0, &vec![event])
            .await
            .expect("expected to append to the event_store");
        let stream = store
            .load("1")
            .await
            .expect("expected to load from the event_store");
        assert_eq!(store.is_offline, false);
        assert_eq!(stream.version, 1);
        let stream_events = stream.events;
        assert_eq!(stream_events.len(), 1);
        assert_eq!(stream_events.get(0).unwrap().name, "Teddy Test");
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_append_and_load_multiple_events() {
        let store = InMemoryEventStore::<DomainEvent>::new();
        let events = vec![
            DomainEvent {
                name: "Teddy Test_1",
            },
            DomainEvent {
                name: "Teddy Test_2",
            },
            DomainEvent {
                name: "Teddy Test_3",
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
        assert_eq!(stream_events.get(0).unwrap().name, "Teddy Test_1");
        assert_eq!(stream_events.get(1).unwrap().name, "Teddy Test_2");
        assert_eq!(stream_events.get(2).unwrap().name, "Teddy Test_3");
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_to_append_if_the_wrong_version_is_expected() {
        let store = InMemoryEventStore::<DomainEvent>::new();
        let event = DomainEvent { name: "Teddy Test" };
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

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_to_load_if_the_event_store_is_offline() {
        let mut store = InMemoryEventStore::<DomainEvent>::new();
        store.toggle_offline();
        let stream_result = store
            .load("1")
            .await;
        assert!(stream_result.is_err());
        assert!(store.is_offline);
        match stream_result {
            Err(EventStoreError::Backend(msg)) => {
                assert_eq!(msg, "Event store offline");
            },
            _ => panic!("expected EventStoreError::Backend error"),
        }
    }
}
