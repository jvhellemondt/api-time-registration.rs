# time-backend repository

Purpose
- This repository contains the backend implementation for the time registration bounded context.
- It is structured as a Rust workspace. This scaffold creates the time entries crate.

Structure
- crates/time_entries: the time entries bounded context.
- Inside the crate, code is split into:
  - core (pure domain logic)
  - application (imperative orchestration)
  - adapters (input and output implementations)
  - shell (developer runners and composition)

Guiding principles
- Keep the core pure and free of input or output.
- Put orchestration and transactions in application.
- Place all input and output in adapters and shell.
- Write unit tests in core, contract tests for adapters, and end-to-end tests across layers.

Evolution rules
- Prefer additive changes.
- Version events for breaking changes.

