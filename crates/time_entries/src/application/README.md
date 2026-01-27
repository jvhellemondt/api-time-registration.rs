# application folder

Purpose
- Orchestrates input and output around the pure core: command handlers, projector runners, repositories, and queries.

What belongs here
- command_handlers: one file per intent that wires event store, decider, and domain outbox.
- projector: repository traits and runners for building read models.
- queries and query_handlers: read model access shapes and their handler traits.

Boundaries
- No business rules. Business rules are in core.
- This layer coordinates persistence and messaging using ports implemented by adapters.

