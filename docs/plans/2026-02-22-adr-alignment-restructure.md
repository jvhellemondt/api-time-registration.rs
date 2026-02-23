# ADR Alignment: Big Bang Restructure Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Restructure `crates/time_entries/src/` to match ADR-0001 folder structure and introduce
the Decision type, domain intents, and projections as specified in ADR-0003, ADR-0005, ADR-0006.

**Architecture:** Functional Core Imperative Shell (FCIS) with modular vertical slices.
All new files are created first, `lib.rs` is rewritten last, then old files are deleted.
Nothing compiles until all files exist and `lib.rs` is updated — this is the big bang.

**Tech Stack:** Rust, async-trait, tokio, serde_json, thiserror, rstest, cargo-nextest

---

## Target structure

```
src/
  shared/
    core/primitives.rs
    infrastructure/
      event_store/mod.rs + in_memory.rs
      intent_outbox/mod.rs + in_memory.rs
  modules/
    time_entries/
      core/events.rs + events/v1/time_entry_registered.rs
           state.rs + evolve.rs + intents.rs + projections.rs
      use_cases/
        register_time_entry/command.rs + decision.rs + decide.rs + handler.rs
        list_time_entries_by_user/projection.rs + queries_port.rs + handler.rs
      adapters/outbound/projections.rs + projections_in_memory.rs
                        intent_outbox.rs + event_store.rs
  shell/mod.rs + workers/projector_runner.rs
  tests/ (unchanged — only imports inside new files are updated)
```

---

### Task 1: Create `shared/infrastructure/event_store/mod.rs`

**File:** Create `crates/time_entries/src/shared/infrastructure/event_store/mod.rs`

Move `EventStore`, `EventStoreError`, `LoadedStream` from `core/ports.rs`. No logic changes.

```rust
use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventStoreError {
    #[error("version mismatch: expected {expected}, actual {actual}")]
    VersionMismatch { expected: i64, actual: i64 },

    #[error("backend error: {0}")]
    Backend(String),
}

#[derive(Debug, Clone)]
pub struct LoadedStream<E> {
    pub events: Vec<E>,
    pub version: i64,
}

#[async_trait]
pub trait EventStore<Event: Clone + Send + Sync + 'static>: Send + Sync {
    async fn load(&self, stream_id: &str) -> Result<LoadedStream<Event>, EventStoreError>;
    async fn append(
        &self,
        stream_id: &str,
        expected_version: i64,
        new_events: &[Event],
    ) -> Result<(), EventStoreError>;
}
```

---

### Task 2: Create `shared/infrastructure/event_store/in_memory.rs`

**File:** Create `crates/time_entries/src/shared/infrastructure/event_store/in_memory.rs`

Same as `adapters/in_memory/in_memory_event_store.rs`. Only the `use` path changes.

```rust
use crate::shared::infrastructure::event_store::{EventStore, EventStoreError, LoadedStream};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;

pub struct InMemoryEventStore<Event: Clone + Send + Sync + 'static> {
    inner: RwLock<HashMap<String, Vec<Event>>>,
    is_offline: bool,
    delay_append_ms: AtomicU64,
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
            delay_append_ms: AtomicU64::new(0),
        }
    }

    pub fn toggle_offline(&mut self) {
        self.is_offline = !self.is_offline;
    }

    pub fn set_delay_append_ms(&self, ms: u64) {
        self.delay_append_ms.store(ms, Ordering::SeqCst);
    }
}

#[async_trait::async_trait]
impl<Event> EventStore<Event> for InMemoryEventStore<Event>
where
    Event: Clone + Send + Sync + 'static,
{
    async fn load(&self, id: &str) -> Result<LoadedStream<Event>, EventStoreError> {
        if self.is_offline {
            return Err(EventStoreError::Backend("Event store offline".to_string()));
        }
        let guard = self.inner.read().await;
        let events = guard.get(id).cloned().unwrap_or_default();
        Ok(LoadedStream {
            version: guard.get(id).map(|v| v.len()).unwrap_or(0) as i64,
            events,
        })
    }

    async fn append(
        &self,
        stream_id: &str,
        expected_version: i64,
        new_events: &[Event],
    ) -> Result<(), EventStoreError> {
        let ms = self.delay_append_ms.load(Ordering::SeqCst);
        if ms > 0 {
            tokio::time::sleep(Duration::from_millis(ms)).await;
        }
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
    use crate::tests::fixtures::events::domain_event::DomainEvent;

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
        store.append("1", 0, &vec![event]).await.expect("expected to append");
        let stream = store.load("1").await.expect("expected to load");
        assert_eq!(store.is_offline, false);
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
        store.append("1", 0, &vec![event]).await.expect("expected to append");
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
            DomainEvent { name: "Teddy Test_1" },
            DomainEvent { name: "Teddy Test_2" },
            DomainEvent { name: "Teddy Test_3" },
        ];
        store.append("1", 0, &events).await.expect("expected to append");
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
        let mut store = InMemoryEventStore::<DomainEvent>::new();
        store.toggle_offline();
        let result = store.load("1").await;
        assert!(result.is_err());
        assert!(store.is_offline);
        match result {
            Err(EventStoreError::Backend(msg)) => assert_eq!(msg, "Event store offline"),
            _ => panic!("expected EventStoreError::Backend error"),
        }
    }
}
```

---

### Task 3: Create `shared/infrastructure/intent_outbox/mod.rs`

**File:** Create `crates/time_entries/src/shared/infrastructure/intent_outbox/mod.rs`

Move `DomainOutbox`, `OutboxRow`, `OutboxError` from `core/ports.rs`.

```rust
use async_trait::async_trait;
use thiserror::Error;
use serde_json::Value as Json;

#[derive(Debug, Clone)]
pub struct OutboxRow {
    pub topic: String,
    pub event_type: String,
    pub event_version: i32,
    pub stream_id: String,
    pub stream_version: i64,
    pub occurred_at: i64,
    pub payload: Json,
}

#[derive(Debug, Error)]
pub enum OutboxError {
    #[error("duplicate outbox row for stream {stream_id} v{stream_version}")]
    Duplicate { stream_id: String, stream_version: i64 },

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("transient backend error: {0}")]
    Transient(String),

    #[error("backend error: {0}")]
    Backend(String),
}

#[async_trait]
pub trait DomainOutbox: Send + Sync {
    async fn enqueue(&self, row: OutboxRow) -> Result<(), OutboxError>;
}
```

---

### Task 4: Create `shared/infrastructure/intent_outbox/in_memory.rs`

**File:** Create `crates/time_entries/src/shared/infrastructure/intent_outbox/in_memory.rs`

Same as `adapters/in_memory/in_memory_domain_outbox.rs`. Only the `use` path changes.

```rust
use crate::shared::infrastructure::intent_outbox::{DomainOutbox, OutboxError, OutboxRow};
use std::collections::HashSet;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct InMemoryDomainOutbox {
    pub rows: Mutex<Vec<OutboxRow>>,
    seen: Mutex<HashSet<(String, i64)>>,
}

impl InMemoryDomainOutbox {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl DomainOutbox for InMemoryDomainOutbox {
    async fn enqueue(&self, row: OutboxRow) -> Result<(), OutboxError> {
        let key = (row.stream_id.clone(), row.stream_version);
        {
            let mut s = self.seen.lock().await;
            if !s.insert(key) {
                return Err(OutboxError::Duplicate {
                    stream_id: row.stream_id,
                    stream_version: row.stream_version,
                });
            }
        }
        self.rows.lock().await.push(row);
        Ok(())
    }
}

#[cfg(test)]
mod time_entry_in_memory_domain_outbox_tests {
    use super::*;
    use rstest::rstest;
    use crate::tests::fixtures::events::domain_event::DomainEvent;

    #[rstest]
    #[tokio::test]
    async fn it_should_enqueue_the_event() {
        let outbox = InMemoryDomainOutbox::new();
        let event = DomainEvent { name: "Teddy Test" };
        let row = OutboxRow {
            topic: "test_topic".to_string(),
            event_type: "test_event_type".to_string(),
            event_version: 0,
            stream_id: "123".to_string(),
            stream_version: 0,
            occurred_at: 0,
            payload: serde_json::to_value(&event).unwrap(),
        };
        assert!(outbox.enqueue(row).await.is_ok());
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_to_enqueue_if_duplicate_event() {
        let outbox = InMemoryDomainOutbox::new();
        let event = DomainEvent { name: "Teddy Test" };
        let row = OutboxRow {
            topic: "test_topic".to_string(),
            event_type: "test_event_type".to_string(),
            event_version: 0,
            stream_id: "123".to_string(),
            stream_version: 0,
            occurred_at: 0,
            payload: serde_json::to_value(&event).unwrap(),
        };
        outbox.enqueue(row.clone()).await.unwrap();
        let result = outbox.enqueue(row).await;
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(OutboxError::Duplicate { stream_id: _, stream_version: 0 })
        ));
    }
}
```

---

### Task 5: Create `shared/core/primitives.rs`

**File:** Create `crates/time_entries/src/shared/core/primitives.rs`

Placeholder — no bounded-context-wide primitives yet.

```rust
// Bounded context-wide primitive types shared across all modules.
// Add types here only when two or more modules need the same type.
```

---

### Task 6: Create `modules/time_entries/core/events.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/core/events.rs`

Identical content to `core/time_entry/event.rs`.

```rust
pub mod v1 {
    pub mod time_entry_registered;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum TimeEntryEvent {
    TimeEntryRegisteredV1(v1::time_entry_registered::TimeEntryRegisteredV1),
}
```

---

### Task 7: Create `modules/time_entries/core/events/v1/time_entry_registered.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/core/events/v1/time_entry_registered.rs`

Same content as `core/time_entry/event/v1/time_entry_registered.rs`. Only the fixture import path
is the same (tests/fixtures path does not change).

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TimeEntryRegisteredV1 {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub created_by: String,
}

#[cfg(test)]
mod time_entry_registered_event_tests {
    use super::*;
    use rstest::{fixture, rstest};
    use std::fs;
    use crate::tests::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;

    #[fixture]
    fn registered_event() -> TimeEntryRegisteredV1 {
        make_time_entry_registered_v1_event()
    }

    #[rstest]
    fn it_should_create_the_registered_event(registered_event: TimeEntryRegisteredV1) {
        assert_eq!(registered_event.time_entry_id, "te-fixed-0001");
        assert_eq!(registered_event.user_id, "user-fixed-0001");
        assert_eq!(registered_event.tags, vec!["Work".to_string()]);
    }

    #[fixture]
    fn golden_registered_event_json() -> serde_json::Value {
        let s = fs::read_to_string("./src/tests/fixtures/events/json/registered_event_v1.json")
            .unwrap();
        serde_json::from_str(&s).unwrap()
    }

    #[rstest]
    fn it_serializes_registered_event_stable(
        registered_event: TimeEntryRegisteredV1,
        golden_registered_event_json: serde_json::Value,
    ) {
        let json = serde_json::to_value(&registered_event).unwrap();
        assert_eq!(json, golden_registered_event_json);
    }
}
```

---

### Task 8: Create `modules/time_entries/core/state.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/core/state.rs`

Identical content to `core/time_entry/state.rs`. No imports to update.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeEntryState {
    None,
    Registered {
        time_entry_id: String,
        user_id: String,
        start_time: i64,
        end_time: i64,
        tags: Vec<String>,
        description: String,
        created_at: i64,
        created_by: String,
        updated_at: i64,
        updated_by: String,
        deleted_at: Option<i64>,
        last_event_id: Option<String>,
    },
}

#[cfg(test)]
mod time_entry_state_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn it_should_create_the_blank_state() {
        let state = TimeEntryState::None;
        match state {
            TimeEntryState::None => assert!(true),
            _ => panic!("expected None state"),
        }
    }

    #[rstest]
    fn it_should_create_the_registered_state() {
        let state = TimeEntryState::Registered {
            time_entry_id: "te-fixed-0001".to_string(),
            user_id: "user-fixed-0001".to_string(),
            start_time: 1_700_000_000_000i64,
            end_time: 1_700_000_360_000i64,
            tags: vec!["Work".to_string()],
            description: "This is a test".to_string(),
            created_at: 1_700_000_000_000i64,
            created_by: "user-fixed-0001".to_string(),
            updated_at: 1_700_000_000_000i64,
            updated_by: "user-fixed-0001".to_string(),
            deleted_at: None,
            last_event_id: None,
        };
        match state {
            TimeEntryState::Registered { time_entry_id, user_id, tags, .. } => {
                assert_eq!(time_entry_id, "te-fixed-0001");
                assert_eq!(user_id, "user-fixed-0001");
                assert_eq!(tags, vec!["Work".to_string()]);
            }
            _ => panic!("expected Registered state"),
        }
    }
}
```

---

### Task 9: Create `modules/time_entries/core/evolve.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/core/evolve.rs`

Same logic as `core/time_entry/evolve.rs`. Update `use` paths.

```rust
use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::state::TimeEntryState;

pub fn evolve(state: TimeEntryState, event: TimeEntryEvent) -> TimeEntryState {
    match (state, event) {
        (TimeEntryState::None, TimeEntryEvent::TimeEntryRegisteredV1(e)) => {
            TimeEntryState::Registered {
                time_entry_id: e.time_entry_id,
                user_id: e.user_id,
                start_time: e.start_time,
                end_time: e.end_time,
                tags: e.tags,
                description: e.description,
                created_at: e.created_at,
                created_by: e.created_by.clone(),
                updated_at: e.created_at,
                updated_by: e.created_by,
                deleted_at: None,
                last_event_id: None,
            }
        }
        (state, _) => state,
    }
}

#[cfg(test)]
mod time_entry_evolve_tests {
    use super::*;
    use crate::modules::time_entries::core::events::v1::time_entry_registered::TimeEntryRegisteredV1;
    use rstest::{fixture, rstest};
    use crate::tests::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;

    #[fixture]
    fn registered_event() -> TimeEntryRegisteredV1 {
        make_time_entry_registered_v1_event()
    }

    #[rstest]
    fn it_should_evolve_the_state_to_registered(registered_event: TimeEntryRegisteredV1) {
        let state = evolve(
            TimeEntryState::None,
            TimeEntryEvent::TimeEntryRegisteredV1(registered_event.clone()),
        );
        match state {
            TimeEntryState::Registered {
                time_entry_id,
                user_id,
                start_time,
                end_time,
                tags,
                description,
                created_at,
                created_by,
                updated_at,
                updated_by,
                deleted_at,
                last_event_id,
            } => {
                assert_eq!(time_entry_id, registered_event.time_entry_id);
                assert_eq!(user_id, registered_event.user_id);
                assert_eq!(start_time, registered_event.start_time);
                assert_eq!(end_time, registered_event.end_time);
                assert_eq!(tags, registered_event.tags);
                assert_eq!(description, registered_event.description);
                assert_eq!(created_at, registered_event.created_at);
                assert_eq!(created_by, registered_event.created_by);
                assert_eq!(updated_at, registered_event.created_at);
                assert_eq!(updated_by, registered_event.created_by);
                assert_eq!(deleted_at, None);
                assert_eq!(last_event_id, None);
            }
            _ => panic!("expected Registered state"),
        }
    }

    #[rstest]
    fn it_should_not_change_on_duplicate_registered_event(registered_event: TimeEntryRegisteredV1) {
        let registered = evolve(
            TimeEntryState::None,
            TimeEntryEvent::TimeEntryRegisteredV1(registered_event.clone()),
        );
        let ev = TimeEntryEvent::TimeEntryRegisteredV1(TimeEntryRegisteredV1 {
            time_entry_id: "te-fixed-0001".into(),
            user_id: "user-fixed-0001".into(),
            start_time: 1_700_000_000_000,
            end_time: 1_700_000_360_000,
            tags: vec!["Work".into()],
            description: "This is a test".into(),
            created_at: 1_700_000_000_000,
            created_by: "user-fixed-0001".into(),
        });
        let next = evolve(registered.clone(), ev);
        assert_eq!(
            format!("{:?}", next),
            format!("{:?}", registered),
            "state should be unchanged by fallback arm"
        );
    }
}
```

---

### Task 10: Create `modules/time_entries/core/intents.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/core/intents.rs`

New file. Domain vocabulary for what should happen after a decision. No infrastructure details here.

```rust
use crate::modules::time_entries::core::events::v1::time_entry_registered::TimeEntryRegisteredV1;

/// Domain intents produced by the decider as part of an Accepted decision.
/// The outbound intent_outbox adapter translates these into OutboxRows.
pub enum TimeEntryIntent {
    PublishTimeEntryRegistered { payload: TimeEntryRegisteredV1 },
}
```

---

### Task 11: Create `modules/time_entries/core/projections.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/core/projections.rs`

Same logic as `core/time_entry/projector/apply.rs`. Update `use` paths.

```rust
use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::TimeEntryRow;

pub enum Mutation {
    Upsert(TimeEntryRow),
}

pub fn apply(stream_id: &str, version: i64, event: &TimeEntryEvent) -> Vec<Mutation> {
    let stream_key = format!("{stream_id}:{version}");
    match event {
        TimeEntryEvent::TimeEntryRegisteredV1(details) => vec![Mutation::Upsert(TimeEntryRow {
            time_entry_id: details.time_entry_id.clone(),
            user_id: details.user_id.clone(),
            start_time: details.start_time,
            end_time: details.end_time,
            tags: details.tags.clone(),
            description: details.description.clone(),
            created_at: details.created_at,
            created_by: details.created_by.clone(),
            updated_at: details.created_at,
            updated_by: details.created_by.clone(),
            deleted_at: None,
            last_event_id: Some(stream_key),
        })],
    }
}

#[cfg(test)]
mod time_entry_projector_apply_tests {
    use super::*;
    use crate::tests::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;
    use rstest::rstest;

    #[rstest]
    fn it_should_apply_the_event() {
        let stream_id = "time-entries-0001";
        let event = make_time_entry_registered_v1_event();
        let mutations = apply(stream_id, 1, &TimeEntryEvent::TimeEntryRegisteredV1(event));
        assert_eq!(mutations.len(), 1);
        assert!(
            matches!(&mutations[0], Mutation::Upsert(TimeEntryRow { .. })),
            "expected first mutation to be Upsert(..) with a TimeEntryRow"
        );
    }
}
```

---

### Task 12: Create `modules/time_entries/use_cases/list_time_entries_by_user/projection.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/use_cases/list_time_entries_by_user/projection.rs`

Merge of `core/time_entry/projector/model.rs` (TimeEntryRow) + `application/query_handlers/time_entries_queries.rs` (TimeEntryView) + `adapters/mappers/time_entry_row_to_time_entry_view.rs` (From impl).

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimeEntryRow {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub created_by: String,
    pub updated_at: i64,
    pub updated_by: String,
    pub deleted_at: Option<i64>,
    pub last_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimeEntryView {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub created_by: String,
    pub updated_at: i64,
    pub updated_by: String,
    pub deleted_at: Option<i64>,
}

impl From<TimeEntryRow> for TimeEntryView {
    fn from(row: TimeEntryRow) -> Self {
        Self {
            time_entry_id: row.time_entry_id,
            user_id: row.user_id,
            start_time: row.start_time,
            end_time: row.end_time,
            tags: row.tags,
            description: row.description,
            created_at: row.created_at,
            created_by: row.created_by,
            updated_at: row.updated_at,
            updated_by: row.updated_by,
            deleted_at: row.deleted_at,
        }
    }
}

#[cfg(test)]
mod time_entry_projector_model_tests {
    use rstest::rstest;
    use crate::tests::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;
    use super::*;

    #[rstest]
    fn it_should_create_the_model() {
        let event = make_time_entry_registered_v1_event();
        let model = TimeEntryRow {
            time_entry_id: event.time_entry_id.clone(),
            user_id: event.user_id.clone(),
            start_time: event.start_time,
            end_time: event.end_time,
            tags: event.tags.clone(),
            description: event.description.clone(),
            created_at: event.created_at,
            created_by: event.created_by.clone(),
            updated_at: event.created_at,
            updated_by: event.created_by.clone(),
            deleted_at: None,
            last_event_id: None,
        };
        assert_eq!(model.time_entry_id, event.time_entry_id);
        assert_eq!(model.user_id, event.user_id);
    }
}
```

---

### Task 13: Create `modules/time_entries/use_cases/list_time_entries_by_user/queries_port.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/use_cases/list_time_entries_by_user/queries_port.rs`

Same as `application/query_handlers/time_entries_queries.rs`. Update `use` path for `TimeEntryView`.

```rust
use async_trait::async_trait;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::TimeEntryView;

#[async_trait]
pub trait TimeEntryQueries {
    async fn list_by_user_id(
        &self,
        user_id: &str,
        offset: u64,
        limit: u64,
        sort_by_start_time_desc: bool,
    ) -> anyhow::Result<Vec<TimeEntryView>>;
}
```

---

### Task 14: Create `modules/time_entries/use_cases/list_time_entries_by_user/handler.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/use_cases/list_time_entries_by_user/handler.rs`

Move `Projector` from `application/projector/runner.rs`. Update `use` paths.
Tests from `runner.rs` move here too.

```rust
use std::sync::Arc;
use crate::modules::time_entries::adapters::outbound::projections::{TimeEntryProjectionRepository, WatermarkRepository};
use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::projections::{apply, Mutation};

#[derive(Clone)]
pub struct Projector<TRepository, TWatermarkRepository>
where
    TRepository: TimeEntryProjectionRepository + Send + Sync + 'static,
    TWatermarkRepository: WatermarkRepository + Send + Sync + 'static,
{
    pub name: String,
    pub repository: Arc<TRepository>,
    pub watermark_repository: Arc<TWatermarkRepository>,
}

impl<TRepository, TWatermarkRepository> Projector<TRepository, TWatermarkRepository>
where
    TRepository: TimeEntryProjectionRepository + Send + Sync + 'static,
    TWatermarkRepository: WatermarkRepository + Send + Sync + 'static,
{
    pub fn new(
        name: impl Into<String>,
        repository: Arc<TRepository>,
        watermark: Arc<TWatermarkRepository>,
    ) -> Self {
        Self { name: name.into(), repository, watermark_repository: watermark }
    }

    pub async fn apply_one(
        &self,
        stream_id: &str,
        version: i64,
        event: &TimeEntryEvent,
    ) -> anyhow::Result<()> {
        for mutation in apply(stream_id, version, event) {
            match mutation {
                Mutation::Upsert(row) => self.repository.upsert(row).await?,
            }
        }
        self.watermark_repository
            .set(&self.name, &format!("{stream_id}:{version}"))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod time_entry_projector_runner_tests {
    use super::*;
    use crate::modules::time_entries::adapters::outbound::projections_in_memory::InMemoryProjections;
    use crate::modules::time_entries::core::events::v1::time_entry_registered::TimeEntryRegisteredV1;
    use crate::tests::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;
    use rstest::{fixture, rstest};

    #[fixture]
    fn before_each() -> (TimeEntryRegisteredV1, InMemoryProjections) {
        let event = make_time_entry_registered_v1_event();
        let repository = InMemoryProjections::new();
        (event, repository)
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_apply_mutations_to_the_repository(
        before_each: (TimeEntryRegisteredV1, InMemoryProjections),
    ) {
        let (event, store) = before_each;
        let st = Arc::new(store);
        let projector = Projector::new("projector-name".to_string(), st.clone(), st.clone());
        projector
            .apply_one("time-entries-0001", 0, &TimeEntryEvent::TimeEntryRegisteredV1(event))
            .await
            .expect("apply_one failed");
        assert_eq!(
            st.get("projector-name").await.unwrap(),
            Some(String::from("time-entries-0001:0"))
        );
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_if_the_repository_is_offline(
        before_each: (TimeEntryRegisteredV1, InMemoryProjections),
    ) {
        let (event, mut store) = before_each;
        store.toggle_offline();
        let st = Arc::new(store);
        let projector = Projector::new("projector-name".to_string(), st.clone(), st.clone());
        let result = projector
            .apply_one("time-entries-0001", 0, &TimeEntryEvent::TimeEntryRegisteredV1(event))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Projections repository offline"));
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_if_the_watermark_repository_is_offline(
        before_each: (TimeEntryRegisteredV1, InMemoryProjections),
    ) {
        let (event, store) = before_each;
        let mut watermark_repository = InMemoryProjections::new();
        watermark_repository.toggle_offline();
        let wm = Arc::new(watermark_repository);
        let projector = Projector::new("projector-name".to_string(), Arc::new(store), wm);
        let result = projector
            .apply_one("time-entries-0001", 0, &TimeEntryEvent::TimeEntryRegisteredV1(event))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Watermark repository offline"));
    }
}
```

---

### Task 15: Create `modules/time_entries/adapters/outbound/projections.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/adapters/outbound/projections.rs`

Move `TimeEntryProjectionRepository` and `WatermarkRepository` from `application/projector/repository.rs`.
Update `use` path for `TimeEntryRow`.

```rust
use async_trait::async_trait;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::TimeEntryRow;

#[async_trait]
pub trait TimeEntryProjectionRepository: Send + Sync {
    async fn upsert(&self, row: TimeEntryRow) -> anyhow::Result<()>;
}

#[async_trait]
pub trait WatermarkRepository: Send + Sync {
    async fn get(&self, name: &str) -> anyhow::Result<Option<String>>;
    async fn set(&self, name: &str, last: &str) -> anyhow::Result<()>;
}
```

---

### Task 16: Create `modules/time_entries/adapters/outbound/projections_in_memory.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/adapters/outbound/projections_in_memory.rs`

Same logic as `adapters/in_memory/in_memory_projections.rs`. Update all `use` paths.

```rust
use crate::modules::time_entries::adapters::outbound::projections::{
    TimeEntryProjectionRepository, WatermarkRepository,
};
use crate::modules::time_entries::use_cases::list_time_entries_by_user::{
    projection::{TimeEntryRow, TimeEntryView},
    queries_port::TimeEntryQueries,
};
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct InMemoryProjections {
    rows: RwLock<HashMap<(String, String), TimeEntryRow>>,
    watermark: RwLock<HashMap<String, String>>,
    is_offline: bool,
}

impl InMemoryProjections {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle_offline(&mut self) {
        self.is_offline = !self.is_offline;
    }
}

#[async_trait::async_trait]
impl TimeEntryProjectionRepository for InMemoryProjections {
    async fn upsert(&self, row: TimeEntryRow) -> anyhow::Result<()> {
        if self.is_offline {
            return Err(anyhow::anyhow!("Projections repository offline"));
        }
        let mut guard = self.rows.write().await;
        guard.insert((row.user_id.clone(), row.time_entry_id.clone()), row);
        Ok(())
    }
}

#[async_trait::async_trait]
impl WatermarkRepository for InMemoryProjections {
    async fn get(&self, name: &str) -> anyhow::Result<Option<String>> {
        if self.is_offline {
            return Err(anyhow::anyhow!("Watermark repository offline"));
        }
        Ok(self.watermark.read().await.get(name).cloned())
    }

    async fn set(&self, name: &str, last: &str) -> anyhow::Result<()> {
        if self.is_offline {
            return Err(anyhow::anyhow!("Watermark repository offline"));
        }
        self.watermark.write().await.insert(name.to_string(), last.to_string());
        Ok(())
    }
}

#[async_trait::async_trait]
impl TimeEntryQueries for InMemoryProjections {
    async fn list_by_user_id(
        &self,
        user_id: &str,
        offset: u64,
        limit: u64,
        sort_by_start_time_desc: bool,
    ) -> anyhow::Result<Vec<TimeEntryView>> {
        let guard = self.rows.read().await;
        let mut items: Vec<TimeEntryRow> = guard
            .iter()
            .filter(|((uid, _), _)| uid == user_id)
            .map(|(_, row)| row.clone())
            .collect();

        items.sort_by_key(|r| r.start_time);
        if sort_by_start_time_desc {
            items.reverse();
        }

        let start = offset as usize;
        let end = start.saturating_add(limit as usize).min(items.len());
        if start >= items.len() {
            return Ok(Vec::new());
        }
        Ok(items[start..end].iter().cloned().map(TimeEntryView::from).collect())
    }
}

#[cfg(test)]
pub mod time_entry_in_memory_projections_tests {
    use super::*;
    use crate::modules::time_entries::core::events::v1::time_entry_registered::TimeEntryRegisteredV1;
    use crate::tests::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;
    use rstest::{fixture, rstest};

    #[fixture]
    fn before_each() -> (TimeEntryRegisteredV1, TimeEntryRow, InMemoryProjections) {
        let event = make_time_entry_registered_v1_event();
        let row = TimeEntryRow {
            time_entry_id: event.time_entry_id.clone(),
            user_id: event.user_id.clone(),
            start_time: event.start_time,
            end_time: event.end_time,
            tags: event.tags.clone(),
            description: event.description.clone(),
            created_at: event.created_at,
            created_by: event.created_by.clone(),
            updated_at: event.created_at,
            updated_by: event.created_by.clone(),
            deleted_at: None,
            last_event_id: None,
        };
        (event, row, InMemoryProjections::new())
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_add_the_time_entry_row_to_the_repository(
        before_each: (TimeEntryRegisteredV1, TimeEntryRow, InMemoryProjections),
    ) {
        let (event, row, repository) = before_each;
        repository.upsert(row.clone()).await.expect("upsert failed");
        assert_eq!(repository.rows.read().await.len(), 1);
        assert_eq!(
            repository
                .rows
                .read()
                .await
                .get(&(event.user_id.clone(), event.time_entry_id.clone()))
                .unwrap(),
            &row
        );
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_set_the_watermark_and_confirm_its_set(
        before_each: (TimeEntryRegisteredV1, TimeEntryRow, InMemoryProjections),
    ) {
        let (_, _, repository) = before_each;
        repository.set("projector-name", "event-id").await.expect("set failed");
        assert_eq!(
            repository.get("projector-name").await.unwrap(),
            Some(String::from("event-id"))
        );
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_if_the_projections_repository_is_offline(
        before_each: (TimeEntryRegisteredV1, TimeEntryRow, InMemoryProjections),
    ) {
        let (_, row, mut repository) = before_each;
        repository.toggle_offline();
        let result = repository.upsert(row).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Projections repository offline"));
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_to_set_if_the_watermark_repository_is_offline(
        before_each: (TimeEntryRegisteredV1, TimeEntryRow, InMemoryProjections),
    ) {
        let (_, _, mut repository) = before_each;
        repository.toggle_offline();
        let result = repository.set("projector-name", "event-id").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Watermark repository offline"));
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_to_get_if_the_watermark_repository_is_offline(
        before_each: (TimeEntryRegisteredV1, TimeEntryRow, InMemoryProjections),
    ) {
        let (_, _, mut repository) = before_each;
        repository.toggle_offline();
        let result = repository.get("projector-name").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Watermark repository offline"));
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_list_all_time_entries_stored(
        before_each: (TimeEntryRegisteredV1, TimeEntryRow, InMemoryProjections),
    ) {
        let (event, row, repository) = before_each;
        repository.upsert(row.clone()).await.expect("upsert failed");
        let list = repository.list_by_user_id(&event.user_id, 0, 10, false).await.unwrap();
        let view = row.into();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], view);
    }
}
```

---

### Task 17: Create `modules/time_entries/adapters/outbound/intent_outbox.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/adapters/outbound/intent_outbox.rs`

New file. Translates `TimeEntryIntent` into `OutboxRow`. Extracts the raw outbox construction
that previously lived inside the command handler.

```rust
use crate::modules::time_entries::core::intents::TimeEntryIntent;
use crate::shared::infrastructure::intent_outbox::{DomainOutbox, OutboxError, OutboxRow};

/// Translate a list of domain intents into outbox rows and enqueue them.
/// `starting_version` is the event store stream version before the append.
/// Each intent corresponds to one new version: starting_version + index + 1.
pub async fn dispatch_intents(
    outbox: &impl DomainOutbox,
    stream_id: &str,
    starting_version: i64,
    topic: &str,
    intents: Vec<TimeEntryIntent>,
) -> Result<(), OutboxError> {
    for (i, intent) in intents.into_iter().enumerate() {
        let stream_version = starting_version + i as i64 + 1;
        match intent {
            TimeEntryIntent::PublishTimeEntryRegistered { payload } => {
                outbox
                    .enqueue(OutboxRow {
                        topic: topic.to_string(),
                        event_type: "TimeEntryRegistered".to_string(),
                        event_version: 1,
                        stream_id: stream_id.to_string(),
                        stream_version,
                        occurred_at: payload.created_at,
                        payload: serde_json::to_value(payload).unwrap(),
                    })
                    .await?;
            }
        }
    }
    Ok(())
}
```

---

### Task 18: Create `modules/time_entries/adapters/outbound/event_store.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/adapters/outbound/event_store.rs`

Placeholder — module-specific event store adapter wiring note.

```rust
// Module-specific event store outbound adapter.
// Inject a concrete EventStore implementation here when wiring in shell/mod.rs.
```

---

### Task 19: Create `modules/time_entries/use_cases/register_time_entry/command.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/use_cases/register_time_entry/command.rs`

Identical content to `core/time_entry/decider/register/command.rs`. Fixture import unchanged.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterTimeEntry {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub created_by: String,
}

#[cfg(test)]
mod time_entry_registered_command_tests {
    use super::*;
    use rstest::{fixture, rstest};
    use crate::tests::fixtures::commands::register_time_entry::RegisterTimeEntryBuilder;

    #[fixture]
    fn register_command() -> RegisterTimeEntry {
        RegisterTimeEntryBuilder::new().build()
    }

    #[rstest]
    fn it_should_create_the_command(register_command: RegisterTimeEntry) {
        assert_eq!(register_command.time_entry_id, "te-fixed-0001");
        assert_eq!(register_command.user_id, "user-fixed-0001");
        assert_eq!(register_command.tags, vec!["Work".to_string()]);
    }
}
```

---

### Task 20: Create `modules/time_entries/use_cases/register_time_entry/decision.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/use_cases/register_time_entry/decision.rs`

New file. `DecideError` moves here from `decide.rs` (prevents a circular import).
`Decision` is the new return type for the decider.

```rust
use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::intents::TimeEntryIntent;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DecideError {
    #[error("time entry already exists")]
    AlreadyExists,

    #[error("end time must be after start time")]
    InvalidInterval,
}

pub enum Decision {
    Accepted { events: Vec<TimeEntryEvent>, intents: Vec<TimeEntryIntent> },
    Rejected { reason: DecideError },
}
```

---

### Task 21: Create `modules/time_entries/use_cases/register_time_entry/decide.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/use_cases/register_time_entry/decide.rs`

Updated from `core/time_entry/decider/register/decide.rs`:
- Imports `Decision` and `DecideError` from `decision.rs`
- Returns `Decision` instead of `Result<Vec<Event>, DecideError>`
- Produces both events and intents in the `Accepted` arm
- Tests updated to match on `Decision` variants

```rust
use crate::modules::time_entries::core::{
    events::{v1::time_entry_registered::TimeEntryRegisteredV1, TimeEntryEvent},
    intents::TimeEntryIntent,
    state::TimeEntryState,
};
use crate::modules::time_entries::use_cases::register_time_entry::{
    command::RegisterTimeEntry,
    decision::{DecideError, Decision},
};

pub fn decide_register(state: &TimeEntryState, command: RegisterTimeEntry) -> Decision {
    match state {
        TimeEntryState::None => {
            if command.end_time <= command.start_time {
                return Decision::Rejected { reason: DecideError::InvalidInterval };
            }
            let payload = TimeEntryRegisteredV1 {
                time_entry_id: command.time_entry_id,
                user_id: command.user_id,
                start_time: command.start_time,
                end_time: command.end_time,
                tags: command.tags,
                description: command.description,
                created_at: command.created_at,
                created_by: command.created_by,
            };
            Decision::Accepted {
                events: vec![TimeEntryEvent::TimeEntryRegisteredV1(payload.clone())],
                intents: vec![TimeEntryIntent::PublishTimeEntryRegistered { payload }],
            }
        }
        _ => Decision::Rejected { reason: DecideError::AlreadyExists },
    }
}

#[cfg(test)]
mod time_entry_register_decide_tests {
    use super::*;
    use crate::modules::time_entries::core::evolve::evolve;
    use crate::modules::time_entries::use_cases::register_time_entry::decision::DecideError;
    use rstest::{fixture, rstest};
    use crate::tests::fixtures::commands::register_time_entry::RegisterTimeEntryBuilder;

    #[fixture]
    fn register_command() -> RegisterTimeEntry {
        RegisterTimeEntryBuilder::new().build()
    }

    #[rstest]
    fn it_should_decide_to_register_the_time_entry(register_command: RegisterTimeEntry) {
        let state = TimeEntryState::None;
        let decision = decide_register(&state, register_command);
        match decision {
            Decision::Accepted { events, intents } => {
                assert_eq!(events.len(), 1);
                assert_eq!(intents.len(), 1);
                assert!(matches!(&events[0], TimeEntryEvent::TimeEntryRegisteredV1(_)));
                assert!(matches!(
                    &intents[0],
                    TimeEntryIntent::PublishTimeEntryRegistered { .. }
                ));
            }
            Decision::Rejected { .. } => panic!("expected Accepted"),
        }
    }

    #[rstest]
    fn it_should_decide_that_the_time_entry_already_exists(register_command: RegisterTimeEntry) {
        let state = TimeEntryState::None;
        let first = decide_register(&state, register_command.clone());
        let register_event = match first {
            Decision::Accepted { mut events, .. } => events.remove(0),
            _ => panic!("expected Accepted for first decision"),
        };
        let registered_state = evolve(state, register_event);
        let second = decide_register(&registered_state, register_command);
        assert!(matches!(second, Decision::Rejected { reason: DecideError::AlreadyExists }));
    }

    #[rstest]
    fn it_should_decide_that_the_time_entry_is_invalid_by_interval(
        register_command: RegisterTimeEntry,
    ) {
        let state = TimeEntryState::None;
        let command = RegisterTimeEntryBuilder::new()
            .start_time(register_command.end_time)
            .end_time(register_command.start_time)
            .build();
        let decision = decide_register(&state, command);
        assert!(matches!(
            decision,
            Decision::Rejected { reason: DecideError::InvalidInterval }
        ));
    }
}
```

---

### Task 22: Create `modules/time_entries/use_cases/register_time_entry/handler.rs`

**File:** Create `crates/time_entries/src/modules/time_entries/use_cases/register_time_entry/handler.rs`

Updated from `application/command_handlers/register_handler.rs`:
- `ApplicationError` moves here (was in `application/errors.rs`)
- Handler renamed to `RegisterTimeEntryHandler`
- Uses `Decision` type; delegates outbox writing to `dispatch_intents`
- Tests updated for new imports and handler name

```rust
use std::sync::Arc;
use thiserror::Error;
use crate::shared::infrastructure::event_store::{EventStore, EventStoreError};
use crate::shared::infrastructure::intent_outbox::{DomainOutbox, OutboxError};
use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::evolve::evolve;
use crate::modules::time_entries::core::state::TimeEntryState;
use crate::modules::time_entries::use_cases::register_time_entry::command::RegisterTimeEntry;
use crate::modules::time_entries::use_cases::register_time_entry::decide::decide_register;
use crate::modules::time_entries::use_cases::register_time_entry::decision::Decision;
use crate::modules::time_entries::adapters::outbound::intent_outbox::dispatch_intents;

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
        Self { topic: topic.into(), event_store, outbox }
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

        let state = stream.events.iter().cloned().fold(TimeEntryState::None, evolve);

        match decide_register(&state, command) {
            Decision::Accepted { events, intents } => {
                self.event_store
                    .append(stream_id, stream.version, &events)
                    .await
                    .map_err(ApplicationError::VersionConflict)?;
                dispatch_intents(&*self.outbox, stream_id, stream.version, &self.topic, intents)
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
    use std::sync::Arc;
    use crate::shared::infrastructure::event_store::{EventStore, EventStoreError};
    use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
    use crate::shared::infrastructure::intent_outbox::{OutboxError, OutboxRow};
    use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
    use crate::modules::time_entries::core::events::TimeEntryEvent;
    use crate::modules::time_entries::use_cases::register_time_entry::command::RegisterTimeEntry;
    use crate::modules::time_entries::use_cases::register_time_entry::decision::DecideError;
    use crate::modules::time_entries::use_cases::register_time_entry::handler::{
        ApplicationError, RegisterTimeEntryHandler,
    };
    use crate::tests::fixtures::commands::register_time_entry::RegisterTimeEntryBuilder;
    use crate::tests::fixtures::events::time_entry_registered_v1::make_time_entry_registered_v1_event;
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
        let es = Arc::new(event_store);
        let ob = Arc::new(outbox);
        let handler = RegisterTimeEntryHandler::new(TOPIC, es.clone(), ob);
        handler.handle(stream_id, command).await.expect("handle failed");
        let stream = es.load(stream_id).await.expect("load failed");
        assert_eq!(stream.events.len(), 1);
    }

    #[rstest]
    #[tokio::test]
    async fn handle_register_fails_if_time_entry_exists(before_each: BeforeEachReturn) {
        let (stream_id, command, event_store, outbox) = before_each;
        let handler =
            RegisterTimeEntryHandler::new(TOPIC, Arc::new(event_store), Arc::new(outbox));
        handler.handle(stream_id, command.clone()).await.expect("first handle failed");
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
        let handler =
            RegisterTimeEntryHandler::new(TOPIC, Arc::new(event_store), Arc::new(outbox));
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
        let (result1, result2) =
            join!(handler1.handle(stream_id, command.clone()), handler2.handle(stream_id, command));
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
        let handler =
            RegisterTimeEntryHandler::new(TOPIC, Arc::new(event_store), Arc::new(outbox));
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
```

---

### Task 23: Create `shell/mod.rs`

**File:** Create `crates/time_entries/src/shell/mod.rs`

Composition root module. Currently a placeholder — wiring will be added when infrastructure
implementations are chosen.

```rust
// Composition root for the time_entries bounded context.
//
// Responsibilities (when implemented):
// - Read config from environment.
// - Instantiate concrete infrastructure implementations.
// - Wire implementations into use case handlers.
// - Spawn background workers (projector runner, intent relay runner, event relay runner).
// - Expose the HTTP router to time_entries_api.

pub mod workers;
```

Also update `shell/workers/projector_runner.rs` — it stays empty, but the path now comes from
`shell/mod.rs` declaring `pub mod workers;`. No content change needed.

---

### Task 24: Rewrite `lib.rs`

**File:** Overwrite `crates/time_entries/src/lib.rs`

Declare the new module tree. This is the moment everything either compiles or doesn't.

```rust
pub mod shared {
    pub mod core {
        pub mod primitives;
    }
    pub mod infrastructure {
        pub mod event_store;
        pub mod intent_outbox;
    }
}

pub mod modules {
    pub mod time_entries {
        pub mod core {
            pub mod events;
            pub mod state;
            pub mod evolve;
            pub mod intents;
            pub mod projections;
        }
        pub mod use_cases {
            pub mod register_time_entry {
                pub mod command;
                pub mod decision;
                pub mod decide;
                pub mod handler;
            }
            pub mod list_time_entries_by_user {
                pub mod projection;
                pub mod queries_port;
                pub mod handler;
            }
        }
        pub mod adapters {
            pub mod outbound {
                pub mod projections;
                pub mod projections_in_memory;
                pub mod intent_outbox;
                pub mod event_store;
            }
        }
    }
}

pub mod shell;

#[cfg(test)]
pub mod tests {
    pub mod fixtures;

    pub mod e2e {
        pub mod list_time_entries_by_user_tests;
    }
}
```

---

### Task 25: Update the e2e test

**File:** Overwrite `crates/time_entries/src/tests/e2e/list_time_entries_by_user_tests.rs`

Update all `use` paths. Logic is unchanged.

```rust
use std::sync::Arc;
use crate::shared::infrastructure::event_store::EventStore;
use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
use crate::modules::time_entries::adapters::outbound::projections_in_memory::InMemoryProjections;
use crate::modules::time_entries::use_cases::register_time_entry::handler::RegisterTimeEntryHandler;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::handler::Projector;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::queries_port::TimeEntryQueries;
use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::tests::fixtures::commands::register_time_entry::RegisterTimeEntryBuilder;

#[tokio::test]
async fn lists_time_entries_by_user() {
    let store = Arc::new(InMemoryEventStore::<TimeEntryEvent>::new());
    let outbox = Arc::new(InMemoryDomainOutbox::new());
    let projections = Arc::new(InMemoryProjections::new());
    let projector = Projector {
        name: "time_entry_summary".into(),
        repository: projections.clone(),
        watermark_repository: projections.clone(),
    };
    let handler = RegisterTimeEntryHandler::new("time-entries", store.clone(), outbox);

    let commands: Vec<_> = [1000, 2000, 1500]
        .into_iter()
        .map(|start| {
            RegisterTimeEntryBuilder::new()
                .time_entry_id(format!("te-{start}"))
                .start_time(start)
                .end_time(start + 60_000)
                .build()
        })
        .collect();

    for (iteration, command) in commands.iter().cloned().enumerate() {
        handler
            .handle(&format!("TimeEntry-te-{iteration}"), command)
            .await
            .unwrap();

        let loaded = store.load(&format!("TimeEntry-te-{iteration}")).await.unwrap();
        projector
            .apply_one(
                &format!("TimeEntry-te-{iteration}"),
                1,
                loaded.events.first().unwrap(),
            )
            .await
            .unwrap();
    }

    let list = projections.list_by_user_id("user-fixed-0001", 0, 10, true).await.unwrap();

    assert_eq!(list.len(), 3);
    assert!(list[0].start_time >= list[1].start_time);
    assert_eq!(list[0].time_entry_id, commands[1].time_entry_id);
    assert_eq!(list[0].start_time, commands[1].start_time);
}
```

---

### Task 26: Delete old files

Run from `crates/time_entries/src/`:

```bash
rm -rf core/
rm -rf application/
rm adapters/in_memory/in_memory_event_store.rs
rm adapters/in_memory/in_memory_domain_outbox.rs
rm adapters/in_memory/in_memory_projections.rs
rm adapters/mappers/time_entry_row_to_time_entry_view.rs
rmdir adapters/in_memory adapters/mappers adapters/
rm shell/workers/projector_runner.rs
```

`shell/workers/projector_runner.rs` needs to be recreated (empty) after the rmdir cascade, since
`shell/mod.rs` now declares `pub mod workers;` and `workers/` needs to exist.

Recreate the empty worker file:

```bash
mkdir -p shell/workers
touch shell/workers/projector_runner.rs
```

Add a minimal comment so the file is non-empty and rustfmt doesn't warn:

```rust
// Projector background worker — spawned from shell/mod.rs at startup.
// Not yet implemented.
```

---

### Task 27: Run checks and commit

```bash
cd crates/time_entries
cargo run-script fmt
cargo run-script lint
cargo run-script test
cargo run-script coverage
```

Expected: all pass, coverage at 100%.

If `fmt` fails: run `cargo run-script fmt-fix` then re-check.

If `lint` fails: read the clippy output carefully — most issues will be unused imports or
missing `#[allow(...)]` attributes introduced during the move.

If `test` fails: check the failing test's `use` paths first — the most common mistake in a
big-bang restructure is a stale import in a test's `#[cfg(test)]` block.

Once all checks pass:

```bash
git add crates/time_entries/src/
git commit -m "$(cat <<'EOF'
refactor: align time_entries crate with ADR-0001 through ADR-0006

- Restructure src/ into shared/infrastructure/, modules/time_entries/,
  and shell/ per ADR-0001 folder conventions
- Introduce Decision type (Accepted/Rejected) per ADR-0005
- Introduce TimeEntryIntent domain vocabulary per ADR-0003
- Extract outbox translation into adapters/outbound/intent_outbox.rs
- Co-locate projection model and mapper into list use case
- Move projection repository traits to module outbound adapters

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```
