# core/time_entry folder

Purpose
- Contains the domain model for time entries and the pure logic that governs it.

What belongs here
- state.rs: domain state.
- event.rs and event/ folder: root event enumeration and versioned event payloads.
- evolve.rs: state transitions from events.
- decider/ folder: pure decision logic per command intent.
- projector/ folder: mapping from events to read model mutations.

Boundaries
- No input or output.
- No knowledge of databases, brokers, or frameworks.

