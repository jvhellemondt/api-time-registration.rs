# Plan: Keep Deleted Tags in Projection

## Context

Deleted tags are currently removed from the `list_tags` projection via `Mutation::Delete`, which calls `state.rows.remove(&tag_id)`. This means once a tag is deleted, it disappears from the projection entirely and cannot be resolved by reference (e.g. when a time entry references a deleted tag). The fix is to keep deleted tags in the projection, marking them with a `deleted: bool` flag instead of removing them.

## Changes

### 1. `src/modules/tags/use_cases/list_tags/projection.rs`

- Bump `SCHEMA_VERSION` from `1` to `2` (triggers projection rebuild on deploy)
- Add `deleted: bool` to `TagRow`
- Add `deleted: bool` to `TagView`
- Update `From<TagRow> for TagView` to include `deleted`
- Update tests:
  - `make_row()` → add `deleted: false`
  - `it_should_convert_row_to_view` → assert `view.deleted == false`

### 2. `src/modules/tags/core/projections.rs`

- Replace `Mutation::Delete(String)` with:
  ```rust
  Mutation::MarkDeleted {
      tag_id: String,
      deleted_at: i64,
      deleted_by: String,
      last_event_id: String,
  }
  ```
- Update `apply()` for `TagEvent::TagDeletedV1(e)` to emit `Mutation::MarkDeleted { ... }` with fields from the event
- Update test `it_applies_tag_deleted` to match `Mutation::MarkDeleted { .. }`

### 3. `src/modules/tags/use_cases/list_tags/projector.rs`

- In `apply_stored_event`, replace `Mutation::Delete(tag_id) => { state.rows.remove(&tag_id); }` with:
  ```rust
  Mutation::MarkDeleted { tag_id, deleted_at: _, deleted_by: _, last_event_id } => {
      if let Some(row) = state.rows.get_mut(&tag_id) {
          row.deleted = true;
          row.last_event_id = Some(last_event_id);
      }
  }
  ```
- Update affected tests:
  - `it_should_apply_set_name_set_color_set_description_and_delete_mutations`: change assertion from `assert!(!state.rows.contains_key("t2"))` to `assert!(state.rows.get("t2").unwrap().deleted)`; also update `make_row` calls to include `deleted: false` where `TagRow` is constructed manually
  - `it_should_skip_set_mutations_when_tag_row_missing`: this test deletes t1 then asserts `!state.rows.contains_key("t1")`. With the new behavior t1 remains in rows as deleted, so update assertion to `assert!(state.rows.get("t1").unwrap().deleted)`. Rename test to `it_should_mark_deleted_tag_as_deleted_and_keep_in_projection`

### 4. `src/modules/tags/use_cases/list_tags/queries.rs`

- No change to `list_all()` — it already returns all rows from the HashMap. Deleted tags will now be included automatically with `deleted: true`.
- Update `make_row()` in tests to add `deleted: false`

## No changes needed

- `core/state.rs`, `core/evolve.rs`, `core/events/` — domain layer is correct as-is
- HTTP handler — returns whatever `list_all()` returns; downstream consumers see `deleted: bool` in `TagView`

## Verification

```bash
cargo run-script fmt-fix
cargo run-script lint
cargo run-script test
cargo run-script coverage
```
