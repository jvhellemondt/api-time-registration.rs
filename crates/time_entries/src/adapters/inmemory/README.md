# adapters/inmemory folder

Purpose
- Simple in memory implementations of ports and repositories for tests and development.

What belongs here
- In memory event store.
- In memory domain outbox.
- In memory projection repository and watermark.
- Optional in memory query handler.

Boundaries
- These implementations are not for production and do not persist data across process restarts.

