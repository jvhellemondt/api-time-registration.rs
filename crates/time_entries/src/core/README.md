# core folder

Purpose
- Pure domain logic: state, events, deciders, evolve, and projector mapping.
- This layer does not perform input or output.

What belongs here
- Domain state types.
- Domain event enumerations and versioned payloads.
- Deciders (pure functions that turn commands into events).
- Evolve (pure function that turns events into state).
- Projection mapping functions (event to read model mutation).
- Ports interfaces (traits) that the core depends on.

Boundaries
- No database queries, no network calls, no file system.
- If time or identifiers are needed, they are passed in as parameters by the caller.

