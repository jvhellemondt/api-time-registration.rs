use crate::shared::infrastructure::event_store::{
    EventStore, EventStoreError, LoadedStream, StoredEvent,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

struct InnerState<Event> {
    streams: HashMap<String, Vec<Event>>,
    global_log: Vec<StoredEvent<Event>>,
}

struct Inner<Event> {
    state: RwLock<InnerState<Event>>,
    is_offline: AtomicBool,
    delay_append_ms: AtomicU64,
    sender: Option<tokio::sync::broadcast::Sender<StoredEvent<Event>>>,
}

#[derive(Clone)]
pub struct InMemoryEventStore<Event: Clone + Send + Sync + 'static> {
    inner: Arc<Inner<Event>>,
}

impl<Event: Clone + Send + Sync + 'static> Default for InMemoryEventStore<Event> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Event: Clone + Send + Sync + 'static> InMemoryEventStore<Event> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                state: RwLock::new(InnerState {
                    streams: HashMap::new(),
                    global_log: Vec::new(),
                }),
                is_offline: AtomicBool::new(false),
                delay_append_ms: AtomicU64::new(0),
                sender: None,
            }),
        }
    }

    pub fn new_with_sender(sender: tokio::sync::broadcast::Sender<StoredEvent<Event>>) -> Self {
        Self {
            inner: Arc::new(Inner {
                state: RwLock::new(InnerState {
                    streams: HashMap::new(),
                    global_log: Vec::new(),
                }),
                is_offline: AtomicBool::new(false),
                delay_append_ms: AtomicU64::new(0),
                sender: Some(sender),
            }),
        }
    }

    pub fn toggle_offline(&self) {
        self.inner.is_offline.fetch_xor(true, Ordering::SeqCst);
    }

    pub fn is_offline(&self) -> bool {
        self.inner.is_offline.load(Ordering::SeqCst)
    }

    pub fn set_delay_append_ms(&self, ms: u64) {
        self.inner.delay_append_ms.store(ms, Ordering::SeqCst);
    }

    pub async fn load_all_from(&self, from: u64) -> Result<Vec<StoredEvent<Event>>, EventStoreError> {
        if self.inner.is_offline.load(Ordering::SeqCst) {
            return Err(EventStoreError::Backend("Event store offline".to_string()));
        }
        let g = self.inner.state.read().await;
        Ok(g.global_log
            .iter()
            .filter(|e| e.global_position >= from)
            .cloned()
            .collect())
    }
}


#[async_trait::async_trait]
impl<Event> EventStore<Event> for InMemoryEventStore<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    async fn load(&self, id: &str) -> Result<LoadedStream<Event>, EventStoreError> {
        if self.inner.is_offline.load(Ordering::SeqCst) {
            return Err(EventStoreError::Backend("Event store offline".to_string()));
        }
        let guard = self.inner.state.read().await;
        let events = guard.streams.get(id).cloned().unwrap_or_default();
        Ok(LoadedStream {
            version: guard.streams.get(id).map(|v| v.len()).unwrap_or(0) as i64,
            events,
        })
    }

    async fn append(
        &self,
        stream_id: &str,
        expected_version: i64,
        new_events: &[Event],
    ) -> Result<(), EventStoreError> {
        let ms = self.inner.delay_append_ms.load(Ordering::SeqCst);
        if ms > 0 {
            tokio::time::sleep(Duration::from_millis(ms)).await;
        }

        let stored_events = {
            let mut g = self.inner.state.write().await;
            let actual = g.streams.get(stream_id).map(|v| v.len()).unwrap_or(0) as i64;
            if actual != expected_version {
                return Err(EventStoreError::VersionMismatch {
                    expected: expected_version,
                    actual,
                });
            }
            let global_start = g.global_log.len() as u64;
            let stored: Vec<StoredEvent<Event>> = new_events
                .iter()
                .enumerate()
                .map(|(i, event)| StoredEvent {
                    global_position: global_start + i as u64,
                    stream_id: stream_id.to_string(),
                    stream_version: expected_version + i as i64 + 1,
                    event: event.clone(),
                })
                .collect();
            g.streams
                .entry(stream_id.to_string())
                .or_default()
                .extend_from_slice(new_events);
            g.global_log.extend(stored.clone());
            stored
        };

        if let Some(sender) = &self.inner.sender {
            for stored in stored_events {
                let _ = sender.send(stored);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod time_entry_in_memory_event_store_tests {
    use super::*;
    use crate::tests::fixtures::events::domain_event::DomainEvent;
    use rstest::rstest;

    #[rstest]
    #[tokio::test]
    async fn it_should_initiate_with_new_and_default() {
        let _store_with_new: InMemoryEventStore<DomainEvent> = InMemoryEventStore::new();
        let _store_with_default: InMemoryEventStore<DomainEvent> = InMemoryEventStore::default();
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
            .expect("expected to append");
        let stream = store.load("1").await.expect("expected to load");
        assert_eq!(store.is_offline(), false);
        assert_eq!(stream.version, 1);
        assert_eq!(stream.events.len(), 1);
        assert_eq!(stream.events.get(0).unwrap().name, "Teddy Test");
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_append_and_load_an_event_with_delay() {
        let store = InMemoryEventStore::<DomainEvent>::new();
        store.set_delay_append_ms(100);
        let event = DomainEvent { name: "Teddy Test" };
        store
            .append("1", 0, &vec![event])
            .await
            .expect("expected to append");
        let stream = store.load("1").await.expect("expected to load");
        assert_eq!(stream.version, 1);
        assert_eq!(stream.events.len(), 1);
        assert_eq!(stream.events.get(0).unwrap().name, "Teddy Test");
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
            .expect("expected to append");
        let stream = store.load("1").await.expect("expected to load");
        assert_eq!(stream.version, 3);
        assert_eq!(stream.events.len(), 3);
        assert_eq!(stream.events.get(0).unwrap().name, "Teddy Test_1");
        assert_eq!(stream.events.get(1).unwrap().name, "Teddy Test_2");
        assert_eq!(stream.events.get(2).unwrap().name, "Teddy Test_3");
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_to_append_if_the_wrong_version_is_expected() {
        let store = InMemoryEventStore::<DomainEvent>::new();
        let event = DomainEvent { name: "Teddy Test" };
        let result = store.append("1", 1, &vec![event]).await;
        assert!(result.is_err());
        match result {
            Err(EventStoreError::VersionMismatch { expected, actual }) => {
                assert_eq!(actual, 0);
                assert_eq!(expected, 1);
            }
            _ => panic!("expected VersionMismatch error"),
        }
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_to_load_if_the_event_store_is_offline() {
        let store = InMemoryEventStore::<DomainEvent>::new();
        store.toggle_offline();
        let result = store.load("1").await;
        assert!(result.is_err());
        assert!(store.is_offline());
        match result {
            Err(EventStoreError::Backend(msg)) => assert_eq!(msg, "Event store offline"),
            _ => panic!("expected EventStoreError::Backend error"),
        }
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_populate_the_global_log_on_append() {
        let store = InMemoryEventStore::<DomainEvent>::new();
        let event = DomainEvent { name: "Log Test" };
        store.append("stream-1", 0, &[event]).await.unwrap();
        let log = store.load_all_from(0).await.unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].global_position, 0);
        assert_eq!(log[0].stream_id, "stream-1");
        assert_eq!(log[0].stream_version, 1);
        assert_eq!(log[0].event.name, "Log Test");
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_load_all_from_a_given_position() {
        let store = InMemoryEventStore::<DomainEvent>::new();
        store
            .append("s1", 0, &[DomainEvent { name: "e1" }])
            .await
            .unwrap();
        store
            .append("s2", 0, &[DomainEvent { name: "e2" }])
            .await
            .unwrap();
        store
            .append("s3", 0, &[DomainEvent { name: "e3" }])
            .await
            .unwrap();
        let from_1 = store.load_all_from(1).await.unwrap();
        assert_eq!(from_1.len(), 2);
        assert_eq!(from_1[0].global_position, 1);
        assert_eq!(from_1[1].global_position, 2);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_load_all_from_when_offline() {
        let store = InMemoryEventStore::<DomainEvent>::new();
        store.toggle_offline();
        let result = store.load_all_from(0).await;
        assert!(result.is_err());
        match result {
            Err(EventStoreError::Backend(msg)) => assert_eq!(msg, "Event store offline"),
            _ => panic!("expected Backend error"),
        }
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_publish_to_broadcast_channel_on_append() {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<StoredEvent<DomainEvent>>(16);
        let store = InMemoryEventStore::<DomainEvent>::new_with_sender(tx);
        let event = DomainEvent {
            name: "Broadcast Test",
        };
        store.append("s1", 0, &[event]).await.unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received.global_position, 0);
        assert_eq!(received.stream_id, "s1");
        assert_eq!(received.stream_version, 1);
        assert_eq!(received.event.name, "Broadcast Test");
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_assign_sequential_stream_versions_for_multiple_appended_events() {
        let store = InMemoryEventStore::<DomainEvent>::new();
        let events = vec![DomainEvent { name: "a" }, DomainEvent { name: "b" }];
        store.append("s1", 0, &events).await.unwrap();
        let log = store.load_all_from(0).await.unwrap();
        assert_eq!(log[0].stream_version, 1);
        assert_eq!(log[1].stream_version, 2);
        assert_eq!(log[0].global_position, 0);
        assert_eq!(log[1].global_position, 1);
    }
}
