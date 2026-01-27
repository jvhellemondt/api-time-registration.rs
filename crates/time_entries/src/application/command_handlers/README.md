# application/command_handlers folder

Purpose
- Contains the orchestrators that execute write commands against the core.

What belongs here
- One file per intent (for example: register_handler.rs) that:
  - loads past events from the event store
  - folds to state using the evolve function
  - calls the decider
  - appends new events with optimistic concurrency
  - enqueues domain events into the outbox

