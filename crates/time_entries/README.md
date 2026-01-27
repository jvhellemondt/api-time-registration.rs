# time_entries crate

Purpose
- Implements the time registration bounded context.

Layers
- core: pure domain logic (state, events, deciders, evolve, projector mapping).
- application: command and query orchestration, projector runners, repository interfaces.
- adapters: implementations for input and output (start with in memory).
- shell: developer utilities and runners.

Navigation
- Start in core/time_entry to understand the domain state and events.
- See application/command_handlers for how write commands are executed.
- See application/projector for read model building.
- See adapters/inmemory for test-friendly implementations.

Testing
- Unit test core deciders and evolve functions.
- Integration test command handlers with in memory adapters.
- Integration and end-to-end test projectors and queries.

