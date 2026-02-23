# modules/time_entries/use_cases folder

Purpose
- Orchestrates input and output around the pure core: command handlers, projection handlers,
  and query ports.

What belongs here
- One subfolder per use case, each containing:
  - command data type
  - decision type (Accepted/Rejected) and decide function (pure)
  - handler (orchestrates event store, decider, and outbox for writes; applies mutations for reads)
  - projection and query port (for read use cases)

Boundaries
- No business rules. Business rules are in core.
- Handlers coordinate persistence and messaging using ports implemented by adapters.
