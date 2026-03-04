# Events vs Intents vs Technical Events

This guide explains the difference between domain events, domain intents, and technical events,
how they relate to each other, when to use each, and how to structure relays for outbound
communication.

---

## The Three Event Types

| | Domain Event | Intent | Technical Event |
|---|---|---|---|
| **Meaning** | Something happened (past fact) | Something must happen next (directed instruction) | The system did something at an I/O boundary |
| **Produced by** | Decider | Decider / handler | Adapter (fire-and-forget) |
| **Stored in** | Event store (append-only, permanent) | Intent outbox (durable until delivered) | Technical event store |
| **Delivery** | Broadcast via event store tail | At-least-once via outbox → relay | Fire-and-forget, never blocks or fails the main flow |
| **Consumer** | Projectors, event relay | A specific relay → external system | Monitoring, alerting, analysis |
| **Examples** | `TimeEntryRegisteredV1` | `PublishTimeEntryRegistered`, `ChargeCustomer` | `CommandAccepted`, `OutboundAdapterFailed` |

**Domain events** are facts about what happened in the domain. Permanent, local to the module,
source of truth for state and projections.

**Intents** are instructions for what must happen next outside the module. Durable, directed,
at-least-once delivery via outbox and relay.

**Technical events** are observations about what the system did at I/O boundaries — latency,
failures, acceptance rates. Fire-and-forget. Never written transactionally. Never block the
main flow. This is the architecture's substitute for logging (there are no log statements).

---

## Choosing Between Intent and Technical Event

This is the most common point of confusion. The question to ask:

> Is this a **business consequence** of a domain decision, or an **operational observation**
> about system behaviour?

| Scenario | Use |
|----------|-----|
| Notify another BC that a time entry was registered | Intent (`PublishTimeEntryRegistered`) |
| Charge a customer after approval | Intent (`ChargeCustomer`) |
| Send an approval email to the user | Intent (`NotifyUserOfApproval`) |
| Record that a command took 120ms and was accepted | Technical event (`CommandAccepted`) |
| Record that the event store write failed | Technical event (`OutboundAdapterFailed`) |
| Record that an HTTP request was received | Technical event (`HttpRequestReceived`) |
| Send weekly aggregated hours to a finance dashboard (business SLA) | Intent |
| Post per-request latency to Datadog | Technical event |

**Rule of thumb:** if losing it would violate a business contract or SLA → intent. If losing it
would reduce observability but not affect correctness → technical event.

Technical events are written by adapters at every I/O boundary — inbound and outbound — using
the `TechnicalEventStore` port. They have no return type: a failed write never propagates.

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

Ask: **does anything outside this module need to reliably receive a consequence of this decision?**

| Scenario | Use |
|----------|-----|
| Notify another bounded context via Kafka | Intent |
| Call a payment provider | Intent |
| Send an email or push notification | Intent |
| Business-critical metric delivery (finance SLA) | Intent |
| Operational metrics — latency, failure rates, request counts | Technical event |
| Audit log to an internal observability store | Technical event |
| Update a read model (projection) | Neither — projector tails the event store directly |
| Reconstruct aggregate state | Neither — `evolve()` reads domain events |

The outbox exists for **business-contract delivery**. If the relay fails to deliver, it retries.
Use it when losing a delivery would be a correctness problem. Use technical events for everything
that is observability — where dropping an occasional write reduces visibility but not correctness.

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

## Example 2: Observability — Technical Events (Not Intents)

Operational metrics — latency, acceptance rates, failure counts — are **technical events**, not
intents. They are written fire-and-forget by the adapter at the I/O boundary. No outbox, no
relay, no retry.

```rust
// use_cases/register_time_entry/handler.rs

pub async fn handle(&self, stream_id: &str, command: RegisterTimeEntry) -> Result<(), ApplicationError> {
    let start = std::time::Instant::now();

    // ... load, fold, decide ...

    match decide_register(&state, command) {
        Decision::Accepted { events, intents } => {
            self.event_store.append(stream_id, stream.version, &events).await?;
            dispatch_intents(/* ... */).await?;

            // Technical event — fire-and-forget, no return type
            let _ = self.technical_tx.send(TechnicalEvent::CommandAccepted {
                command_type: "RegisterTimeEntry".to_string(),
                event_count: events.len(),
                duration_ms: start.elapsed().as_millis() as u64,
            });

            Ok(())
        }
        Decision::Rejected { reason } => {
            let _ = self.technical_tx.send(TechnicalEvent::CommandRejected {
                command_type: "RegisterTimeEntry".to_string(),
                reason: reason.to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
            });

            Err(ApplicationError::Domain(reason.to_string()))
        }
    }
}
```

Technical events flow to a `TechnicalEventStore` — a separate store from the domain event store
and the intent outbox. The implementation can write to Postgres, emit JSON to stdout for a log
aggregator (Datadog, Loki), or discard in tests. The adapter never knows which.

**Do not route operational metrics through the intent outbox.** That conflates delivery
guarantees with observability, adds unnecessary relay complexity, and puts operational noise
into the business-contract delivery path.

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
      handler.rs                  ← dispatches intents; writes technical events

  adapters/
    outbound/
      intent_outbox.rs            ← dispatch_intents(): intent enum → OutboxRow
      relays/
        publish_time_entry_registered_relay.rs   ← Kafka (intent relay)
        charge_customer_relay.rs                 ← payment provider (intent relay)
        inform_caller_of_rejection_relay.rs      ← notification (intent relay)

src/shared/infrastructure/
  technical_event_store/
    mod.rs                        ← TechnicalEventStore trait (fire-and-forget write)
    in_memory.rs                  ← dev / test
    postgres.rs                   ← structured rows, queryable by SQL
    stdout.rs                     ← newline-delimited JSON for log aggregators

src/shell/
  main.rs                         ← wires IntentRelayRunner + TechnicalEventStore
  workers/
    intent_relay_runner.rs        ← polls outbox, dispatches to per-intent relays
```

**One relay per intent type.** Each relay file handles exactly one variant of the intent enum.
No relay knows about any other relay. No broker topic or external URL appears outside relay files.

Operational metrics and observability go through `TechnicalEventStore` — a completely separate
path with no outbox, no relay, and no delivery guarantees.

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