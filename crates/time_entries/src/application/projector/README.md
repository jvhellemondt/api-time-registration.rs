# application/projector folder

Purpose
- Store agnostic projection machinery: repository traits and the runner that applies mutations.

What belongs here
- repository.rs: repository traits for read models and watermarks.
- runner.rs: projector runner that consumes an event feed and persists mutations.

Boundaries
- No domain decisions. Only apply and persist data reliably.

