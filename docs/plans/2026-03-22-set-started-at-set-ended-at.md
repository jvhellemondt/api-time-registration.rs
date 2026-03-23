# Plan: Set Started At / Set Ended At — Field-by-Field Time Entry Registration

## Context

Replace the monolithic `register_time_entry` use case with two field-by-field use cases: `set_started_at` and `set_ended_at`. Each field submission emits its own event. A time entry begins in `Draft` state and auto-transitions to `Registered` once both `started_at` and `ended_at` are present and form a valid interval.

Registered entries also accept field updates (with interval validation) — there is no `AlreadyRegistered` rejection. This diverges from the design spec and is the correct settled behaviour.

The client always supplies a UUID v7 as `time_entry_id`. There are no server-side ID–generating POST endpoints; all mutations go through PUT.

`start_time` / `end_time` are renamed to `started_at` / `ended_at` throughout. `user_id` moves from query parameter to request body.

---

## Files to Create

### Events

| File | Content |
|------|---------|
| `src/modules/time_entries/core/events/v1/time_entry_initiated.rs` | `TimeEntryInitiatedV1 { time_entry_id, user_id, created_at, created_by }` |
| `src/modules/time_entries/core/events/v1/time_entry_start_set.rs` | `TimeEntryStartSetV1 { time_entry_id, started_at, updated_at, updated_by }` |
| `src/modules/time_entries/core/events/v1/time_entry_end_set.rs` | `TimeEntryEndSetV1 { time_entry_id, ended_at, updated_at, updated_by }` |

`TimeEntryRegisteredV1` is repurposed in-place as a finalization marker: `{ time_entry_id, occurred_at }`. `TimeEntryDeletedV1` is added as a stub.

### `set_started_at` use case

| File | Content |
|------|---------|
| `src/modules/time_entries/use_cases/set_started_at/command.rs` | `SetStartedAt { time_entry_id, user_id, started_at, updated_at, updated_by }` |
| `src/modules/time_entries/use_cases/set_started_at/decision.rs` | `Decision` (Accepted/Rejected), `DecideError` (InvalidInterval) |
| `src/modules/time_entries/use_cases/set_started_at/decide.rs` | `decide_set_started_at(state, cmd) → Decision` |
| `src/modules/time_entries/use_cases/set_started_at/handler.rs` | `SetStartedAtHandler<TEventStore, TOutbox>` |
| `src/modules/time_entries/use_cases/set_started_at/inbound/http.rs` | `PUT /time-entries/{id}/start` |
| `src/modules/time_entries/use_cases/set_started_at/inbound/graphql.rs` | `SetStartedAtMutation` → `setStartedAt` |

### `set_ended_at` use case

| File | Content |
|------|---------|
| `src/modules/time_entries/use_cases/set_ended_at/command.rs` | `SetEndedAt { time_entry_id, user_id, ended_at, updated_at, updated_by }` |
| `src/modules/time_entries/use_cases/set_ended_at/decision.rs` | `Decision` (Accepted/Rejected), `DecideError` (InvalidInterval) |
| `src/modules/time_entries/use_cases/set_ended_at/decide.rs` | `decide_set_ended_at(state, cmd) → Decision` |
| `src/modules/time_entries/use_cases/set_ended_at/handler.rs` | `SetEndedAtHandler<TEventStore, TOutbox>` |
| `src/modules/time_entries/use_cases/set_ended_at/inbound/http.rs` | `PUT /time-entries/{id}/end` |
| `src/modules/time_entries/use_cases/set_ended_at/inbound/graphql.rs` | `SetEndedAtMutation` → `setEndedAt` |

### Test fixtures

| File | Content |
|------|---------|
| `src/tests/fixtures/commands/set_started_at.rs` | `SetStartedAtBuilder` |
| `src/tests/fixtures/commands/set_ended_at.rs` | `SetEndedAtBuilder` |
| `src/tests/fixtures/commands/json/set_started_at.json` | Default fixture JSON |
| `src/tests/fixtures/commands/json/set_ended_at.json` | Default fixture JSON |
| `src/tests/fixtures/events/time_entry_initiated_v1.rs` | `TimeEntryInitiatedV1Builder` |
| `src/tests/fixtures/events/time_entry_start_set_v1.rs` | `TimeEntryStartSetV1Builder` |
| `src/tests/fixtures/events/time_entry_end_set_v1.rs` | `TimeEntryEndSetV1Builder` |
| `src/tests/fixtures/events/json/initiated_event_v1.json` | Golden JSON |
| `src/tests/fixtures/events/json/start_set_event_v1.json` | Golden JSON |
| `src/tests/fixtures/events/json/end_set_event_v1.json` | Golden JSON |

---

## Files to Modify

### Core domain

| File | Change |
|------|--------|
| `src/modules/time_entries/core/events.rs` | Add `TimeEntryInitiatedV1`, `TimeEntryStartSetV1`, `TimeEntryEndSetV1`, `TimeEntryDeletedV1` variants; add sub-module declarations |
| `src/modules/time_entries/core/state.rs` | Replace single `Registered` variant with `None / Draft { started_at: Option<i64>, ended_at: Option<i64>, tag_ids, created_at, created_by, updated_at, updated_by } / Registered { started_at: i64, ended_at: i64, tag_ids, created_at, created_by, updated_at, updated_by, deleted_at: Option<i64> }` |
| `src/modules/time_entries/core/evolve.rs` | Implement evolve arms for all five events |
| `src/modules/time_entries/core/intents.rs` | Change `PublishTimeEntryRegistered` inner type to inline fields `{ time_entry_id, occurred_at }` |

### Projection and query

| File | Change |
|------|--------|
| `src/modules/time_entries/core/projections.rs` | Replace `Mutation::Register` with `Upsert(TimeEntryRow)` and `Update(TimeEntryPatch)`; add mapping for all five events |
| `src/modules/time_entries/use_cases/list_time_entries_by_user/projection.rs` | Rename `start_time`/`end_time` → `started_at`/`ended_at`; change to `Option<i64>`; add `status: TimeEntryStatus` |
| `src/modules/time_entries/use_cases/list_time_entries_by_user/projector.rs` | Handle new `Upsert` / `Update` mutations |
| `src/modules/time_entries/use_cases/list_time_entries_by_user/queries.rs` | Update struct field names |
| `src/modules/time_entries/use_cases/list_time_entries_by_user/inbound/graphql.rs` | Expose `started_at`, `ended_at`, `status` on `TimeEntryView` |

### Wiring

| File | Change |
|------|--------|
| `src/shell/state.rs` | Add `set_started_at_handler` and `set_ended_at_handler` fields |
| `src/shell/http.rs` | Add routes `PUT /time-entries/{id}/start` and `PUT /time-entries/{id}/end`; remove old register routes |
| `src/shell/main.rs` | Instantiate both handlers; remove `RegisterTimeEntryHandler` |
| `src/shell/graphql.rs` | Add `SetStartedAtMutation` and `SetEndedAtMutation` to `MutationRoot` |

### Test fixtures and E2E

| File | Change |
|------|--------|
| `src/tests/fixtures.rs` | Replace `register_time_entry` fixture imports with `set_started_at` / `set_ended_at` |
| `src/tests/fixtures/commands/mod.rs` | Add `pub mod set_started_at` and `pub mod set_ended_at` |
| `src/tests/fixtures/tags.rs` | Add `set_started_at_handler` and `set_ended_at_handler` to `make_test_app_state()` |
| `src/tests/e2e/list_time_entries_by_user_tests.rs` | Rewrite to use new field-by-field flow |

---

## Files to Remove

- `src/modules/time_entries/use_cases/register_time_entry/` (entire directory: command, decide, handler, inbound/http, inbound/graphql, README)
- `src/tests/fixtures/commands/register_time_entry.rs`
- `src/tests/fixtures/commands/json/register_time_entry.json`

---

## Decider Logic

### `decide_set_started_at`

| State | Guard | Emitted events | Intent |
|-------|-------|----------------|--------|
| `None` | — | `[TimeEntryInitiatedV1, TimeEntryStartSetV1]` | — |
| `Draft { ended_at: None }` | — | `[TimeEntryStartSetV1]` | — |
| `Draft { ended_at: Some(e) }` | `started_at >= e` | — | `InvalidInterval` rejection |
| `Draft { ended_at: Some(_) }` | `started_at < ended_at` | `[TimeEntryStartSetV1, TimeEntryRegisteredV1]` | `PublishTimeEntryRegistered` |
| `Registered { ended_at }` | `started_at >= ended_at` | — | `InvalidInterval` rejection |
| `Registered { ended_at }` | `started_at < ended_at` | `[TimeEntryStartSetV1]` | — |

### `decide_set_ended_at`

| State | Guard | Emitted events | Intent |
|-------|-------|----------------|--------|
| `None` | — | `[TimeEntryInitiatedV1, TimeEntryEndSetV1]` | — |
| `Draft { started_at: None }` | — | `[TimeEntryEndSetV1]` | — |
| `Draft { started_at: Some(s) }` | `ended_at <= s` | — | `InvalidInterval` rejection |
| `Draft { started_at: Some(_) }` | `ended_at > started_at` | `[TimeEntryEndSetV1, TimeEntryRegisteredV1]` | `PublishTimeEntryRegistered` |
| `Registered { started_at }` | `ended_at <= started_at` | — | `InvalidInterval` rejection |
| `Registered { started_at }` | `ended_at > started_at` | `[TimeEntryEndSetV1]` | — |

> **Note:** `Registered` state allows field updates (no `AlreadyRegistered` rejection). Interval validity is re-checked on every update.

---

## HTTP Endpoints

All mutations require a client-supplied UUID v7 `time_entry_id`. Non-UUID or non-v7 IDs are rejected with `422 Unprocessable Entity`.

```
PUT /time-entries/{id}/start
Body: { "user_id": "...", "started_at": <ms> }
200  — field set (or entry created and field set)
409  — InvalidInterval
422  — invalid or non-v7 UUID, or malformed JSON body
500  — infrastructure error

PUT /time-entries/{id}/end
Body: { "user_id": "...", "ended_at": <ms> }
200  — field set (or entry created and field set)
409  — InvalidInterval
422  — invalid or non-v7 UUID, or malformed JSON body
500  — infrastructure error
```

---

## GraphQL Mutations

```graphql
mutation {
  setStartedAt(input: { timeEntryId: "...", userId: "...", startedAt: 1000 })
  setEndedAt(input:   { timeEntryId: "...", userId: "...", endedAt:   2000 })
}
```

Both return `Boolean` (true on success) and surface domain rejections as GraphQL errors.

---

## Verification

```bash
cargo run-script fmt
cargo run-script lint
cargo run-script test
cargo run-script coverage
```
