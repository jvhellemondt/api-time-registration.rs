# Plan: Add Set Tags to Time Entry

## Context

Users need the ability to associate tags with time entries. Tags already exist as a separate domain (`src/modules/tags/`), and this feature adds the ability to set a list of tag IDs on a time entry. The tag set replaces the current tag set (full replacement semantics). Tags can be set on entries in any state (None auto-initiates, Draft, or Registered).

## Approach

Add a `set_time_entry_tags` use case following the exact same pattern as `set_started_at` / `set_ended_at`. New event `TimeEntryTagsSetV1` carries `tag_ids: Vec<String>`. Tags are stored as IDs only (no cross-aggregate validation). Setting tags does NOT trigger registration (only completing both start/end does).

---

## Files to Create

### 1. Event struct
`src/modules/time_entries/core/events/v1/time_entry_tags_set.rs`
```rust
pub struct TimeEntryTagsSetV1 {
    pub time_entry_id: String,
    pub tag_ids: Vec<String>,
    pub updated_at: i64,
    pub updated_by: String,
}
```

### 2. Use case files
- `src/modules/time_entries/use_cases/set_time_entry_tags/command.rs` — `SetTimeEntryTags { time_entry_id, user_id, tag_ids, updated_at, updated_by }`
- `src/modules/time_entries/use_cases/set_time_entry_tags/decision.rs` — `Decision` enum (Accepted/Rejected); `DecideError` enum (no variants initially — empty enum using `#[derive]` but kept for extensibility, OR just use an infallible decision pattern... looking at the pattern, all existing DecideErrors have at least InvalidInterval. For set_tags, no domain rejections exist, so DecideError can have no variants: `pub enum DecideError {}`)
- `src/modules/time_entries/use_cases/set_time_entry_tags/decide.rs` — `decide_set_time_entry_tags(state, command) -> Decision`
  - `None` → emit `TimeEntryInitiatedV1` + `TimeEntryTagsSetV1`, no intents
  - `Draft` → emit `TimeEntryTagsSetV1`, no intents
  - `Registered` → emit `TimeEntryTagsSetV1`, no intents
- `src/modules/time_entries/use_cases/set_time_entry_tags/handler.rs` — `SetTimeEntryTagsHandler<TEventStore, TOutbox>`
- `src/modules/time_entries/use_cases/set_time_entry_tags/inbound/http.rs` — `PUT /time-entries/{id}/tags`, body `{ user_id, tag_ids }`
- `src/modules/time_entries/use_cases/set_time_entry_tags/inbound/graphql.rs` — `SetTimeEntryTagsMutation`

### 3. Test fixtures
- `src/tests/fixtures/commands/set_time_entry_tags.rs` — `SetTimeEntryTagsBuilder`
- `src/tests/fixtures/commands/json/set_time_entry_tags.json` — default fixture data

---

## Files to Modify

### Core domain
| File | Change |
|------|--------|
| `src/modules/time_entries/core/events.rs` | Add `TimeEntryTagsSetV1` variant + `pub mod v1::time_entry_tags_set` |
| `src/modules/time_entries/core/state.rs` | Add `tag_ids: Vec<String>` to `Draft` and `Registered` variants |
| `src/modules/time_entries/core/evolve.rs` | Handle `TimeEntryTagsSetV1` on both `Draft` and `Registered`; `None + TagsSet` stays None (Initiated handles None) |

### Projection
| File | Change |
|------|--------|
| `src/modules/time_entries/core/projections.rs` | Add `SetTags { time_entry_id, tag_ids, updated_at, updated_by, last_event_id }` mutation; handle `TimeEntryTagsSetV1` in `apply()` |
| `src/modules/time_entries/use_cases/list_time_entries_by_user/projection.rs` | Add `tag_ids: Vec<String>` to `TimeEntryRow` and `TimeEntryView`; update `From<TimeEntryRow> for TimeEntryView` |
| `src/modules/time_entries/use_cases/list_time_entries_by_user/projector.rs` | Apply `SetTags` mutation in projector's mutation handler |

### Wiring
| File | Change |
|------|--------|
| `src/shell/state.rs` | Add `set_time_entry_tags_handler: SetTimeEntryTagsHandler<...>` field to `AppState` |
| `src/shell/http.rs` | Add route `.route("/time-entries/{id}/tags", put(set_time_entry_tags_http::handle_put))` |
| `src/shell/main.rs` | Instantiate `SetTimeEntryTagsHandler` and wire into `AppState` |
| `src/shell/graphql.rs` | Add `SetTimeEntryTagsMutation` to `MutationRoot` |

### Test fixtures
| File | Change |
|------|--------|
| `src/tests/fixtures/commands/mod.rs` | Add `pub mod set_time_entry_tags` |
| `src/tests/fixtures/tags.rs` | Add `set_time_entry_tags_handler` to `make_test_app_state()` |

---

## Decide Logic Detail

```
decide_set_time_entry_tags(state, cmd):
  match state:
    None =>
      Accepted {
        events: [TimeEntryInitiatedV1 { time_entry_id, user_id, created_at: cmd.updated_at, created_by: cmd.updated_by },
                 TimeEntryTagsSetV1 { time_entry_id, tag_ids, updated_at, updated_by }],
        intents: []
      }
    Draft { .. } =>
      Accepted {
        events: [TimeEntryTagsSetV1 { time_entry_id, tag_ids, updated_at, updated_by }],
        intents: []
      }
    Registered { .. } =>
      Accepted {
        events: [TimeEntryTagsSetV1 { time_entry_id, tag_ids, updated_at, updated_by }],
        intents: []
      }
```

No domain rejections (empty `DecideError` enum).

## Evolve changes for tag_ids in state

Both `Draft` and `Registered` gain a `tag_ids: Vec<String>` field (default `vec![]` on `TimeEntryInitiatedV1`). `TimeEntryTagsSetV1` updates `tag_ids` in both variants.

The `TimeEntryRegisteredV1` transition from `Draft` → `Registered` must preserve `tag_ids`.

## HTTP Endpoint

```
PUT /time-entries/{id}/tags
Body: { "user_id": "...", "tag_ids": ["tag-id-1", "tag-id-2"] }
Responses:
  200 OK — tags set
  422 — invalid UUID or JSON
  500 — infrastructure error
```

No 409 because there are no domain rejections.

---

## Verification

1. Run `cargo run-script fmt` — formatting passes
2. Run `cargo run-script lint` — clippy passes
3. Run `cargo run-script test` — all tests pass including new ones:
   - `set_time_entry_tags/decide.rs` unit tests (None/Draft/Registered state coverage)
   - `set_time_entry_tags/handler.rs` integration tests
   - `set_time_entry_tags/inbound/http.rs` HTTP adapter tests
4. Run `cargo run-script coverage` — 100% coverage maintained
