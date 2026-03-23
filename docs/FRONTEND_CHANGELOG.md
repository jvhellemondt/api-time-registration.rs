# Frontend Changelog

Changes since commit `19e37eed` (2026-03-21 tags design addendum).

---

## [2026-03-23] Tags on Time Entries

### New endpoint: `PUT /time-entries/{id}/tags`

Assigns a set of tags to a time entry. Replaces the full tag list on every call.

**Request body:**
```json
{ "tag_ids": ["<uuid-v7>", "..."], "updated_at": 1234567890, "updated_by": "user-id" }
```

**Responses:** `200 OK` (empty body) | `404` (time entry not found or deleted) | `422` (invalid UUID v7)

### New GraphQL mutation: `setTimeEntryTags`

```graphql
mutation {
  setTimeEntryTags(timeEntryId: "<uuid-v7>", tagIds: ["<uuid-v7>"], updatedAt: 0, updatedBy: "")
}
```

Returns `Boolean`.

### `TimeEntryView` now includes `tag_ids`

The HTTP query response (`GET /time-entries`) now includes a `tag_ids: string[]` field on every entry. GraphQL `listTimeEntriesByUserId` does **not** yet expose this field (see note below).

**Rationale:** Tags are a full-replace relationship — no add/remove delta. Storing only IDs keeps the time-entry aggregate decoupled from tag content; the frontend resolves names/colors from the tags list.

---

## [2026-03-23] Client-Provided UUID v7 Required for All Time Entry Mutations

### Breaking change

The server no longer generates time entry IDs. The following endpoints have been removed:

| Removed | Replacement |
|---|---|
| `POST /time-entries/start` | `PUT /time-entries/{id}/started-at` |
| `POST /time-entries/end` | `PUT /time-entries/{id}/ended-at` |
| GraphQL `createWithStartedAt` | GraphQL `setStartedAt` |
| GraphQL `createWithEndedAt` | GraphQL `setEndedAt` |

All mutation paths now require a client-supplied UUID v7 in the path or as an argument. A non-UUID or non-v7 value is rejected with `422 Unprocessable Entity` (HTTP) or a GraphQL error.

**Rationale:** Client-generated IDs enable idempotent retries — the frontend can generate a UUID v7 and safely re-submit on network failure without risk of duplicate entries. UUID v7 is time-ordered, which aids event store sequencing and debugging.

---

## [2026-03-22] Time Entry: Field-by-Field Draft → Registered Flow

### Breaking change: `register_time_entry` removed

The monolithic `registerTimeEntry` mutation and `POST /time-entries` endpoint have been removed entirely. Time entries are now created and updated field-by-field.

### New lifecycle

A time entry starts as a **Draft** when either `started_at` or `ended_at` is first set. It automatically transitions to **Registered** once both fields are present.

### New endpoints

| Method | Path | Purpose |
|---|---|---|
| `PUT` | `/time-entries/{id}/started-at` | Set or update `started_at` |
| `PUT` | `/time-entries/{id}/ended-at` | Set or update `ended_at` |

Both accept:
```json
{ "user_id": "...", "started_at": <ms-timestamp>, "updated_at": <ms-timestamp>, "updated_by": "..." }
```

`user_id` is now part of the **request body**, not the URL.

### New GraphQL mutations

```graphql
mutation { setStartedAt(timeEntryId: "<uuid-v7>", userId: "", startedAt: 0, updatedAt: 0, updatedBy: "") }
mutation { setEndedAt(timeEntryId: "<uuid-v7>", userId: "", endedAt: 0, updatedAt: 0, updatedBy: "") }
```

Both return `Boolean`.

### Field renames

| Old | New |
|---|---|
| `start_time` | `started_at` |
| `end_time` | `ended_at` |

### New `status` field on `TimeEntryView`

Every time entry now has a `status` field: `"Draft"` or `"Registered"`. A Draft entry has one or both time fields as `null`. A Registered entry has both set.

### `deleted_at` field added

Soft-deleted entries are retained in the projection. `deleted_at` is `null` for active entries and a millisecond timestamp when deleted.

**GraphQL:** `GqlTimeEntryStatus` enum with variants `DRAFT` and `REGISTERED`. `deletedAt: Int` and `status: GqlTimeEntryStatus` are present on `GqlTimeEntry`.

**Rationale:** Separating the two fields allows partial data entry — a user can record a start time and fill in the end time later. The auto-transition to Registered means no explicit "register" action is needed; the domain handles the state change implicitly when data is complete.

---

## [2026-03-22] Tags Module

### New resource: Tags

Tags can be created, updated, and deleted independently of time entries.

### HTTP endpoints

| Method | Path | Body | Purpose |
|---|---|---|---|
| `POST` | `/tags` | `{ name, color?, description?, created_at, created_by }` | Create tag (random pastel color if `color` omitted) |
| `DELETE` | `/tags/{id}` | `{ deleted_at, deleted_by }` | Soft-delete tag |
| `PATCH` | `/tags/{id}/name` | `{ name, updated_at, updated_by }` | Rename tag |
| `PATCH` | `/tags/{id}/color` | `{ color, updated_at, updated_by }` | Change color |
| `PATCH` | `/tags/{id}/description` | `{ description, updated_at, updated_by }` | Set description |
| `GET` | `/tags` | — | List all active tags |

### GraphQL

```graphql
# Queries
query { listTags { tagId name color description } }

# Mutations
mutation { createTag(name: "", color: null, description: null, createdAt: 0, createdBy: "") }
mutation { deleteTag(tagId: "") }
mutation { setTagName(tagId: "", name: "", updatedAt: 0, updatedBy: "") }
mutation { setTagColor(tagId: "", color: "", updatedAt: 0, updatedBy: "") }
mutation { setTagDescription(tagId: "", description: "", updatedAt: 0, updatedBy: "") }
```

`createTag` returns the new `tagId: String`. All mutating operations return `Boolean`.

### `GqlTag` / `TagView` fields

| Field | Type | Notes |
|---|---|---|
| `tagId` | `String` | UUID v7 |
| `name` | `String` | |
| `color` | `String` | Hex color, e.g. `#a8d8a8` |
| `description` | `String?` | Optional |

### Deleted tags are retained in the projection

Once a tag is deleted it no longer appears in `listTags`, but the projection retains a tombstone record so that `tag_ids` on existing time entries can still be resolved to their name/color. The frontend should handle this gracefully (e.g. show tag as "deleted" or greyed-out when its ID is present on an entry but absent from the active tag list).

**Rationale:** Removing a tag should not corrupt historical time entry data. By keeping the tombstone, the frontend can always resolve a tag ID to a name even after deletion, and the domain avoids a cross-aggregate lookup at query time.
