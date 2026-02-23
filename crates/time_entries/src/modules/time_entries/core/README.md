# modules/time_entries/core folder

Purpose
- Pure domain logic: state, events, evolve, intents, and projector mapping.
- This layer does not perform input or output.

What belongs here
- `state.rs`: domain state types.
- `events.rs` and `events/` folder: root event enumeration and versioned payloads.
- `evolve.rs`: pure function that folds events into state.
- `intents.rs`: domain intent vocabulary.
- `projections.rs`: pure mapping from domain events to read model mutations.

Boundaries
- No database queries, no network calls, no file system.
- If time or identifiers are needed, they are passed in as parameters by the caller.
