# Tags Feature Design — Addendum

Date: 2026-03-21
Supersedes / amends: `docs/plans/2026-03-03-tags-design.md`

## Context

The original 2026-03-03 plan introduced tags as managed entities with snapshot-copy semantics (a `Tag` shared primitive embedded in `TimeEntryRegisteredV1`), a `TagLookupPort`, and no explicit update lifecycle. After further consideration:

- Tags are **soft-deleted** and always recoverable by `tag_id` — there is no need to snapshot-copy tag data into the time entry event. Referencing by ID is sufficient and simpler.
- Because tags are referenced by ID, the `Tag` shared primitive in `src/shared/core/` and the `TagLookupPort` in `time_entries` are **no longer necessary**.
- Tags need a richer lifecycle: three additional update commands (`SetTagName`, `SetTagColor`, `SetTagDescription`) with their own events.
- Tags gain an optional `description` field.
- `color` is optional at creation time; when absent the handler picks randomly from 10 fixed pastel colors (imperative shell concern).
- External notification of tag events happens via the event relay tailing the event store (ADR-0009). No `PublishTag` intent belongs in the outbox.

---

## Changes vs the 2026-03-03 Plan

### 1. Removed: `Tag` shared primitive

`src/shared/core/` gains **no** `Tag` struct. There is no cross-module data type. Time entries reference tags by `tag_id` string only.

### 2. Removed: `TagLookupPort` in `time_entries`

`time_entries/core/ports.rs` does **not** gain a `TagLookupPort`. The registration handler passes `tag_ids: Vec<String>` directly without resolving tag data.

### 3. Removed: `PublishTag` intent

`tags/core/intents.rs` is either empty or absent. External consumers observe tag events via the event relay (ADR-0009) tailing the tags event store, not via an outbox intent.

### 4. Updated: Tag aggregate events

**Existing events (amended):**
```rust
TagCreatedV1 { tag_id, tenant_id, name, color, description: Option<String>, created_at, created_by }
TagDeletedV1 { tag_id, tenant_id, deleted_at, deleted_by }  // unchanged
```

**New events:**
```rust
TagNameSetV1        { tag_id, tenant_id, name,                        set_at, set_by }
TagColorSetV1       { tag_id, tenant_id, color,                       set_at, set_by }
TagDescriptionSetV1 { tag_id, tenant_id, description: Option<String>, set_at, set_by }
```

`TagDescriptionSetV1.description` is `Option<String>` to support clearing the description.

### 5. Updated: Tag state

```rust
pub enum TagState {
    None,
    Created {
        tag_id, tenant_id, name, color,
        description: Option<String>,
        created_at, created_by,
    },
    Deleted {
        tag_id, tenant_id, name, color,
        description: Option<String>,
    },
}
```

`evolve` gains match arms for `TagNameSetV1`, `TagColorSetV1`, `TagDescriptionSetV1` — each updates the relevant field in the `Created` variant. Receiving these events in `None` or `Deleted` state is a panic (illegal — events sourced from a valid stream).

### 6. New: Three update deciders

`set_tag_name`:
- `TagState::Created` → `Accepted { TagNameSetV1 }`
- `TagState::None` → `Rejected(TagNotFound)`
- `TagState::Deleted` → `Rejected(TagDeleted)`

`set_tag_color`:
- `TagState::Created` → `Accepted { TagColorSetV1 }`
- `TagState::None` → `Rejected(TagNotFound)`
- `TagState::Deleted` → `Rejected(TagDeleted)`

`set_tag_description`:
- `TagState::Created` → `Accepted { TagDescriptionSetV1 }`
- `TagState::None` → `Rejected(TagNotFound)`
- `TagState::Deleted` → `Rejected(TagDeleted)`

### 7. Updated: Color at creation

`CreateTag` command: `color: Option<String>`.

`create_tag` handler (imperative shell): if `color` is `None`, pick randomly from the following 10 fixed pastel colors before building the command passed to the decider. The decider always receives a `String` color.

```rust
const PASTEL_COLORS: [&str; 10] = [
    "#FFB3BA", // pastel pink
    "#FFDFBA", // pastel orange
    "#FFFFBA", // pastel yellow
    "#BAFFC9", // pastel green
    "#BAE1FF", // pastel blue
    "#D4BAFF", // pastel purple
    "#FFBAF3", // pastel magenta
    "#BAF7FF", // pastel cyan
    "#FFC8BA", // pastel coral
    "#BAFFED", // pastel mint
];
```

Selection uses `rand::random::<usize>() % 10`. The `rand` crate is already a likely dependency; if not, use `std`-based modulo on a seeded value or add `rand` as a dependency.

### 8. Updated: Module structure (additions only)

```
src/modules/tags/
  core/
    events/
      v1/
        tag_name_set.rs         ← new
        tag_color_set.rs        ← new
        tag_description_set.rs  ← new
  use_cases/
    set_tag_name/               ← new
      command.rs
      decide.rs
      decision.rs
      handler.rs
      inbound/
        http.rs
        graphql.rs
    set_tag_color/              ← new
      command.rs
      decide.rs
      decision.rs
      handler.rs
      inbound/
        http.rs
        graphql.rs
    set_tag_description/        ← new
      command.rs
      decide.rs
      decision.rs
      handler.rs
      inbound/
        http.rs
        graphql.rs
```

### 9. Updated: Time entries — no snapshot, just tag IDs

All `Vec<Tag>` references in the original plan become `Vec<String>` (tag IDs):

| Location | Original plan | This plan |
|---|---|---|
| `RegisterTimeEntry` command | `tags: Vec<Tag>` | `tag_ids: Vec<String>` |
| `TimeEntryRegisteredV1` event | `tags: Vec<Tag>` | `tag_ids: Vec<String>` |
| `TimeEntryState::Registered` | `tags: Vec<Tag>` | `tag_ids: Vec<String>` |
| `TimeEntryRow` | `tags: Vec<Tag>` | `tag_ids: Vec<String>` |
| `TimeEntryView` | `tags: Vec<Tag>` | `tag_ids: Vec<String>` |
| Handler | resolves tags via port | passes `tag_ids` directly |

`register_time_entry` handler gains **no** `tag_lookup_port`. It accepts `tag_ids: Vec<String>` from the inbound adapter and passes them straight to the command.

### 10. Updated: HTTP API

Additions to the original plan:

```
PATCH /tags/:tag_id/name         set_tag_name         → 204
PATCH /tags/:tag_id/color        set_tag_color        → 204
PATCH /tags/:tag_id/description  set_tag_description  → 204
```

Body schemas:
```json
// PATCH /tags/:tag_id/name
{ "name": "Billable" }

// PATCH /tags/:tag_id/color
{ "color": "#FF5733" }

// PATCH /tags/:tag_id/description
{ "description": "All client-billable work" }   // null or absent → clears description
```

`POST /tags` body gains optional fields:
```json
{ "name": "Billable", "color": "#FF5733", "description": "Optional context" }
// color omitted → random pastel picked by handler
```

`GET /tags` response gains `description` field:
```json
[{ "tag_id": "...", "name": "Billable", "color": "#FFB3BA", "description": null }]
```

### 11. Updated: GraphQL API

`GqlTag` gains `description: Option<String>`.

`create_tag` mutation gains optional `color: Option<String>` and `description: Option<String>` arguments.

New mutations (one struct per use case per ADR-0001):
```rust
pub struct SetTagNameMutation;
pub struct SetTagColorMutation;
pub struct SetTagDescriptionMutation;
```

Each returns `bool` (true on success); `TagNotFound` / `TagDeleted` → GQL error.

`TimeEntryMutations.register_time_entry` gains `tag_ids: Vec<String>` (IDs only; no resolution).

Shell `MutationRoot` expands:
```rust
#[derive(MergedObject, Default)]
pub struct MutationRoot(
    TimeEntryMutations,
    CreateTagMutation,
    DeleteTagMutation,
    SetTagNameMutation,
    SetTagColorMutation,
    SetTagDescriptionMutation,
);
```

### 12. Updated: AppState

No `tag_lookup_port`. Add three new handlers:
```rust
pub tag_set_name_handler: Arc<SetTagNameHandler<...>>,
pub tag_set_color_handler: Arc<SetTagColorHandler<...>>,
pub tag_set_description_handler: Arc<SetTagDescriptionHandler<...>>,
```

---

## Files affected

### New files (tags module)
- `src/modules/tags/core/events/v1/tag_name_set.rs`
- `src/modules/tags/core/events/v1/tag_color_set.rs`
- `src/modules/tags/core/events/v1/tag_description_set.rs`
- `src/modules/tags/use_cases/set_tag_name/{command,decide,decision,handler}.rs`
- `src/modules/tags/use_cases/set_tag_name/inbound/{http,graphql}.rs`
- `src/modules/tags/use_cases/set_tag_color/{command,decide,decision,handler}.rs`
- `src/modules/tags/use_cases/set_tag_color/inbound/{http,graphql}.rs`
- `src/modules/tags/use_cases/set_tag_description/{command,decide,decision,handler}.rs`
- `src/modules/tags/use_cases/set_tag_description/inbound/{http,graphql}.rs`

### Modified files (tags module)
- `src/modules/tags/core/events/v1/tag_created.rs` — add `description: Option<String>`
- `src/modules/tags/core/events.rs` — register new event variants
- `src/modules/tags/core/state.rs` — add `description` field to `Created` and `Deleted`; add `TagDeleted` rejection variant
- `src/modules/tags/core/evolve.rs` — add arms for 3 new events
- `src/modules/tags/use_cases/create_tag/command.rs` — `color: Option<String>`, add `description: Option<String>`
- `src/modules/tags/use_cases/create_tag/handler.rs` — pastel color selection when `color` is `None`
- `src/modules/tags/use_cases/create_tag/inbound/http.rs` — optional color + description in request body
- `src/modules/tags/use_cases/create_tag/inbound/graphql.rs` — optional color + description args
- `src/modules/tags/use_cases/list_tags/projection.rs` — add `description` to `TagRow`
- `src/modules/tags/use_cases/list_tags/inbound/graphql.rs` — `GqlTag.description`

### Modified files (time_entries module)
- `src/modules/time_entries/core/events/v1/time_entry_registered.rs` — field name `tag_ids` (currently `tags: Vec<String>`)
- `src/modules/time_entries/use_cases/register_time_entry/command.rs` — field name `tag_ids: Vec<String>`
- `src/modules/time_entries/use_cases/register_time_entry/handler.rs` — no lookup port
- `src/modules/time_entries/use_cases/register_time_entry/inbound/http.rs` — `tag_ids` in request body
- `src/modules/time_entries/use_cases/register_time_entry/inbound/graphql.rs` — `tag_ids` arg
- `src/modules/time_entries/use_cases/list_time_entries_by_user/projection.rs` — field name `tag_ids`

### NOT created
- `src/shared/core/` — no `Tag` struct
- `src/modules/time_entries/core/ports.rs` — no `TagLookupPort`
- `src/modules/tags/core/intents.rs` — not created (no intents needed)

### Shell
- `src/shell/state.rs` — three new handler fields; no `tag_lookup_port`
- `src/shell/graphql.rs` — merge three new mutation structs into `MutationRoot`

---

## Testing additions

**Core (pure):**
- `decide` for `set_tag_name`, `set_tag_color`, `set_tag_description`: all state transitions (Created → Accepted; None → Rejected(TagNotFound); Deleted → Rejected(TagDeleted))
- `evolve`: correct state update for each new event; `description` field updates correctly including `None`
- Golden JSON serialisation for `TagNameSetV1`, `TagColorSetV1`, `TagDescriptionSetV1`
- `TagCreatedV1` golden JSON updated with `description` field

**Projection:**
- Created tag appears in list with `description: None`; tag with description shows it
- After `TagNameSetV1` → projection reflects new name
- After `TagColorSetV1` → projection reflects new color
- After `TagDescriptionSetV1` with value → description set; with `None` → description cleared

**Inbound HTTP:**
- `POST /tags` without color → 201, color is one of the 10 defined pastel hex strings
- `POST /tags` with explicit color → 201, that color is used
- `POST /tags` with description → 201, description stored
- `PATCH /tags/:tag_id/name` → 204; unknown tag_id → 422; deleted tag → 422
- `PATCH /tags/:tag_id/color` → 204
- `PATCH /tags/:tag_id/description` with value → 204; with `null` → 204, clears description
- `POST /register-time-entry` with `tag_ids: []` → 201
- `POST /register-time-entry` with `tag_ids: ["any-id"]` → 201 (no validation; IDs passed through)

**Inbound GraphQL:**
- `createTag` with no color → returns tag_id; `listTags` shows color is one of the 10 pastels
- `createTag` with explicit color → that color used
- `setTagName`, `setTagColor`, `setTagDescription` mutations → success and error paths
- `registerTimeEntry` with `tagIds` → returns time_entry_id (no tag resolution)
