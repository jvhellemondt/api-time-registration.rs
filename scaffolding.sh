#!/usr/bin/env bash
set -euo pipefail

echo "[scaffold] starting"

REPO_DIR="."
CRATE_DIR="$REPO_DIR/crates/time_entries"

# Helpers
mkfile() {
  local path="$1"
  mkdir -p "$(dirname "$path")"
  cat >"$path" <<'EOF'
// ================================================================
// Purpose
//   This file is part of the time entries bounded context. It should
//   contain code that matches the responsibilities described below.
//
// Responsibilities
//   - Implement the module's specific responsibilities.
//   - Keep the code focused, small, and easy to test.
//
// Inputs and outputs
//   - Inputs: See "How it is used" for what calls into this file.
//   - Outputs: Types or values produced for callers in the same layer
//     or in the layer above.
//
// Boundaries
//   - Follow the rules of the layer this file belongs to:
//     * Core: no input or output, no database, no network.
//     * Application: orchestration only, no business rules.
//     * Adapters: input and output allowed, implement ports.
//     * Shell: composition and process management only.
//
// How it is used
//   - This comment should be adapted once code is added to explain
//     which modules call into this file and which modules it calls.
//
// Testing guidance
//   - Add unit tests for pure logic.
//   - Add integration tests when input or output is involved.
//
// Versioning and evolution
//   - Prefer additive changes.
//   - When making breaking changes to events, create a new version type.
//
// Change log
//   - 2026-01-27: Initial scaffold with comments only.
// ================================================================
EOF
  printf "// File: %s\n" "$path" >>"$path"
}

mkfile_text() {
  local path="$1"; shift
  mkdir -p "$(dirname "$path")"
  printf "%s\n" "$*" >"$path"
}

mkdoc() {
  local path="$1"; shift
  mkdir -p "$(dirname "$path")"
  printf "%s\n" "$*" >"$path"
}

# Root structure
mkdir -p "$CRATE_DIR/src"

# Root README
mkdoc "$REPO_DIR/README.md" "# time-backend repository

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
"

# Workspace Cargo.toml
mkfile_text "$REPO_DIR/Cargo.toml" "[workspace]
members = [
  \"crates/time_entries\"
]
resolver = \"2\"
"

# Crate README
mkdoc "$CRATE_DIR/README.md" "# time_entries crate

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
"

# Crate Cargo.toml
mkfile_text "$CRATE_DIR/Cargo.toml" "[package]
name = \"time_entries\"
version = \"0.1.0\"
edition = \"2021\"

[dependencies]
serde = { version = \"1\", features = [\"derive\"] }
serde_json = \"1\"
anyhow = \"1\"
async-trait = \"0.1\"
futures = \"0.3\"
tokio = { version = \"1\", features = [\"macros\", \"rt-multi-thread\", \"sync\"] }
"

# Crate lib.rs with 2018-style module declarations
mkfile_text "$CRATE_DIR/src/lib.rs" "// Crate entry point. Re-export modules so tests and binaries can import them easily.
//
// Responsibilities
// - Only declare and expose modules. No business logic here.
//
// How it is used
// - Tests import modules from this crate root to reach the code under test.
pub mod core;
pub mod application;
pub mod adapters;
pub mod shell;
"

# core folder
mkdir -p "$CRATE_DIR/src/core"
mkdoc "$CRATE_DIR/src/core/README.md" "# core folder

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
"

# core/mod 2018 style: a file per module (no mod.rs)
# Declare core as a module tree by adding a core.rs file in src
# But here we expose submodules directly via lib.rs pub mod core;
# Inside core/, create files that are modules by their filenames.

# ports.rs
mkfile_text "$CRATE_DIR/src/core/ports.rs" "// Ports define what the core needs from the outside world, without implementing it.
//
// Purpose
// - Describe abstract input and output capabilities as traits (for example: EventStore, DomainOutbox).
//
// Responsibilities
// - Keep the core independent from any database or broker by coding against traits.
//
// Boundaries
// - No concrete input or output here. Adapters implement these traits in the adapters layer.
//
// Testing guidance
// - Provide in memory implementations for tests and local development.
"

# time_entry module (2018 style): time_entry.rs file acts as module root.
mkfile_text "$CRATE_DIR/src/core/time_entry.rs" "// This module groups time entry domain components in 2018 style.
//
// Structure
// - state.rs: domain state
// - event.rs + event/: root event enum and versioned payloads
// - evolve.rs: pure state transitions
// - decider/: pure decision logic per command intent
// - projector/: mapping from events to read model mutations
//
// Import pattern
// - Use 'pub mod state;' etc. in this file once you add code. For now, see files in core/time_entry/.
"
# Create the sibling folder that holds submodules
mkdir -p "$CRATE_DIR/src/core/time_entry"

# time_entry README
mkdoc "$CRATE_DIR/src/core/time_entry/README.md" "# core/time_entry folder

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
"

# state.rs
mkfile_text "$CRATE_DIR/src/core/time_entry/state.rs" "// TimeEntryState is the canonical domain state after folding events.
//
// Suggested structure (to implement later)
// - None
// - Registered { time_entry_id, user_id, start_time, end_time, tags, description, created_at,
//                created_by, updated_at, updated_by, deleted_at, last_event_id }
//
// Boundaries
// - This file must not perform input or output.
// - Keep it framework-free.
//
// Testing guidance
// - Use the evolve function to produce states from events and assert expected fields.
"

# event.rs file-as-module + event/ folder for versions
mkfile_text "$CRATE_DIR/src/core/time_entry/event.rs" "// Root event enumeration for time entry and re-exports of versioned payloads.
//
// Purpose
// - Provide a single type to pattern match in evolve and projectors.
//
// Versioning and evolution
// - Prefer additive changes. If a breaking change is needed, add a new version and a new variant.
// - Do not change the meaning of historical events.
//
// Structure
// - This file defines the root event enumeration (later).
// - The sibling folder 'event/' contains versioned payload modules (for example: v1/).
"
mkdir -p "$CRATE_DIR/src/core/time_entry/event/v1"
mkdoc "$CRATE_DIR/src/core/time_entry/event/README.md" "# core/time_entry/event folder

Purpose
- Holds the root event enumeration (in event.rs) and versioned event payload modules (in event/ subfolders).

What belongs here
- event.rs: root enumeration and re-exports for convenience.
- event/v1: first version of concrete event payload types.

Versioning
- Prefer adding fields when evolving events.
- For breaking changes, add a new version under a new folder and a new variant in the root enumeration.
"
mkfile_text "$CRATE_DIR/src/core/time_entry/event/v1/time_entry_registered.rs" "// Event payload: TimeEntryRegisteredV1.
//
// Purpose
// - Record the business fact that a time entry was registered with the minimal fields.
//
// Responsibilities
// - Carry only identifiers and snapshot values needed by the domain today.
//
// Inputs and outputs
// - Inputs: values from the command validated by the decider.
// - Outputs: fed into evolve to produce the first registered state and into projectors.
//
// Versioning and evolution
// - Prefer adding fields. For breaking changes, create TimeEntryRegisteredV2 in a new file and add a new variant.
"

# evolve.rs
mkfile_text "$CRATE_DIR/src/core/time_entry/evolve.rs" "// Evolve function: combine a prior state with a new event to produce the next state.
//
// Purpose
// - Define deterministic transitions for each event.
//
// Boundaries
// - No input or output. No side effects.
//
// Testing guidance
// - Given a sequence of events, folding them should yield an expected state.
// - Re-applying the same event should not apply twice.
"

# decider folder (file-as-modules under it)
mkdir -p "$CRATE_DIR/src/core/time_entry/decider"
mkdoc "$CRATE_DIR/src/core/time_entry/decider/README.md" "# core/time_entry/decider folder

Purpose
- Holds pure decision logic for each command intent. Each intent gets its own subfolder or file.

What belongs here
- A subfolder per intent containing:
  - command data type
  - decide function that validates and produces events

Boundaries
- No input or output. No database or broker logic.
- Accept current time and other external values as function parameters.
"

# decider/register (2018 style: files inside subfolder)
mkdir -p "$CRATE_DIR/src/core/time_entry/decider/register"
mkdoc "$CRATE_DIR/src/core/time_entry/decider/register/README.md" "# decider/register folder

Purpose
- Defines the command and decision rules for registering a new time entry.

What belongs here
- command.rs: the command data structure for registration.
- decide.rs: the pure function that validates the command and emits an event.
- The handler for this command lives in application/command_handlers.

Boundaries
- Pure logic only. No input or output.
"
mkfile_text "$CRATE_DIR/src/core/time_entry/decider/register/command.rs" "// Command data type for registering a time entry.
//
// Purpose
// - Express user intent to create a time entry with start and end time, tags, and description.
//
// Responsibilities
// - Carry input data for the decider to validate and convert into an event.
// - Be independent of transport layer details (not tied to HTTP or GraphQL).
"
mkfile_text "$CRATE_DIR/src/core/time_entry/decider/register/decide.rs" "// Pure decision function for registration.
//
// Purpose
// - Validate the command against the current state and produce domain events on success.
//
// Responsibilities
// - Enforce rules: end time must be after start time, tag count must be within limits.
// - If state is None, emit TimeEntryRegisteredV1. If state is already registered, return an error.
// - Never perform input or output.
"

# projector folder
mkdir -p "$CRATE_DIR/src/core/time_entry/projector"
mkdoc "$CRATE_DIR/src/core/time_entry/projector/README.md" "# core/time_entry/projector folder

Purpose
- Contains pure mapping from domain events to read model mutations.

What belongs here
- model.rs: read model data shape for a single time entry row.
- apply.rs: functions that translate events into upsert or patch mutations.

Boundaries
- No database writes. Only shape data for the application projector runner to persist.
"
mkfile_text "$CRATE_DIR/src/core/time_entry/projector/model.rs" "// Read model row for a single time entry and last_event_id for idempotency.
//
// Purpose
// - Represent how a time entry is stored for fast reads in the projection store.
//
// Responsibilities
// - Map from event fields where possible.
// - Include last_event_id so idempotent upserts and patches are possible.
"
mkfile_text "$CRATE_DIR/src/core/time_entry/projector/apply.rs" "// Translate a domain event into read model mutations.
//
// Purpose
// - Build an upsert for registration and minimal patches for future change events.
//
// Responsibilities
// - Calculate last_event_id as a stable identifier like \"stream_id:version\".
// - Return a list of mutations to be persisted by the application runner.
"

# application folder
mkdir -p "$CRATE_DIR/src/application"
mkdoc "$CRATE_DIR/src/application/README.md" "# application folder

Purpose
- Orchestrates input and output around the pure core: command handlers, projector runners, repositories, and queries.

What belongs here
- command_handlers: one file per intent that wires event store, decider, and domain outbox.
- projector: repository traits and runners for building read models.
- queries and query_handlers: read model access shapes and their handler traits.

Boundaries
- No business rules. Business rules are in core.
- This layer coordinates persistence and messaging using ports implemented by adapters.
"

# application/command_handlers
mkdir -p "$CRATE_DIR/src/application/command_handlers"
mkdoc "$CRATE_DIR/src/application/command_handlers/README.md" "# application/command_handlers folder

Purpose
- Contains the orchestrators that execute write commands against the core.

What belongs here
- One file per intent (for example: register_handler.rs) that:
  - loads past events from the event store
  - folds to state using the evolve function
  - calls the decider
  - appends new events with optimistic concurrency
  - enqueues domain events into the outbox
"
mkfile_text "$CRATE_DIR/src/application/command_handlers/register_handler.rs" "// Registration command handler orchestrates the write flow.
//
// Responsibilities
// - Load past events from the event store and fold them into state.
// - Call the decider with the command and current time.
// - Append new events with optimistic concurrency.
// - Enqueue domain events into the domain outbox for publishing.
"

# application/projector
mkdir -p "$CRATE_DIR/src/application/projector"
mkdoc "$CRATE_DIR/src/application/projector/README.md" "# application/projector folder

Purpose
- Store agnostic projection machinery: repository traits and the runner that applies mutations.

What belongs here
- repository.rs: repository traits for read models and watermarks.
- runner.rs: projector runner that consumes an event feed and persists mutations.

Boundaries
- No domain decisions. Only apply and persist data reliably.
"
mkfile_text "$CRATE_DIR/src/application/projector/repository.rs" "// Repository traits for projection persistence and projector watermark tracking.
//
// Purpose
// - TimeEntryProjectionRepository: upsert and patch read model rows.
// - WatermarkRepository: track last processed event for idempotency.
"
mkfile_text "$CRATE_DIR/src/application/projector/runner.rs" "// Projector runner consumes a stream of events, translates them into mutations,
// persists them using a repository, and advances the watermark.
//
// Purpose
// - Guarantee idempotent application of events and safe recovery on failure.
"

# application/queries and query_handlers
mkdir -p "$CRATE_DIR/src/application/queries"
mkdir -p "$CRATE_DIR/src/application/query_handlers"
mkdoc "$CRATE_DIR/src/application/queries/README.md" "# application/queries folder

Purpose
- Declares shapes for read requests coming from callers.

What belongs here
- Simple data structures that describe what to fetch and with which parameters.

Boundaries
- No database logic here. The query handler traits handle data access.
"
mkfile_text "$CRATE_DIR/src/application/queries/get_time_entry.rs" "// Query shape for fetching a single time entry view.
//
// Purpose
// - Capture parameters such as organization identifier and time entry identifier.
"
mkdoc "$CRATE_DIR/src/application/query_handlers/README.md" "# application/query_handlers folder

Purpose
- Declares handler traits that implement read access for queries.

What belongs here
- For each query, a trait that returns a read model type.

Boundaries
- Implementations live in adapters (for example: in memory or PostgreSQL).
"
mkfile_text "$CRATE_DIR/src/application/query_handlers/get_time_entry_handler.rs" "// Trait for fetching a single time entry view from the projection store.
//
// Purpose
// - Abstract data access so that different storage backends can implement it.
"

# adapters
mkdir -p "$CRATE_DIR/src/adapters"
mkdoc "$CRATE_DIR/src/adapters/README.md" "# adapters folder

Purpose
- Implement the input and output required by the application and core layers.

What belongs here
- In memory adapters for fast tests and development.
- Later: database adapters, broker publishers, and cache clients.

Boundaries
- Adapters may perform input and output and depend on external libraries.
"

# adapters/inmemory
mkdir -p "$CRATE_DIR/src/adapters/inmemory"
mkdoc "$CRATE_DIR/src/adapters/inmemory/README.md" "# adapters/inmemory folder

Purpose
- Simple in memory implementations of ports and repositories for tests and development.

What belongs here
- In memory event store.
- In memory domain outbox.
- In memory projection repository and watermark.
- Optional in memory query handler.

Boundaries
- These implementations are not for production and do not persist data across process restarts.
"
mkfile_text "$CRATE_DIR/src/adapters/inmemory/inmem_event_store.rs" "// In memory implementation of the EventStore port.
//
// Purpose
// - Support command handler tests and local development without a database.
//
// Responsibilities
// - Store events per stream in memory.
// - Enforce optimistic concurrency by checking the expected version.
"
mkfile_text "$CRATE_DIR/src/adapters/inmemory/inmem_domain_outbox.rs" "// In memory implementation of the DomainOutbox port.
//
// Purpose
// - Support tests and development for verifying that command handlers enqueue domain events.
//
// Responsibilities
// - Collect enqueued domain events in a list for inspection.
"
mkfile_text "$CRATE_DIR/src/adapters/inmemory/inmem_projections.rs" "// In memory projection repository, watermark repository, and query handler.
//
// Purpose
// - Exercise projectors and queries without a database.
//
// Responsibilities
// - Store read model rows in a map keyed by identifiers.
// - Track last processed event per projector.
// - Implement query handler traits for reads.
"

# shell
mkdir -p "$CRATE_DIR/src/shell"
mkdoc "$CRATE_DIR/src/shell/README.md" "# shell folder

Purpose
- Entry points and developer utilities such as runners and workers.

What belongs here
- Small binaries or functions that wire up adapters and run loops (for example: projector runner).

Boundaries
- This is the outer layer. It is allowed to perform input and output and to compose the system.
"
mkdir -p "$CRATE_DIR/src/shell/workers"
mkdoc "$CRATE_DIR/src/shell/workers/README.md" "# shell/workers folder

Purpose
- Developer helpers to run loops like projectors or publishers.

What belongs here
- Small utilities that set up in memory adapters and run the projector for demos and manual testing.
"
mkfile_text "$CRATE_DIR/src/shell/workers/projector_runner.rs" "// Developer helper to run a projector over an in memory list or stream of events.
//
// Purpose
// - Wire the projector runner with in memory repositories for quick demos and manual testing.
"

# tests
mkdir -p "$CRATE_DIR/tests"
mkdoc "$CRATE_DIR/tests/README.md" "# tests folder

Purpose
- Contains unit tests and integration tests for the time_entries crate.

Suggested test types
- Unit tests for deciders and evolve functions in the core.
- Flow tests for command handlers using in memory adapters.
- Projector tests using in memory repositories and feeds.

Naming guidance
- Name tests after the behavior they verify, not the method names.
"
mkfile_text "$CRATE_DIR/tests/register_decide_tests.rs" "// Unit tests for the registration decider and the evolve function.
//
// Responsibilities when you add code
// - Assert validation rules (end time after start time, tag limits).
// - Assert the happy path emits the expected event.
// - Assert the evolve function produces the registered state.
"
mkfile_text "$CRATE_DIR/tests/register_flow_inmem_tests.rs" "// End to end in memory test for the registration command flow.
//
// Responsibilities when you add code
// - Use in memory event store and in memory domain outbox.
// - Call the registration command handler.
// - Assert that the event store version increments and that outbox rows are produced.
"
mkfile_text "$CRATE_DIR/tests/projector_inmem_tests.rs" "// Tests for the projector mapping and runner using in memory repositories.
//
// Responsibilities when you add code
// - Feed a registration event and assert an upserted read model row exists.
// - Feed the same event again and assert idempotency via last processed event identifier.
"

echo "[scaffold] done: created structure under $REPO_DIR"
