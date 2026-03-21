# Design: Draft Time Entry — Field-by-Field Registration

**Date:** 2026-03-21
**Status:** Approved

## Overview

Split time entry registration into a real-time, field-by-field flow. Each field submitted by the frontend emits its own event. The time entry begins in a `Draft` state and transitions to `Registered` automatically once all required fields (`started_at` and `ended_at`) are present. This replaces the single `RegisterTimeEntry` all-at-once command.

Also renames `start_time → started_at` and `end_time → ended_at` throughout the codebase.

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
  ├── TimeEntryRegisteredV1     (finalization — emitted alongside the completing field event)
  │     time_entry_id, occurred_at
  └── TimeEntryDeletedV1        (soft delete — draft or registered)
        time_entry_id, deleted_at, deleted_by
```

### Event sequences

| Scenario | Events emitted |
|---|---|
| First field is start | `[TimeEntryInitiatedV1, TimeEntryStartSetV1]` |
| First field is end | `[TimeEntryInitiatedV1, TimeEntryEndSetV1]` |
| Second required field (finalizes) | `[TimeEntry{Start\|End}SetV1, TimeEntryRegisteredV1]` |
| Update field in draft | `[TimeEntry{Start\|End}SetV1]` |
| Soft delete | `[TimeEntryDeletedV1]` |

`TimeEntryInitiatedV1` is emitted exactly once per time entry — on the first field submission. The backend generates `time_entry_id` at this point.

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
    },
}
```

`Draft` holds both field values as `Option` since either field may arrive first. `Registered` holds both as concrete values, guaranteed by finalization logic. Soft delete lives on `Registered`.

---

## Use Cases

### `set_start_time`

**Command:** `SetStartTime { time_entry_id: Option<String>, user_id, started_at, occurred_at, occurred_by }`

`time_entry_id` is `None` when state is `None` (backend generates it); present otherwise.

**Decider:**

| State | Emitted events | Rejection |
|---|---|---|
| `None` | `[TimeEntryInitiatedV1, TimeEntryStartSetV1]` | — |
| `Draft { ended_at: None }` | `[TimeEntryStartSetV1]` | — |
| `Draft { ended_at: Some(_) }` | `[TimeEntryStartSetV1, TimeEntryRegisteredV1]` | if `started_at >= ended_at`: `InvalidInterval` |
| `Registered` | — | `AlreadyRegistered` |

### `set_end_time`

**Command:** `SetEndTime { time_entry_id: Option<String>, user_id, ended_at, occurred_at, occurred_by }`

**Decider:**

| State | Emitted events | Rejection |
|---|---|---|
| `None` | `[TimeEntryInitiatedV1, TimeEntryEndSetV1]` | — |
| `Draft { started_at: None }` | `[TimeEntryEndSetV1]` | — |
| `Draft { started_at: Some(_) }` | `[TimeEntryEndSetV1, TimeEntryRegisteredV1]` | if `started_at >= ended_at`: `InvalidInterval` |
| `Registered` | — | `AlreadyRegistered` |

Interval validation (`started_at < ended_at`) is only enforced at finalization, when both values are known.

---

## Projection: `list_time_entries_by_user`

`TimeEntryRow` changes:

- `start_time: i64` → `started_at: Option<i64>`
- `end_time: i64` → `ended_at: Option<i64>`
- Add `status: TimeEntryStatus` (`Draft` | `Registered`)

Projection mappings:

| Event | Mutation |
|---|---|
| `TimeEntryInitiatedV1` | `Upsert` row with `status: Draft`, `started_at: None`, `ended_at: None` |
| `TimeEntryStartSetV1` | Update `started_at`, `updated_at`, `updated_by` |
| `TimeEntryEndSetV1` | Update `ended_at`, `updated_at`, `updated_by` |
| `TimeEntryRegisteredV1` | Update `status: Registered` |
| `TimeEntryDeletedV1` | Update `deleted_at` |

---

## HTTP Adapters

### Create draft (first field)

```
POST /time-entries/start
Body: { user_id, started_at }
201:  { time_entry_id }

POST /time-entries/end
Body: { user_id, ended_at }
201:  { time_entry_id }
```

### Update field on existing draft

```
PUT /time-entries/{id}/start
Body: { user_id, started_at }
200:  (empty)

PUT /time-entries/{id}/end
Body: { user_id, ended_at }
200:  (empty)
```

The client knows finalization has occurred when it sends the last required field (it tracks which fields are set). The backend does not need to signal finalization explicitly in the response.

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

## Out of Scope

- Updating fields after finalization (`Registered` state)
- Soft-deleting a draft entry
- `tags` and `description` fields (added as separate use cases later, same pattern)
