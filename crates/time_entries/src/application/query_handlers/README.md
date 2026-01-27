# application/query_handlers folder

Purpose
- Declares handler traits that implement read access for queries.

What belongs here
- For each query, a trait that returns a read model type.

Boundaries
- Implementations live in adapters (for example: in memory or PostgreSQL).

