# Events vs Intents

This guide explains the difference between domain events and domain intents, how they relate
to each other, when to use each, and how to structure relays for outbound communication.

---

## The Core Distinction

| | Event | Intent |
|---|---|---|
| **Meaning** | Something happened (past fact) | Something must happen next (directed instruction) |
| **Audience** | The event store — local to the module | A specific external target |
| **Stored in** | Event store (append-only, permanent) | Intent outbox (durable until delivered) |
| **Delivery** | Broadcast via event store tail | At-least-once via outbox → relay |
| **Consumer** | Projectors, event relay | A specific relay → external system |
| **Examples** | `TimeEntryRegisteredV1` | `PublishTimeEntryRegistered`, `ChargeCustomer` |

Events are **facts about what happened**. They are stored permanently and are the source of truth
for state reconstruction and projections. They are local to the module by default.

Intents are **instructions for what should happen next**. They are directed at a specific
external target. The outbox ensures they survive process crashes; the relay delivers them.

---

## How They Correlate

A command being accepted typically produces both:

```
decide(state, command) → Decision::Accepted {
    events: [TimeEntryRegisteredV1],   ← the domain fact
    intents: [PublishTimeEntryRegistered, NotifyApprover],  ← the consequences
}
```

The handler then:

```
1. append events → event store         ┐
2. write intents → intent outbox       ┘  atomic

(async, background)
3. intent relay polls outbox
4. relay translates intent → infrastructure message
5. relay delivers to external system
6. relay marks intent delivered
```

The event and the intent carry the same domain data but serve different purposes. The event is
the permanent record. The intent is the delivery mechanism to the outside world.

**Every outbound side-effect goes through an intent.** There are no direct calls to external
systems from command handlers.

---

## When You Need an Intent

Ask: **does anything outside this module need to react to this decision?**

| Scenario | Intent? |
|----------|---------|
| Notify another bounded context via Kafka | Yes |
| Call a payment provider | Yes |
| Send an email or push notification | Yes |
| Post metrics to a monitoring tool | Yes |
| Update a read model (projection) | No — the projector tails the event store directly |
| Reconstruct aggregate state | No — `evolve()` reads events, not intents |

Even a fire-and-forget call (metrics, audit logging to an external service) goes through the
outbox. The reason: if the process crashes after writing the event but before calling the
external service, the intent survives and the relay retries. A direct call would be silently lost.

---

## Example 1: Publishing to Kafka

A time entry is registered. The payroll BC needs to know.

**The intent (domain vocabulary):**

```rust
// core/intents.rs

pub enum TimeEntryIntent {
    PublishTimeEntryRegistered { payload: TimeEntryRegisteredV1 },
}
```

**The decider produces it:**

```rust
// use_cases/register_time_entry/decide.rs

Decision::Accepted {
    events: vec![TimeEntryEvent::TimeEntryRegisteredV1(payload.clone())],
    intents: vec![TimeEntryIntent::PublishTimeEntryRegistered { payload }],
}
```

**The relay translates it to a Kafka message:**

```rust
// adapters/outbound/relays/publish_time_entry_registered_relay.rs

impl PublishTimeEntryRegisteredRelay {
    pub async fn relay(&self, record: OutboxRecord) {
        let TimeEntryIntent::PublishTimeEntryRegistered { payload } = record.intent else {
            return; // wrong relay
        };

        let message = KafkaTimeEntryRegisteredMessage {
            idempotency_key: record.id.to_string(),
            time_entry_id: payload.time_entry_id,
            user_id: payload.user_id,
            start_time: payload.start_time,
            end_time: payload.end_time,
            occurred_at: payload.created_at,
        };

        match self.kafka.publish("time-entries.registered.v1", &message).await {
            Ok(_)  => self.outbox.mark_delivered(record.id).await.ok(),
            Err(e) => self.outbox.record_failure(record.id, e.to_string()).await.ok(),
        };
    }
}
```

The relay is the only place that knows the topic name `"time-entries.registered.v1"`.
No broker knowledge leaks into `core/` or the command handler.

---

## Example 2: Posting Metrics to a Monitoring Tool

An approval happens. You want to post a metric to Datadog (or any HTTP monitoring endpoint).

**The intent:**

```rust
pub enum TimeEntryIntent {
    PublishTimeEntryRegistered { payload: TimeEntryRegisteredV1 },
    RecordApprovalMetric { user_id: String, duration_ms: u64 },  // new
}
```

**The decider produces it alongside the domain event:**

```rust
Decision::Accepted {
    events: vec![TimeEntryEvent::TimeEntryApprovedV1(payload.clone())],
    intents: vec![
        TimeEntryIntent::PublishTimeEntryApproved { payload },
        TimeEntryIntent::RecordApprovalMetric {
            user_id: command.user_id.clone(),
            duration_ms: command.processing_duration_ms,
        },
    ],
}
```

**The relay posts to the monitoring API:**

```rust
// adapters/outbound/relays/record_approval_metric_relay.rs

impl RecordApprovalMetricRelay {
    pub async fn relay(&self, record: OutboxRecord) {
        let TimeEntryIntent::RecordApprovalMetric { user_id, duration_ms } = record.intent else {
            return;
        };

        let result = self.http_client
            .post("https://api.datadoghq.com/api/v2/series")
            .json(&DatadogMetric {
                idempotency_key: record.id.to_string(),
                metric: "time_entries.approval_duration_ms",
                value: duration_ms,
                tags: vec![format!("user_id:{user_id}")],
            })
            .send()
            .await;

        match result {
            Ok(_)  => self.outbox.mark_delivered(record.id).await.ok(),
            Err(e) => self.outbox.record_failure(record.id, e.to_string()).await.ok(),
        };
    }
}
```

Even though this is fire-and-forget from a business perspective, it goes through the outbox —
because a direct HTTP call from the command handler would be lost on a crash.

---

## Example 3: Calling an External Payment Provider

A time entry is approved and triggers billing. This is a **request/response relay** — you
call Stripe, then feed the result back into the system.

**The intent:**

```rust
pub enum TimeEntryIntent {
    ChargeCustomer { user_id: String, amount_cents: u64, idempotency_key: String },
}
```

**The relay calls Stripe and feeds the result back:**

```rust
// adapters/outbound/relays/charge_customer_relay.rs

impl ChargeCustomerRelay {
    pub async fn relay(&self, record: OutboxRecord) {
        let TimeEntryIntent::ChargeCustomer { user_id, amount_cents, idempotency_key } = record.intent else {
            return;
        };

        match self.stripe.charge(amount_cents, &user_id, &idempotency_key).await {
            Ok(charge_id) => {
                // Feed result back into the system via a new command
                self.command_bus.send(RecordPaymentResult {
                    user_id,
                    charge_id,
                    amount_cents,
                }).await.ok();

                self.outbox.mark_delivered(record.id).await.ok();
            }
            Err(e) if e.is_transient() => {
                // Retry — outbox relay will pick it up again
                self.outbox.record_failure(record.id, e.to_string()).await.ok();
            }
            Err(e) => {
                // Permanent failure → dead letter
                self.outbox.dead_letter(record.id, e.to_string()).await.ok();
            }
        }
    }
}
```

The relay marks the intent delivered only after Stripe acknowledges. The `idempotency_key`
(carried from the `OutboxRecord.id`) prevents Stripe from double-charging if the relay retries.

---

## The Two Categories of Intent

### 1. Domain intents (produced by the decider)

These are consequences of a domain decision. They live in `core/intents.rs` and are returned
inside `Decision::Accepted`. The decider owns them.

```rust
// Produced by decide_register():
Decision::Accepted {
    intents: vec![TimeEntryIntent::PublishTimeEntryRegistered { .. }],
}
```

### 2. Application-layer intents (produced by the handler)

These are cross-cutting concerns added by the command handler, not the decider. The canonical
example is rejection notification — every command rejection should notify the caller, but the
decider should not need to know that.

```rust
// handler.rs
Decision::Rejected { reason } => {
    intent_outbox.write(vec![
        TimeEntryIntent::InformCallerOfRejection { reason }
    ]).await?;
    Err(ApplicationError::Domain(reason.to_string()))
}
```

The rule: **the decider expresses domain judgment; the handler expresses communication policy.**

---

## Folder Structure

```
src/modules/time_entries/
  core/
    intents.rs                    ← intent enum (domain vocabulary only)

  use_cases/
    register_time_entry/
      decide.rs                   ← produces intents inside Decision::Accepted
      handler.rs                  ← dispatches intents; adds InformCallerOfRejection

  adapters/
    outbound/
      intent_outbox.rs            ← dispatch_intents(): intent enum → OutboxRow
      relays/
        publish_time_entry_registered_relay.rs   ← Kafka
        record_approval_metric_relay.rs          ← HTTP monitoring
        charge_customer_relay.rs                 ← payment provider
        inform_caller_of_rejection_relay.rs      ← notification

src/shell/
  main.rs                         ← wires IntentRelayRunner with all relays
  workers/
    intent_relay_runner.rs        ← polls outbox, dispatches to per-intent relays
```

**One relay per intent type.** Each relay file handles exactly one variant of the intent enum.
No relay knows about any other relay. No broker topic or external URL appears outside relay files.

---

## Adding a New Intent: Checklist

- [ ] Add variant to `core/intents.rs` with the payload needed by the relay
- [ ] Produce it in `decide.rs` inside `Decision::Accepted` (or in `handler.rs` for cross-cutting intents)
- [ ] Add match arm in `adapters/outbound/intent_outbox.rs` → `dispatch_intents()` to build `OutboxRow`
- [ ] Create `adapters/outbound/relays/<intent_name>_relay.rs`
  - [ ] Handles exactly one intent variant
  - [ ] Carries `OutboxRecord.id` as idempotency key to the external system
  - [ ] Calls `outbox.mark_delivered()` only after external acknowledgement
  - [ ] Calls `outbox.record_failure()` on transient errors (relay will retry)
  - [ ] Calls `outbox.dead_letter()` on permanent errors
- [ ] Register relay in `shell/main.rs` `IntentRelayRunner`
- [ ] External system documented as idempotent or non-idempotent