# modules/time_entries/adapters/outbound folder

Purpose
- In-memory and production implementations of outbound ports for tests and development.

What belongs here
- `projections_in_memory.rs`: in-memory projection repository and watermark.
- `intent_outbox.rs`: intent dispatch adapter translating domain intents to outbox rows.
- `event_store.rs`: event store port bindings (in-memory via shared infrastructure).

Boundaries
- In-memory implementations are not for production and do not persist data across process restarts.
- Production implementations (PostgreSQL, Kafka, etc.) will live alongside in-memory files.
