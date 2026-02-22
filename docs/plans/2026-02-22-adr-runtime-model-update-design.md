# Design: ADR Runtime Model Update

## Context

The project will not use AWS. There is no Lambda, no CDK, no DynamoDB Streams, no SQS, no EventBridge, no CloudWatch. The runtime model is a single long-running Rust process using `tokio::spawn` for background workers.

All ADRs currently written with AWS-specific deployment framing need to be updated to reflect this.

---

## Changes

### ADR-0002 — Rewrite

**New title**: Long-Running Process and tokio Runtime Model

ADR-0002 was never used in the codebase so it is rewritten in place (not superseded).

**New content covers**:
- Single binary, `main.rs` as composition root
- Shell reads config, instantiates infrastructure, wires all use cases and handlers
- Background workers (projector per module, intent relay runner, event relay runner) started via `tokio::spawn` at process startup, each running an infinite poll loop with a configurable sleep interval
- HTTP server started last, runs until SIGTERM triggers graceful shutdown
- Confirmation: no Lambda/CDK types anywhere in the codebase; all background tasks spawned from `main.rs`

---

### ADR-0001 — Surgical edits

- Remove `shell/lambdas/` and `cdk/` from the folder structure diagram
- Update shell description: replace Lambda entry point references with `main.rs` long-running server

---

### ADR-0003 — Surgical edits

- Replace outbox relay trigger description from "Lambda triggered by DynamoDB Stream" → "background polling task via `tokio::spawn`"

---

### ADR-0004 — Surgical edits

- Replace `CloudWatchTechnicalEventStore` example with:
  - Primary: `InMemoryTechnicalEventStore`
  - Common: `PostgresTechnicalEventStore`, `StdoutTechnicalEventStore` (structured JSON to stdout)
- Remove CloudWatch/EventBridge routing text

---

### ADR-0007 — Surgical edits

- Remove AWS event source routing table (SQS, EventBridge, SNS, DynamoDB Stream, Kinesis)
- Replace with transport-agnostic inbound trigger table: HTTP, message queue (RabbitMQ/Redis Streams/NATS), internal channel (`tokio::mpsc`)
- Remove Lambda shell entry point reference at end of section
- Keep all pattern content (two routing paths, schema translation, idempotency) intact

---

### ADR-0008 — Surgical edits

- Replace Lambda + DynamoDB Stream relay trigger with `tokio::spawn` polling loop
- Replace CDK stack section with infrastructure examples:
  - Primary: `InMemoryIntentOutbox` + polling loop
  - Common: `PostgresIntentOutbox` (advisory locks), `RedisIntentOutbox` (list-based queue)
- Keep retry/backoff/dead-letter/idempotency patterns intact
- Update local development section: the polling runner described there is now the production model, not a local-only workaround

---

### ADR-0009 — Surgical edits

- Replace Lambda + DynamoDB Stream event tailing with `tokio::spawn` polling loop
- Replace CDK stack section with infrastructure examples:
  - Primary: in-memory store + in-memory checkpoint
  - Common: `PostgresEventStore` (polling with position sequence column), `RedisEventStore` (Redis Streams with `XREAD`)
- Keep ordering/checkpoint/schema versioning/parallel publish patterns intact
- Update local development section: same as above — polling runner is the production model

---

### architecture-summary.md — Surgical edits

- Remove sentences referencing Lambda, CDK, DynamoDB Streams
- Update to reference long-running process and tokio background workers

---

## Infrastructure Examples Reference

| Store | In-memory | Common option 1 | Common option 2 |
|-------|-----------|-----------------|-----------------|
| Technical event store | `InMemoryTechnicalEventStore` | `PostgresTechnicalEventStore` | `StdoutTechnicalEventStore` |
| Intent outbox | `InMemoryIntentOutbox` | `PostgresIntentOutbox` | `RedisIntentOutbox` |
| Event store | `InMemoryEventStore` | `PostgresEventStore` | `RedisEventStore` |
| Inbound transport | HTTP (Axum) | Message queue (RabbitMQ / NATS) | Internal channel (`tokio::mpsc`) |
