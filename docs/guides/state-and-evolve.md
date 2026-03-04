# State and Evolve

This guide explains how aggregate state is reconstructed from events and how the `evolve`
function works. This is the pure functional core of event sourcing — everything the decider
needs to make a decision comes from this fold.

---

## Why State Is Reconstructed from Events

There is no mutable row in a database that represents the current state of a time entry.
The event store is the source of truth. When the command handler needs to know whether a time
entry already exists, it loads the event stream and folds all past events into state.

This means:
- State is **always derivable** — you can reconstruct it at any point in time by replaying events
- State is **never persisted** — only events are stored; state is a transient computation
- The handler always sees the **actual history**, not a stale snapshot

```
EventStore.load("TimeEntry-abc")
    → [TimeEntryRegisteredV1 { ... }]
        .fold(State::None, evolve)
    → State::Registered { ... }
```

---

## The State Type

`core/state.rs` defines an enum with one variant per **lifecycle stage** of the aggregate.
The `None` variant (the initial/empty state) is the `Default`.

```rust
// core/state.rs

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
```

**`None` is not `Option<T>`** — it's an explicit variant expressing "this aggregate does not
exist yet". The decider matches on it to decide whether a registration is valid or a duplicate.

The state enum must derive `Clone` so the handler can fold by value. `PartialEq + Eq` are useful
for tests but not required by the handler.

---

## The Evolve Function

`core/evolve.rs` contains a single pure function:

```rust
pub fn evolve(state: TimeEntryState, event: TimeEntryEvent) -> TimeEntryState
```

It takes the current state and one event, and returns the next state. No I/O. No `async`.
No `Result`. Evolve never fails — events are already committed history.

```rust
// core/evolve.rs

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
        (state, _) => state, // fallback: unknown combination — no change
    }
}
```

### The fallback arm is intentional

`(state, _) => state` handles every combination not listed above. This means:

- A `TimeEntryRegisteredV1` arriving when the aggregate is already `Registered` is silently
  ignored — the state is returned unchanged. This protects against replaying duplicate events.
- Any event variant added later to `TimeEntryEvent` but not yet handled in `evolve` is ignored
  gracefully — no panic, no error, just a no-op.

**Do not remove or narrow the fallback arm.** It is the safety net that makes the system
tolerant of schema evolution.

---

## Folding a Stream into State

The handler reconstructs state from the full event history using `fold`:

```rust
// use_cases/register_time_entry/handler.rs

let stream = self.event_store.load(stream_id).await?;

let state = stream
    .events
    .iter()
    .cloned()
    .fold(TimeEntryState::None, evolve);
```

`TimeEntryState::None` is the initial accumulator — the state of an aggregate that has
no events. Each call to `evolve` advances the state by one event. The result is the current
state of the aggregate.

---

## Evolve vs Decide

These two functions are often confused. They have completely different responsibilities:

| | `evolve` | `decide` |
|---|---|---|
| **Input** | `(State, Event)` | `(&State, Command)` |
| **Output** | `State` | `Decision` |
| **Purpose** | Reconstruct history | Validate intent |
| **Call order** | Before `decide` | After `evolve` |
| **I/O** | Never | Never |
| **Can fail** | No | No (returns `Rejected` instead) |

`evolve` is called by the handler to fold past events. `decide` is called by the handler with
the result of that fold. They never call each other.

---

## Adding a New Event Variant

When you add a new domain event, the compiler guides you through the changes needed:

**1. Add the variant to `core/events.rs`:**

```rust
pub enum TimeEntryEvent {
    TimeEntryRegisteredV1(TimeEntryRegisteredV1),
    TimeEntryUpdatedV1(TimeEntryUpdatedV1),   // new
}
```

**2. Add an arm to `evolve`:**

```rust
match (state, event) {
    (TimeEntryState::None, TimeEntryEvent::TimeEntryRegisteredV1(e)) => {
        TimeEntryState::Registered { /* ... */ }
    }
    (TimeEntryState::Registered { .. }, TimeEntryEvent::TimeEntryUpdatedV1(e)) => {
        TimeEntryState::Registered {
            updated_at: e.updated_at,
            updated_by: e.updated_by.clone(),
            // carry over unchanged fields from the previous state
            // ...
        }
    }
    (state, _) => state,
}
```

**3. Add arms to `decide` for the use cases that produce this event:**

```rust
fn decide_update(state: &TimeEntryState, command: UpdateTimeEntry) -> Decision {
    match state {
        TimeEntryState::Registered { .. } => Decision::Accepted {
            events: vec![TimeEntryEvent::TimeEntryUpdatedV1(/* ... */)],
            intents: vec![/* ... */],
        },
        TimeEntryState::None => Decision::Rejected { reason: DecideError::NotFound },
    }
}
```

The Rust compiler will flag any non-exhaustive `match` in `decide` — use that as your guide.

---

## Adding a New Lifecycle Variant

When an aggregate gains a new lifecycle stage (e.g., `Deleted`):

**1. Add the variant to `TimeEntryState`:**

```rust
pub enum TimeEntryState {
    None,
    Registered { /* ... */ },
    Deleted { time_entry_id: String, deleted_at: i64 },  // new
}
```

**2. Add the corresponding event and evolve arm:**

```rust
(TimeEntryState::Registered { time_entry_id, .. }, TimeEntryEvent::TimeEntryDeletedV1(e)) => {
    TimeEntryState::Deleted {
        time_entry_id,
        deleted_at: e.deleted_at,
    }
}
```

**3. Update `decide` arms** to handle the new variant explicitly. The compiler will tell you
which match expressions are now non-exhaustive.

---

## Testing Evolve

`evolve` is a pure function — tests need no async, no mocks, no infrastructure. Just call it
and assert on the result.

```rust
#[rstest]
fn it_should_evolve_the_state_to_registered(registered_event: TimeEntryRegisteredV1) {
    let state = evolve(
        TimeEntryState::None,
        TimeEntryEvent::TimeEntryRegisteredV1(registered_event.clone()),
    );
    assert!(matches!(state, TimeEntryState::Registered { .. }));
}

#[rstest]
fn it_should_not_change_on_duplicate_registered_event(registered_event: TimeEntryRegisteredV1) {
    let registered = evolve(
        TimeEntryState::None,
        TimeEntryEvent::TimeEntryRegisteredV1(registered_event.clone()),
    );
    // Applying the same event again hits the fallback arm — state unchanged
    let next = evolve(
        registered.clone(),
        TimeEntryEvent::TimeEntryRegisteredV1(registered_event),
    );
    assert_eq!(next, registered);
}
```

Cover every arm of `evolve` with at least one test. The fallback arm must also be covered —
show that applying an unrecognised event leaves state unchanged.

---

## Summary

| File | What it defines |
|------|----------------|
| `core/state.rs` | `TimeEntryState` enum — lifecycle variants |
| `core/evolve.rs` | `evolve(state, event) → state` — pure fold |
| `core/events.rs` | `TimeEntryEvent` enum — all event variants |

The handler folds them together:

```rust
stream.events.iter().cloned().fold(TimeEntryState::None, evolve)
```

That's the entire state reconstruction. No database reads, no caches, no shared mutable state.
