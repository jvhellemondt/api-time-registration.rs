# use_cases/list_time_entries_by_user folder

Purpose
- Projection handler and query port for listing time entries by user.

What belongs here
- `projection.rs`: TimeEntryRow read model and TimeEntryView query shape.
- `queries_port.rs`: TimeEntryQueries trait for read access.
- `handler.rs`: Projector that applies projection mutations from domain events.

Boundaries
- No business rules. Only applies and persists projection data from the event stream.
- Query implementations live in adapters (for example: in-memory or PostgreSQL).
