---
status: accepted
date: 2026-02-22
decision-makers: []
---

# ADR-0008: Intent Relay Pattern

## Context and Problem Statement

ADR-0003 established that intents are written to an outbox for durable, at-least-once delivery to external systems. The outbox guarantees the intent survives process crashes — but something must read from the outbox, translate each intent to its target format, deliver it, and mark it delivered. This ADR defines how that relay works, how it is structured, where it runs, and how failures are handled.

## Decision Drivers

- Intents must be delivered at-least-once — the relay must retry on failure
- The relay must not deliver an intent more than necessary — external systems must be treated as potentially non-idempotent, so idempotency keys must be carried
- The translation from domain intent to infrastructure message belongs entirely in the relay — no broker or topic knowledge outside it
- The relay must be independently deployable — it must not share a process with the command handler in production
- Delivery failures must be observable via technical events
- The relay must be simple enough to reason about — a god object that knows about all intents and all targets must be avoided

## Considered Options

1. Command handler publishes directly to Kafka after persisting events — no relay
2. Single relay process polling the outbox and routing all intents
3. Per-intent-type relay functions, each responsible for one intent type
4. Relay triggered by outbox store stream, one Lambda per intent type

## Decision Outcome

Chosen option 4: **Relay triggered by outbox store stream, one Lambda per intent type**, because it maps naturally to the vertical slice structure established in ADR-0001 and ADR-0002, keeps each relay function small and focused, avoids a god object, and is independently deployable and scalable per intent type.

### Consequences

* Good, because each relay is small, focused, and independently deployable — adding a new intent type means adding a new relay, nothing else changes
* Good, because the relay is the only place that knows about brokers, topics, and external contracts — domain and application layers are completely isolated from infrastructure topology
* Good, because idempotency keys carried through to external systems make duplicate delivery safe
* Good, because dead letter handling ensures no intent is silently lost
* Good, because technical events provide full observability of relay health — latency, failure rates, dead letter volume
* Bad, because one Lambda per relay means more infrastructure to manage in CDK — offset by the predictable one-to-one mapping with intent types
* Bad, because the local relay runner adds complexity to `shell/local/main.rs` — mitigated by keeping the runner generic and relay implementations thin
* Bad, because external systems that do not support idempotency keys may process duplicate intents on relay retry — document which external systems are idempotent and which are not
* Bad, because dead letter volume may grow unnoticed — ensure CDK alarms are set on dead letter queue depth from day one
* Bad, because DynamoDB Stream filter syntax must be tested thoroughly — incorrect filters may cause a relay Lambda to miss records or receive wrong ones

### Confirmation

Compliance is confirmed by verifying each relay file handles exactly one intent type; no broker or topic string appears outside relay files; `delivered_at` is set only after external system acknowledgement; dead letter alarms are defined in the CDK stack.

## Outbox Structure

The intent outbox stores each intent as a record with enough information for the relay to deliver it:

```rust
pub struct OutboxRecord {
    pub id: OutboxRecordId,        // unique ID — used as idempotency key
    pub intent: Intent,            // domain intent type
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub attempt_count: u32,
    pub last_error: Option<String>,
}
```

The `id` is generated when the intent is written to the outbox and carried through to the external system as an idempotency key. External systems that receive the same `id` twice can safely deduplicate.

## Relay Trigger

The intent relay runner is a background task spawned by the shell at startup. It polls the intent outbox on a configurable interval and dispatches undelivered records to the matching per-intent relay:

```rust
// shell/workers/intent_relay_runner.rs

pub struct IntentRelayRunner {
    outbox: Arc<dyn DomainOutbox>,
    relays: Vec<Box<dyn IntentRelay>>,
}

impl IntentRelayRunner {
    pub async fn run(&self, interval: Duration) {
        loop {
            if let Ok(records) = self.outbox.load_undelivered().await {
                for record in records {
                    for relay in &self.relays {
                        if relay.handles(&record.intent) {
                            relay.relay(record).await;
                            break;
                        }
                    }
                }
            }
            tokio::time::sleep(interval).await;
        }
    }
}
```

In `shell/main.rs`:

```rust
tokio::spawn({
    let runner = intent_relay_runner.clone();
    async move {
        runner.run(Duration::from_millis(500)).await;
    }
});
```

## Per-Intent Relay

Each intent type has its own relay. The relay is a small outbound adapter responsible for exactly one translation and one delivery:

```rust
// adapters/outbound/relays/notify_user_of_approval_relay.rs

pub struct NotifyUserOfApprovalRelay {
    kafka: Arc<dyn KafkaProducer>,
    intent_outbox: Arc<dyn IntentOutbox>,
    technical_store: Arc<dyn TechnicalEventStore>,
}

impl NotifyUserOfApprovalRelay {
    pub async fn relay(&self, record: OutboxRecord) {
        let start = Instant::now();

        let intent = match record.intent {
            Intent::NotifyUserOfApproval { user_id, entry_id } => {
                NotifyUserMessage {
                    idempotency_key: record.id.to_string(),
                    user_id,
                    entry_id,
                    timestamp: Utc::now(),
                }
            }
            _ => {
                // wrong relay for this intent type — should not happen
                self.technical_store.write(TechnicalEvent::RelayMismatch {
                    record_id: record.id,
                    timestamp: Utc::now(),
                }).await;
                return;
            }
        };

        match self.kafka.publish("user-notifications", &intent).await {
            Ok(_) => {
                self.intent_outbox.mark_delivered(record.id).await.ok();

                self.technical_store.write(TechnicalEvent::IntentRelayed {
                    record_id: record.id,
                    intent_type: "NotifyUserOfApproval".to_string(),
                    target: "user-notifications".to_string(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    timestamp: Utc::now(),
                }).await;
            }
            Err(e) => {
                self.intent_outbox.record_failure(record.id, e.to_string()).await.ok();

                self.technical_store.write(TechnicalEvent::IntentRelayFailed {
                    record_id: record.id,
                    intent_type: "NotifyUserOfApproval".to_string(),
                    target: "user-notifications".to_string(),
                    reason: e.to_string(),
                    attempt: record.attempt_count + 1,
                    timestamp: Utc::now(),
                }).await;
            }
        }
    }
}
```

## Folder Structure

Relays live in the module's outbound adapters alongside the event store and intent outbox implementations:

```
modules/
  time_entries/
    adapters/
      outbound/
        event_store.rs
        intent_outbox.rs
        relays/
          notify_user_of_approval_relay.rs
          inform_caller_of_rejection_relay.rs
          sync_to_payroll_relay.rs

shell/
  lambdas/
    time_entries/
      relays/
        notify_user_of_approval.rs    // Lambda entry point for this relay
        inform_caller_of_rejection.rs
        sync_to_payroll.rs
  local/
    main.rs                           // starts background polling task
```

Each relay Lambda entry point follows the same bootstrap pattern as command handler Lambdas (ADR-0002) — instantiate infrastructure, wire the relay, run.

## Retry and Failure Handling

The relay follows an exponential backoff retry policy. After a configurable number of attempts, the record is moved to a dead letter store for manual inspection:

```
attempt 1 → immediate
attempt 2 → 30 seconds
attempt 3 → 5 minutes
attempt 4 → 30 minutes
attempt 5 → dead letter
```

Dead lettered intents are written to a separate DynamoDB table with the full record and error history. An EventBridge rule on the dead letter table triggers an alert. Dead lettered intents are never automatically retried — they require manual intervention or a replay tool.

The `attempt_count` and `last_error` fields on `OutboxRecord` are updated by the relay on each failure so the dead letter record carries the full failure history.

## Idempotency

Each `OutboxRecord` carries a unique `id` generated at write time. The relay carries this `id` as an idempotency key in every message sent to an external system:

```rust
NotifyUserMessage {
    idempotency_key: record.id.to_string(),  // carried to external system
    user_id,
    entry_id,
    timestamp: Utc::now(),
}
```

External systems that support idempotency keys (Kafka with exactly-once semantics, SQS with deduplication IDs, HTTP APIs with idempotency headers) use this key to deduplicate duplicate deliveries. The relay itself does not deduplicate — it relies on the outbox `delivered_at` marker to avoid reprocessing already-delivered records.

## Event Relay vs Intent Relay

The intent relay described in this ADR handles directed intents from the outbox. The event relay (which tails the domain event store and broadcasts integration events) is a separate concern with different semantics:

| | Intent Relay | Event Relay |
|---|---|---|
| Source | Intent outbox (pull) | Domain event store (tail/push) |
| Trigger | New outbox record | New event in store |
| Direction | Directed to specific target | Broadcast to subscribers |
| Delivery guarantee | At-least-once via retry | At-least-once via stream offset |
| On failure | Retry with backoff → dead letter | Retry via stream replay |
| Marks delivered | Yes — `delivered_at` on outbox record | No — advances stream offset |

They must remain separate relays. Merging them creates a god object that conflates directed communication with broadcast observation.

## Worker Wiring in main.rs

The `IntentRelayRunner` is wired and spawned in `shell/main.rs` alongside all other workers:

```rust
// shell/main.rs

let relay_runner = IntentRelayRunner::new(
    intent_outbox.clone(),
    vec![
        Box::new(NotifyUserOfApprovalRelay::new(kafka.clone(), intent_outbox.clone(), technical_store.clone())),
        Box::new(InformCallerOfRejectionRelay::new(kafka.clone(), intent_outbox.clone(), technical_store.clone())),
        Box::new(SyncToPayrollRelay::new(payroll_client.clone(), intent_outbox.clone(), technical_store.clone())),
    ],
    technical_store.clone(),
);

tokio::spawn(async move {
    relay_runner.run(Duration::from_millis(500)).await;
});
```

The `IntentRelayRunner` polls for undelivered records and dispatches each to the matching relay.

## Infrastructure Examples

The intent outbox port can be backed by any durable store. Common implementations:

| Implementation | When to use |
|---|---|
| `InMemoryDomainOutbox` | Development, testing, single-process deployments where durability is not required |
| `PostgresDomainOutbox` | Production — uses advisory locks for safe concurrent polling; records survive process restarts |
| `RedisDomainOutbox` | Production — uses a Redis list as a queue; fast, at-least-once delivery |

The shell chooses the implementation. The relay and the handler never know which backing store is used.

## Rules

1. Each relay handles exactly one intent type — no relay handles multiple intent types
2. Translation from domain intent to infrastructure message happens entirely in the relay — no broker or topic knowledge outside it
3. The relay marks an intent delivered only after the external system acknowledges it — never before
4. Idempotency keys are always carried to external systems — the relay never strips them
5. Failed deliveries are recorded on the outbox record with error and attempt count — never silently dropped
6. Dead lettered intents require manual intervention — the relay never automatically retries beyond the configured attempt limit
7. Technical events are written for every relay attempt, success and failure
