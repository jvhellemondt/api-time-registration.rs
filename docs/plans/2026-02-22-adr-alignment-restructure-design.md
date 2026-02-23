# Design: Full ADR Alignment — Big Bang Restructure

## Context

`crates/time_entries` was developed before the current ADRs were in place. The folder structure
and conceptual model do not match ADR-0001 through ADR-0006. This design covers a single-pass
restructure to bring the crate into full alignment.

Scope: folder moves, module path updates, and the three conceptual gaps (Decision type, intents,
projections). No new behaviour is added.

---

## Target Folder Structure

```
src/
  shared/
    core/
      primitives.rs             # placeholder — no cross-module types yet
    infrastructure/
      event_store/
        mod.rs                  # EventStore, EventStoreError, LoadedStream
        in_memory.rs            # InMemoryEventStore
      intent_outbox/
        mod.rs                  # DomainOutbox, OutboxRow, OutboxError
        in_memory.rs            # InMemoryDomainOutbox
      projections/
        mod.rs                  # TimeEntryProjectionRepository, WatermarkRepository
        in_memory.rs            # InMemoryProjections

  modules/
    time_entries/
      mod.rs
      core/
        events.rs               # TimeEntryEvent enum + versioned payloads
        state.rs                # TimeEntryState
        evolve.rs               # evolve()
        intents.rs              # TimeEntryIntent enum (domain intent vocabulary)
        projections.rs          # apply() — shared projection mappings
      use_cases/
        register_time_entry/
          command.rs            # RegisterTimeEntry
          decision.rs           # Decision { Accepted { events, intents }, Rejected { reason } }
          decide.rs             # decide_register() returning Decision
          handler.rs            # command handler using Decision + intent vocabulary
        list_time_entries_by_user/
          query.rs              # ListTimeEntriesByUser query type
          projection.rs         # TimeEntryRow, TimeEntryView, From impl
          queries_port.rs       # TimeEntryQueries trait
          handler.rs            # query handler
      adapters/
        outbound/
          event_store.rs        # placeholder
          intent_outbox.rs      # placeholder

  shell/
    mod.rs                      # composition root — wires infrastructure into handlers,
                                #   spawns background workers
    workers/
      projector_runner.rs       # (unchanged, still empty)

  tests/                        # unchanged — paths updated
```

`time_entries_api/src/main.rs` remains the binary entry point and calls into `shell/mod.rs`.

---

## File Movements

| From | To |
|---|---|
| `core/ports.rs` (EventStore, EventStoreError, LoadedStream) | `shared/infrastructure/event_store/mod.rs` |
| `core/ports.rs` (DomainOutbox, OutboxRow, OutboxError) | `shared/infrastructure/intent_outbox/mod.rs` |
| `adapters/in_memory/in_memory_event_store.rs` | `shared/infrastructure/event_store/in_memory.rs` |
| `adapters/in_memory/in_memory_domain_outbox.rs` | `shared/infrastructure/intent_outbox/in_memory.rs` |
| `application/projector/repository.rs` | `shared/infrastructure/projections/mod.rs` |
| `adapters/in_memory/in_memory_projections.rs` | `shared/infrastructure/projections/in_memory.rs` |
| `core/time_entry/event.rs` + `event/v1/` | `modules/time_entries/core/events.rs` + `events/v1/` |
| `core/time_entry/state.rs` | `modules/time_entries/core/state.rs` |
| `core/time_entry/evolve.rs` | `modules/time_entries/core/evolve.rs` |
| `core/time_entry/projector/apply.rs` | `modules/time_entries/core/projections.rs` |
| `core/time_entry/decider/register/command.rs` | `modules/time_entries/use_cases/register_time_entry/command.rs` |
| `core/time_entry/decider/register/decide.rs` | `modules/time_entries/use_cases/register_time_entry/decide.rs` |
| `application/command_handlers/register_handler.rs` | `modules/time_entries/use_cases/register_time_entry/handler.rs` |
| `application/query_handlers/time_entries_queries.rs` | `modules/time_entries/use_cases/list_time_entries_by_user/queries_port.rs` |
| `core/time_entry/projector/model.rs` | `modules/time_entries/use_cases/list_time_entries_by_user/projection.rs` |
| `adapters/mappers/time_entry_row_to_time_entry_view.rs` | merged into `projection.rs` above |
| `application/errors.rs` | `modules/time_entries/use_cases/register_time_entry/handler.rs` (inline) |
| `shell/workers/projector_runner.rs` | `shell/workers/projector_runner.rs` (unchanged) |

Files deleted after move: `core/ports.rs`, `core/time_entry.rs`, `application/projector/runner.rs`
(empty), all files under `adapters/`, all files under `application/`.

---

## Conceptual Changes

### Decision type

New file: `use_cases/register_time_entry/decision.rs`

```rust
pub enum Decision {
    Accepted { events: Vec<TimeEntryEvent>, intents: Vec<TimeEntryIntent> },
    Rejected { reason: DecideError },
}
```

`decide_register` changes signature from `Result<Vec<TimeEntryEvent>, DecideError>` to `Decision`.
The handler pattern-matches on `Accepted` / `Rejected` instead of using `?` propagation from the
decider. `ApplicationError::Domain(String)` is replaced with the typed `DecideError` in the
rejected arm.

### Intents

New file: `modules/time_entries/core/intents.rs`

```rust
pub enum TimeEntryIntent {
    PublishTimeEntryRegistered {
        topic: String,
        stream_id: String,
        stream_version: i64,
        payload: TimeEntryRegisteredV1,
    },
}
```

The command handler writes `TimeEntryIntent` values (produced by the decider in the `Accepted`
arm) to the outbox. The outbound adapter (`adapters/outbound/intent_outbox.rs`) translates
`TimeEntryIntent` into `OutboxRow`. The handler no longer constructs `OutboxRow` directly.

### Projections

`core/time_entry/projector/apply.rs` moves to `modules/time_entries/core/projections.rs`. The
`Mutation` enum and `apply()` function are unchanged. `TimeEntryRow` moves to
`use_cases/list_time_entries_by_user/projection.rs` alongside `TimeEntryView` and the `From`
conversion (currently in `adapters/mappers/`).

---

## What Does Not Change

- Event versioning structure (`events/v1/time_entry_registered.rs`)
- `evolve()` function body
- All test assertions and fixture data
- `time_entries_api` crate
- `shell/workers/projector_runner.rs` content (still empty)

---

## Verification

After the restructure, run:

```bash
cargo run-script fmt
cargo run-script lint
cargo run-script test
cargo run-script coverage
```

All checks must pass before the work is considered complete.
