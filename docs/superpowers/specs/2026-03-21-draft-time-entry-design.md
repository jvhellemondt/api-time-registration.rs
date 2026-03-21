# Design: Draft Time Entry — Field-by-Field Registration

**Date:** 2026-03-21
**Status:** Approved

## Overview

Split time entry registration into a real-time, field-by-field flow. Each field submitted by the frontend emits its own event. The time entry begins in a `Draft` state and transitions to `Registered` automatically once all required fields (`started_at` and `ended_at`) are present.

This **retires the existing `register_time_entry` use case** and its single all-at-once command. The existing `TimeEntryRegisteredV1` event struct is repurposed with a new, slimmer payload (finalization marker only). The existing file is updated in-place; no rename is needed.

Also renames `start_time → started_at` and `end_time → ended_at` throughout the codebase.

`user_id` moves from query parameter to request body across all new endpoints (intentional alignment with REST conventions).

---

## Events

```
TimeEntryEvent
  ├── TimeEntryInitiatedV1      (draft created — identity only, no field data)
  │     time_entry_id, user_id, created_at, created_by
  ├── TimeEntryStartSetV1       (sets/updates started_at)
  │     time_entry_id, started_at, updated_at, updated_by
  ├── TimeEntryEndSetV1         (sets/updates ended_at)
  │     time_entry_id, ended_at, updated_at, updated_by
  ├── TimeEntryRegisteredV1     (finalization marker — emitted alongside the completing field event)
  │     time_entry_id, occurred_at
  └── TimeEntryDeletedV1        (soft delete — registered entries only; decider out of scope here)
        time_entry_id, deleted_at, deleted_by
```

**Field mapping for all events:**
- `created_at = command.occurred_at`, `created_by = command.occurred_by` (in `TimeEntryInitiatedV1`)
- `updated_at = command.occurred_at`, `updated_by = command.occurred_by` (in `TimeEntryStartSetV1` and `TimeEntryEndSetV1`)

### Event sequences

| Scenario | Events emitted |
|---|---|
| First field is start | `[TimeEntryInitiatedV1, TimeEntryStartSetV1]` |
| First field is end | `[TimeEntryInitiatedV1, TimeEntryEndSetV1]` |
| Second required field (finalizes) | `[TimeEntry{Start\|End}SetV1, TimeEntryRegisteredV1]` |
| Update field in draft (including re-submission of same value) | `[TimeEntry{Start\|End}SetV1]` |
| Soft delete (registered only) | `[TimeEntryDeletedV1]` |

`TimeEntryInitiatedV1` is emitted exactly once per time entry — on the first field submission. The HTTP adapter generates `time_entry_id` before invoking the handler (consistent with current practice), so `time_entry_id` is always present on commands. Re-submitting a field with the same value always emits the event — no deduplication at the domain layer.

---

## State

```rust
pub enum TimeEntryState {
    None,
    Draft {
        time_entry_id: String,
        user_id: String,
        started_at: Option<i64>,
        ended_at: Option<i64>,
        created_at: i64,
        created_by: String,
        updated_at: i64,
        updated_by: String,
        last_event_id: Box<Option<String>>, // Box used to keep enum variant sizes balanced
    },
    Registered {
        time_entry_id: String,
        user_id: String,
        started_at: i64,
        ended_at: i64,
        created_at: i64,
        created_by: String,
        updated_at: i64,
        updated_by: String,
        deleted_at: Option<i64>,
        last_event_id: Box<Option<String>>, // Box used to keep enum variant sizes balanced
    },
}
```

### Evolve notes

**`TimeEntryInitiatedV1` (None → Draft):**
- `updated_at = created_at`, `updated_by = created_by`
- `started_at = None`, `ended_at = None`
- `last_event_id = Box::new(None)` — this value is transient; when the decider emits `[TimeEntryInitiatedV1, TimeEntryStartSetV1]` atomically, the fold immediately processes `TimeEntryStartSetV1` next and overwrites `last_event_id`. The `None` value is never observable outside the fold.

**`TimeEntryStartSetV1` / `TimeEntryEndSetV1` (Draft → Draft):**
- Update the respective field (`started_at` or `ended_at`)
- Update `updated_at`, `updated_by`, `last_event_id`
- These arms must match on `TimeEntryState::Draft { .. }` and carry all existing fields forward; only the updated field and tracking fields change.

**`TimeEntryRegisteredV1` (Draft → Registered):**
- All fields are carried over from `Draft` state
- `started_at` and `ended_at` are unwrapped from `Option` to concrete values (guaranteed non-`None` by the decider)
- `updated_at`, `updated_by`, and `last_event_id` are already up-to-date from the field-set event applied immediately before this one in the same atomic decision
- `TimeEntryRegisteredV1.occurred_at` is not used for state mutation
- `deleted_at = None`

**`TimeEntryDeletedV1` (Registered → Registered):**
- Stub only; full decider is out of scope. `evolve` sets `deleted_at`. `Draft` state falls through to the catch-all arm (no change) until the delete use case is implemented.

---

## Errors

`DecideError` variants for the new use cases:

| Variant | Meaning |
|---|---|
| `AlreadyRegistered` | A field was submitted on a `Registered` entry — new variant, distinct from `AlreadyExists` |
| `InvalidInterval` | `started_at >= ended_at` at finalization |

`AlreadyExists` is retained until `register_time_entry` is fully removed.

---

## Use Cases

### `set_start_time`

**Command:** `SetStartTime { time_entry_id: String, user_id: String, started_at: i64, occurred_at: i64, occurred_by: String }`

The HTTP adapter generates `time_entry_id` (UUID) before calling the handler. The handler passes `stream_id = time_entry_id` to the event store. Concurrent requests for the same stream ID that both see `None` state will race; the event store's optimistic concurrency control rejects the second with a version conflict — the HTTP adapter surfaces this as `409 Conflict`.

**Decider** — `InvalidInterval` is a pre-emission guard: when it fires, the decision is `Rejected` and no events are emitted.

| State | Guard | Emitted events | Rejection |
|---|---|---|---|
| `None` | — | `[TimeEntryInitiatedV1, TimeEntryStartSetV1]` | — |
| `Draft { ended_at: None }` | — | `[TimeEntryStartSetV1]` | — |
| `Draft { ended_at: Some(e) }` | `started_at >= e` | — | `InvalidInterval` |
| `Draft { ended_at: Some(_) }` | `started_at < ended_at` | `[TimeEntryStartSetV1, TimeEntryRegisteredV1]` | — |
| `Registered` | — | — | `AlreadyRegistered` |

**Intents:**

| Decision | Intents |
|---|---|
| `None → Draft` | none |
| `Draft → Draft` | none |
| `Draft → Registered` | `PublishTimeEntryRegistered { payload: RegisteredPayload }` |
| `Rejected` | `InformCallerOfRejection` (cross-cutting policy, added by handler) |

### `set_end_time`

**Command:** `SetEndTime { time_entry_id: String, user_id: String, ended_at: i64, occurred_at: i64, occurred_by: String }`

Same ID generation and concurrency handling as `set_start_time`.

**Decider:**

| State | Guard | Emitted events | Rejection |
|---|---|---|---|
| `None` | — | `[TimeEntryInitiatedV1, TimeEntryEndSetV1]` | — |
| `Draft { started_at: None }` | — | `[TimeEntryEndSetV1]` | — |
| `Draft { started_at: Some(s) }` | `s >= ended_at` | — | `InvalidInterval` |
| `Draft { started_at: Some(_) }` | `started_at < ended_at` | `[TimeEntryEndSetV1, TimeEntryRegisteredV1]` | — |
| `Registered` | — | — | `AlreadyRegistered` |

**Intents:** same pattern as `set_start_time`.

### `PublishTimeEntryRegistered` intent payload

`TimeEntryRegisteredV1` carries only `time_entry_id` and `occurred_at` — insufficient for downstream consumers. The decider builds the intent payload from draft state at decision time. The `TimeEntryIntent` variant changes its inner type from the old `TimeEntryRegisteredV1` struct to a new `RegisteredPayload`:

```rust
// New intent type — replaces the old TimeEntryRegisteredV1 payload
pub struct RegisteredPayload {
    pub time_entry_id: String,
    pub user_id: String,
    pub started_at: i64,
    pub ended_at: i64,
    pub created_at: i64,
    pub created_by: String,
    pub occurred_at: i64,    // = command.occurred_at
    pub occurred_by: String, // = command.occurred_by
}

pub enum TimeEntryIntent {
    PublishTimeEntryRegistered { payload: RegisteredPayload }, // inner type changed
}
```

---

## Projection: `list_time_entries_by_user`

Draft entries **are included** in query results so the UI can resume partial entries. `status` distinguishes them.

### Mutation variants

```rust
pub enum Mutation {
    Upsert(TimeEntryRow),   // insert or replace full row
    Update(TimeEntryPatch), // partial update — only Some fields are modified
}

pub struct TimeEntryPatch {
    pub time_entry_id: String,            // key — always present
    pub started_at: Option<Option<i64>>,  // Some(v) = set field; None = leave unchanged
    pub ended_at: Option<Option<i64>>,
    pub status: Option<TimeEntryStatus>,
    pub updated_at: Option<i64>,
    pub updated_by: Option<String>,
    pub deleted_at: Option<Option<i64>>,
    pub last_event_id: Option<Option<String>>,
}
```

If an `Update` arrives for a `time_entry_id` with no existing row, it is a no-op (event replay safety).

The projection row's `last_event_id` is set from `stream_key` (formatted as `stream_id:version`) for all events. This is distinct from the domain state's `last_event_id`, which starts as `None` after `TimeEntryInitiatedV1` and is updated only by field-set events. The two serve different purposes: the projection tracks cursor position for the projector; the state tracks idempotency position for the command handler. The `None` in state after `TimeEntryInitiatedV1` is immediately overwritten by the following field-set event in the same fold — it is never observable at rest.

### `TimeEntryRow` changes

- `start_time: i64` → `started_at: Option<i64>`
- `end_time: i64` → `ended_at: Option<i64>`
- Add `status: TimeEntryStatus` — `Draft | Registered`
- `last_event_id: Option<String>` retained

### `TimeEntryView` changes

- `start_time: i64` → `started_at: Option<i64>`
- `end_time: i64` → `ended_at: Option<i64>`
- Add `status: TimeEntryStatus`

The `From<TimeEntryRow>` impl is updated accordingly. Draft entries with `None` fields are serialized as `null` in the API response.

### Projection mappings

| Event | Mutation |
|---|---|
| `TimeEntryInitiatedV1` | `Upsert` — `status: Draft`, `started_at: None`, `ended_at: None`, `updated_at = created_at`, `updated_by = created_by`, `last_event_id = Some(stream_key)` |
| `TimeEntryStartSetV1` | `Update` — set `started_at`, `updated_at`, `updated_by`, `last_event_id` |
| `TimeEntryEndSetV1` | `Update` — set `ended_at`, `updated_at`, `updated_by`, `last_event_id` |
| `TimeEntryRegisteredV1` | `Update` — set `status: Registered`, `last_event_id` |
| `TimeEntryDeletedV1` | `Update` — set `deleted_at`, `last_event_id` |

---

## HTTP Adapters

`user_id` moves to the request body across all new endpoints (aligns with REST conventions; the existing register endpoint used a query parameter).

### Create draft (first field)

```
POST /time-entries/start
Body: { user_id, started_at }
201:  { time_entry_id }
409:  version conflict (concurrent first-field submission for same ID; client should retry)

POST /time-entries/end
Body: { user_id, ended_at }
201:  { time_entry_id }
409:  version conflict
```

The adapter generates `time_entry_id` before calling the handler and returns it in the `201` response.

### Update field on existing draft

```
PUT /time-entries/{id}/start
Body: { user_id, started_at }
200:  (empty)
409:  version conflict

PUT /time-entries/{id}/end
Body: { user_id, ended_at }
200:  (empty)
409:  version conflict
```

The client knows finalization has occurred when it sends the last required field (it tracks which fields are set). The backend does not signal finalization explicitly in the response.

---

## Rename

`start_time → started_at` and `end_time → ended_at` applied throughout:

- Event structs (`TimeEntryInitiatedV1`, new events)
- `TimeEntryState` variants
- Commands and their builders/fixtures
- `TimeEntryRow` / `TimeEntryView`
- `evolve`, `decide_*`, `projections` function bodies
- HTTP request/response shapes
- Golden JSON fixture: `src/tests/fixtures/events/json/initiated_event_v1.json`

---

## Retired

The following are removed as part of this change:

- `register_time_entry` use case (command, handler, decide, HTTP adapter, GraphQL adapter)
- The existing `TimeEntryRegisteredV1` payload struct is replaced in-place with the new slim shape (`time_entry_id`, `occurred_at`)
- The existing `TimeEntryIntent::PublishTimeEntryRegistered { payload: TimeEntryRegisteredV1 }` — inner type becomes `RegisteredPayload`

All tests, fixtures, and golden files referencing the old `RegisterTimeEntry` command or old `TimeEntryRegisteredV1` payload are updated or removed.

---

## Out of Scope

- Updating fields after finalization (`Registered` state)
- Soft-deleting a draft entry (the `TimeEntryDeletedV1` decider will reject `Draft` state — added with the delete use case)
- `tags` and `description` fields (added as separate use cases later, same pattern)