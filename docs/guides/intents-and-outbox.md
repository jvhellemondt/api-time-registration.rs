# Intents and Outbox

This guide explains domain intents, how they flow from the decider to the outbox, and how
to add new intent types and their relays.

---

## Intent vs Event

Both events and intents are produced when a command is accepted. They have different purposes:

| | Event | Intent |
|---|---|---|
| **Meaning** | What happened (past fact) | What must happen next (directed instruction) |
| **Audience** | The event store — anyone tailing it | A specific downstream system |
| **Delivery** | Append-only log | At-least-once via outbox |
| **Examples** | `TimeEntryRegisteredV1` | `PublishTimeEntryRegistered` |

An event is a permanent record in the event store. An intent is a durable message destined for
a specific endpoint — a Kafka topic, an HTTP webhook, an email service.

---

## How Intents Flow

```
decide() returns Decision::Accepted { events, intents }
        │
        ▼
handler: append events to EventStore  ─┐
handler: dispatch_intents to Outbox   ─┘  (written atomically)
        │
        ▼
OutboxRow persisted (topic, event_type, payload, stream_id, stream_version, …)
        │
        ▼  (async, background)
Intent relay polls outbox
        │ reads OutboxRow
        │ translates to infrastructure action
        ▼
Kafka message / HTTP call / email / …
        │
        ▼
Relay deletes or marks row as delivered
```

The handler writes events and outbox rows as a single atomic operation — if the event store
write succeeds but the outbox write fails, the command returns an error and the client retries.
This guarantees that every accepted decision produces a durable intent.

---

## The Intent Enum

`core/intents.rs` defines one variant per domain intent. Each variant carries the payload
the relay needs to act — typically the same data as the corresponding event.

```rust
// core/intents.rs

pub enum TimeEntryIntent {
    PublishTimeEntryRegistered { payload: TimeEntryRegisteredV1 },
}
```

The decider produces intents as part of `Decision::Accepted`:

```rust
// use_cases/register_time_entry/decide.rs

Decision::Accepted {
    events: vec![TimeEntryEvent::TimeEntryRegisteredV1(payload.clone())],
    intents: vec![TimeEntryIntent::PublishTimeEntryRegistered { payload }],
}
```

---

## The Outbox Row

`shared/infrastructure/intent_outbox/mod.rs` defines the `OutboxRow` — the wire format
written to the durable outbox:

```rust
pub struct OutboxRow {
    pub topic: String,           // destination (e.g. Kafka topic, queue name)
    pub event_type: String,      // schema identifier for the payload
    pub event_version: i32,      // payload schema version
    pub stream_id: String,       // source aggregate stream ID
    pub stream_version: i64,     // source aggregate stream version at time of intent
    pub occurred_at: i64,        // timestamp from the producing event (milliseconds)
    pub payload: serde_json::Value,  // serialised intent payload
}
```

`stream_id` + `stream_version` together form a natural idempotency key — if the relay
delivers the same row twice, the downstream system can detect and discard the duplicate.

---

## dispatch_intents

`adapters/outbound/intent_outbox.rs` translates domain intents into `OutboxRow`s and enqueues
them. It lives in the module's outbound adapters because it knows both the domain intent type
and the outbox infrastructure.

```rust
// adapters/outbound/intent_outbox.rs

pub async fn dispatch_intents(
    outbox: &impl DomainOutbox,
    stream_id: &str,
    starting_version: i64,   // stream version before the append
    topic: &str,
    intents: Vec<TimeEntryIntent>,
) -> Result<(), OutboxError> {
    for (i, intent) in intents.into_iter().enumerate() {
        let stream_version = starting_version + i as i64 + 1;
        match intent {
            TimeEntryIntent::PublishTimeEntryRegistered { payload } => {
                outbox.enqueue(OutboxRow {
                    topic: topic.to_string(),
                    event_type: "TimeEntryRegistered".to_string(),
                    event_version: 1,
                    stream_id: stream_id.to_string(),
                    stream_version,
                    occurred_at: payload.created_at,
                    payload: serde_json::to_value(payload).unwrap(),
                }).await?;
            }
        }
    }
    Ok(())
}
```

`stream_version` starts at `starting_version + 1` and increments per intent. This gives each
intent a unique version within its stream, even when a single decision produces multiple intents.

---

## The DomainOutbox Port

```rust
// shared/infrastructure/intent_outbox/mod.rs

#[async_trait]
pub trait DomainOutbox: Send + Sync {
    async fn enqueue(&self, row: OutboxRow) -> Result<(), OutboxError>;
}
```

The handler depends on `DomainOutbox`, not on a concrete implementation. The in-memory
implementation is used in tests and local development; a database-backed implementation
(Postgres, DynamoDB) is used in production.

`OutboxError::Duplicate` is returned if the same `(stream_id, stream_version)` is enqueued
twice — this prevents double-delivery caused by command handler retries.

---

## Adding a New Intent Type

**1. Add the variant to `core/intents.rs`:**

```rust
pub enum TimeEntryIntent {
    PublishTimeEntryRegistered { payload: TimeEntryRegisteredV1 },
    NotifyUserOfApproval { user_id: String, time_entry_id: String },  // new
}
```

**2. Produce it in the decider (`decide.rs`):**

```rust
Decision::Accepted {
    events: vec![TimeEntryEvent::TimeEntryApprovedV1(payload.clone())],
    intents: vec![
        TimeEntryIntent::PublishTimeEntryApproved { payload },
        TimeEntryIntent::NotifyUserOfApproval {
            user_id: command.user_id.clone(),
            time_entry_id: command.time_entry_id.clone(),
        },
    ],
}
```

**3. Add an arm to `dispatch_intents` in `adapters/outbound/intent_outbox.rs`:**

```rust
TimeEntryIntent::NotifyUserOfApproval { user_id, time_entry_id } => {
    outbox.enqueue(OutboxRow {
        topic: "notifications".to_string(),
        event_type: "TimeEntryApprovalNotification".to_string(),
        event_version: 1,
        stream_id: stream_id.to_string(),
        stream_version,
        occurred_at: chrono::Utc::now().timestamp_millis(),
        payload: serde_json::json!({ "user_id": user_id, "time_entry_id": time_entry_id }),
    }).await?;
}
```

The Rust compiler will error if you forget to add the arm — `match` on `TimeEntryIntent` is
exhaustive.

---

## The Intent Relay

An intent relay is a background worker that polls the outbox and translates each `OutboxRow`
into an infrastructure action. One relay per intent type (or per topic).

The relay pattern:
1. Poll the outbox for undelivered rows with `topic = "my-topic"`
2. For each row: translate payload → call target system (Kafka, HTTP, etc.)
3. Mark row as delivered / delete it from the outbox
4. On transient failure: leave the row in the outbox and retry (at-least-once delivery)
5. On permanent failure: move to a dead letter table for manual inspection

Relay implementations are not yet built in this codebase — see ADR-0008 for the design.

---

## At-Least-Once Delivery

The outbox guarantees at-least-once delivery: every intent will eventually be delivered, but
may be delivered more than once on relay retry. Downstream systems must handle duplicates
using the idempotency key (`stream_id` + `stream_version`).

**What this means for receivers:**

```
if already_processed(stream_id, stream_version) {
    return; // idempotent discard
}
process(payload);
mark_processed(stream_id, stream_version);
```

---

## Checklist

- [ ] `core/intents.rs` — new variant added with the payload it carries
- [ ] `decide.rs` — intent produced in the `Decision::Accepted` arm where relevant
- [ ] `adapters/outbound/intent_outbox.rs` — new match arm in `dispatch_intents`
- [ ] `OutboxRow.topic` matches the relay's subscription filter
- [ ] `OutboxRow.event_type` and `event_version` identify the schema for the relay
- [ ] Downstream receiver implements idempotency on `(stream_id, stream_version)`
- [ ] Intent relay implemented (one per topic) — see ADR-0008
