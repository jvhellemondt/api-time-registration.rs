# modules/time_entries/adapters folder

Purpose
- Concrete implementations of the ports required by use cases and core.

What belongs here
- `outbound/`: outbound adapters (projections, intent dispatch, event store bindings).
- Later: inbound adapters (HTTP, GraphQL) co-located here or at the crate boundary.

Boundaries
- Adapters may perform input and output and depend on external libraries.
