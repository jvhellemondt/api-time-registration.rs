# Tags Feature Design

Date: 2026-03-03

## Summary

Add tags as managed entities that can be assigned to time entries at registration time. Tags are tenant-scoped, have a name and color, and are snapshot-copied into the time entry event at registration so historical entries are unaffected by later tag deletion.

`tenant_id` is hardcoded as `"tenant-hardcoded"` in all inbound adapters (HTTP and GraphQL) for now.

## Bounded context placement

Tags get their own bounded context: `src/modules/tags/`. This is warranted because tags have their own aggregate identity, lifecycle (create/delete), and tenant scope independent of any time entry.

## Module structure

```
src/modules/tags/
  mod.rs
  core/
    events.rs
    events/
      v1/
        tag_created.rs
        tag_deleted.rs
    state.rs
    evolve.rs
    intents.rs
  use_cases/
    create_tag/
      command.rs
      decide.rs
      decision.rs
      handler.rs
      inbound/
        http.rs
        graphql.rs        ← CreateTagMutation
    delete_tag/
      command.rs
      decide.rs
      decision.rs
      handler.rs
      inbound/
        http.rs
        graphql.rs        ← DeleteTagMutation
    list_tags/
      queries.rs
      projection.rs
      projector.rs
      handler.rs
      inbound/
        http.rs
        graphql.rs        ← TagsQueryRoot
  adapters/
    outbound/
      event_store.rs
      intent_outbox.rs
      projections.rs
      projections_in_memory.rs
```

## Tag aggregate

**Events:**
- `TagCreatedV1 { tag_id, tenant_id, name, color, created_at, created_by }`
- `TagDeletedV1 { tag_id, tenant_id, deleted_at, deleted_by }`

**State:**
```rust
pub enum TagState {
    None,
    Created { tag_id, tenant_id, name, color, created_at, created_by },
    Deleted { tag_id, tenant_id, name, color },
}
```

**Deciders:**

`create_tag`:
- `TagState::None` → `Accepted { TagCreatedV1 }`
- `TagState::Created` | `TagState::Deleted` → `Rejected(TagAlreadyExists)`

`delete_tag`:
- `TagState::Created` → `Accepted { TagDeletedV1 }`
- `TagState::None` → `Rejected(TagNotFound)`
- `TagState::Deleted` → `Rejected(TagAlreadyDeleted)`

**Projection (`list_tags`):**

`HashMap<(tenant_id, tag_id), TagRow>`. Only rows with `deleted_at: None` are returned in query results.

## Shared primitive

`Tag { tag_id: String, name: String, color: String }` lives in `src/shared/core/`. It is the snapshot value embedded in time entry events and used by the `TagLookupPort`.

## Cross-module data flow

Modules never depend on each other. The lookup is bridged via a port defined by the consumer:

**`TagLookupPort`** in `time_entries/core/ports.rs`:
```rust
pub trait TagLookupPort: Send + Sync {
    async fn resolve(&self, tenant_id: &str, tag_ids: &[String])
        -> Result<Vec<Tag>, TagLookupError>;
}

pub enum TagLookupError {
    TagNotFound(String),
    Unavailable,
}
```

The concrete implementation lives in the shell (the composition root with visibility of both modules). It wraps the tags projection store.

**Registration flow:**
1. HTTP/GraphQL input arrives with `tag_ids: Vec<String>`
2. Handler calls `tag_lookup_port.resolve("tenant-hardcoded", &tag_ids)`
3. `TagLookupError::TagNotFound` → `ApplicationError::TagNotFound` → HTTP 422 / GQL error
4. Success → builds `RegisterTimeEntry { tags: Vec<Tag>, ... }`
5. Decider stores tags as opaque data into `TimeEntryRegisteredV1`

## Changes to `time_entries`

- `RegisterTimeEntry` command: `tags: Vec<Tag>` (was `Vec<String>`)
- `TimeEntryRegisteredV1` event: `tags: Vec<Tag>` (was `Vec<String>`)
- `TimeEntryState::Registered`: `tags: Vec<Tag>`
- `TimeEntryRow` / `TimeEntryView`: `tags: Vec<Tag>`
- `RegisterTimeEntryBody` (HTTP): replaces commented-out `tags` with `tag_ids: Vec<String>`
- `register_time_entry` handler: gains `tag_lookup_port: Arc<dyn TagLookupPort>`
- GraphQL `MutationRoot` renamed to `TimeEntryMutations` (see GraphQL section)
- GraphQL `QueryRoot` renamed to `TimeEntryQueries`
- GraphQL `register_time_entry` mutation: `tag_ids: Vec<String>` arg; resolves tags before dispatching command

## HTTP API

```
POST   /tags             create_tag     → 201 { tag_id }
DELETE /tags/:tag_id     delete_tag     → 204
GET    /tags             list_tags      → 200 [{ tag_id, name, color }]
```

`POST /tags` body: `{ "name": "Billable", "color": "#FF5733" }`

`tenant_id` is hardcoded as `"tenant-hardcoded"` in all handlers.

Updated `time_entries` endpoint gains `tag_ids` in the request body:
```json
{ "start_time": 1000, "end_time": 2000, "tag_ids": ["tag-uuid-1"], "description": "..." }
```

## GraphQL API

The existing `MutationRoot` / `QueryRoot` in `time_entries` are **renamed** to `TimeEntryMutations` / `TimeEntryQueries`. The shell composes them with tag roots via `MergedObject`.

### Tags mutations (`create_tag/inbound/graphql.rs` and `delete_tag/inbound/graphql.rs`)

```rust
// create_tag/inbound/graphql.rs
pub struct CreateTagMutation;

#[Object]
impl CreateTagMutation {
    async fn create_tag(
        &self,
        context: &Context<'_>,
        name: String,
        color: String,
    ) -> GqlResult<ID> { ... }  // 201 → ID; duplicate → GQL error
}

// delete_tag/inbound/graphql.rs
pub struct DeleteTagMutation;

#[Object]
impl DeleteTagMutation {
    async fn delete_tag(
        &self,
        context: &Context<'_>,
        tag_id: String,
    ) -> GqlResult<bool> { ... }  // true on success; not-found / already-deleted → GQL error
}
```

Both hardcode `tenant_id = "tenant-hardcoded"`.

### Tags query (`list_tags/inbound/graphql.rs`)

```rust
#[derive(SimpleObject, Clone)]
pub struct GqlTag {
    pub tag_id: String,
    pub name: String,
    pub color: String,
}

pub struct TagsQueryRoot;

#[Object]
impl TagsQueryRoot {
    async fn list_tags(&self, context: &Context<'_>) -> GqlResult<Vec<GqlTag>> { ... }
}
```

Hardcodes `tenant_id = "tenant-hardcoded"`.

### Updated `register_time_entry` GraphQL mutation

```rust
// register_time_entry/inbound/graphql.rs  (renamed struct)
pub struct TimeEntryMutations;

#[Object]
impl TimeEntryMutations {
    async fn register_time_entry(
        &self,
        context: &Context<'_>,
        user_id: String,
        start_time: i64,
        end_time: i64,
        tag_ids: Vec<String>,   // was `tags: Vec<String>`
        description: String,
    ) -> GqlResult<ID> { ... }  // resolves tags via tag_lookup_port before dispatching
}
```

### Shell wiring (`shell/graphql.rs`)

```rust
use async_graphql::{EmptySubscription, MergedObject, Schema};

use crate::modules::tags::use_cases::create_tag::inbound::graphql::CreateTagMutation;
use crate::modules::tags::use_cases::delete_tag::inbound::graphql::DeleteTagMutation;
use crate::modules::tags::use_cases::list_tags::inbound::graphql::TagsQueryRoot;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::inbound::graphql::TimeEntryQueries;
use crate::modules::time_entries::use_cases::register_time_entry::inbound::graphql::TimeEntryMutations;

#[derive(MergedObject, Default)]
pub struct MutationRoot(TimeEntryMutations, CreateTagMutation, DeleteTagMutation);

#[derive(MergedObject, Default)]
pub struct QueryRoot(TimeEntryQueries, TagsQueryRoot);

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;
```

### AppState expansion (`shell/state.rs`)

```rust
pub struct AppState {
    // existing
    pub queries: Arc<dyn TimeEntryQueries + Send + Sync>,
    pub register_handler: Arc<RegisterTimeEntryHandler<...>>,
    pub event_store: Arc<InMemoryEventStore<TimeEntryEvent>>,
    // new
    pub tag_create_handler: Arc<CreateTagHandler<InMemoryEventStore<TagEvent>, InMemoryDomainOutbox>>,
    pub tag_delete_handler: Arc<DeleteTagHandler<InMemoryEventStore<TagEvent>, InMemoryDomainOutbox>>,
    pub tag_queries: Arc<dyn TagQueries + Send + Sync>,
    pub tag_lookup_port: Arc<dyn TagLookupPort + Send + Sync>,
}
```

The `tag_lookup_port` concrete impl (shell-level) wraps the `tags` projection store directly, resolving `tag_ids → Vec<Tag>`.

## Testing approach

**Core (pure):**
- `decide` for `create_tag` and `delete_tag`: all state transitions
- `evolve`: correct state for each event
- Golden JSON serialisation tests for `TagCreatedV1` and `TagDeletedV1`

**Projection:**
- Created tag appears in list; deleted tag is filtered out

**Inbound HTTP adapters (in-process):**
- `POST /tags` → 201; duplicate → 409
- `DELETE /tags/:tag_id` → 204; not found → 409
- `GET /tags` → 200 with list
- `POST /register-time-entry` with valid `tag_ids` → 201
- `POST /register-time-entry` with unknown tag_id → 422

**Inbound GraphQL adapters (in-process, using `Schema::execute`):**
- `createTag` mutation → returns tag_id; duplicate → GQL error
- `deletTag` mutation → returns true; not-found → GQL error
- `listTags` query → returns list
- `registerTimeEntry` mutation with valid `tagIds` → returns time_entry_id
- `registerTimeEntry` mutation with unknown tagId → GQL error
