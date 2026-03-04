# Serving a Query

This guide covers the read-side user-facing layer: the queries port, the query handler, and the
inbound HTTP and GraphQL adapters. It focuses on what callers see and how results are shaped.

For the projection infrastructure that keeps the read model up to date, see
[implementing-projections.md](implementing-projections.md).

---

## How Queries Work

```
HTTP / GraphQL inbound adapter
        │ extract params → call queries port
        ▼
Query handler (implements queries port)
        │ store.state().await?        ← reads from ProjectionStore<State>
        │ filter / sort / paginate
        ▼
Vec<View>  →  JSON / GQL response
```

The query handler reads from a **pre-built projection** — never from the event store, never
from the command side. The projection is eventually consistent; it is updated asynchronously
by the projector background task.

---

## Step 1 — The View and Row Types

These live in `use_cases/<use_case>/projection.rs` alongside the projection state.

`Row` is the full stored record including internal fields. `View` is the DTO returned to callers
— it strips anything callers should not see (e.g., `last_event_id` for cursor tracking).

```rust
// use_cases/list_time_entries_by_user/projection.rs

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimeEntryRow {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub created_by: String,
    pub updated_at: i64,
    pub updated_by: String,
    pub deleted_at: Option<i64>,
    pub last_event_id: Option<String>,  // internal — not in View
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimeEntryView {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub created_by: String,
    pub updated_at: i64,
    pub updated_by: String,
    pub deleted_at: Option<i64>,
}

impl From<TimeEntryRow> for TimeEntryView {
    fn from(row: TimeEntryRow) -> Self {
        Self {
            time_entry_id: row.time_entry_id,
            user_id: row.user_id,
            // ... map all public fields ...
        }
    }
}
```

`TimeEntryView` derives `Serialize` so the HTTP adapter can return it directly as JSON.

---

## Step 2 — The Queries Port

Create `use_cases/<use_case>/queries_port.rs`. The port is a trait. Inbound adapters depend on
this trait — not on the concrete query handler. This makes inbound adapters testable with a
stub implementation.

```rust
// use_cases/list_time_entries_by_user/queries_port.rs

use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::TimeEntryView;
use async_trait::async_trait;

#[async_trait]
pub trait TimeEntryQueries {
    async fn list_by_user_id(
        &self,
        user_id: &str,
        offset: u64,
        limit: u64,
        sort_by_start_time_desc: bool,
    ) -> anyhow::Result<Vec<TimeEntryView>>;
}
```

Naming convention: the trait name matches the use case (`MyQueries`) and the method names
describe what they return (`list_by_*`, `find_by_*`).

---

## Step 3 — The Query Handler

Create `use_cases/<use_case>/queries.rs`. The handler is generic over
`TStore: ProjectionStore<State>`. It reads state from the store, filters/sorts/paginates
in memory, and converts `Row` to `View`.

```rust
// use_cases/list_time_entries_by_user/queries.rs

pub struct ListTimeEntriesQueryHandler<TStore>
where
    TStore: ProjectionStore<ListTimeEntriesState> + Send + Sync + 'static,
{
    store: Arc<TStore>,
}

impl<TStore> ListTimeEntriesQueryHandler<TStore>
where
    TStore: ProjectionStore<ListTimeEntriesState> + Send + Sync + 'static,
{
    pub fn new(store: Arc<TStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<TStore> TimeEntryQueries for ListTimeEntriesQueryHandler<TStore>
where
    TStore: ProjectionStore<ListTimeEntriesState> + Send + Sync + 'static,
{
    async fn list_by_user_id(
        &self,
        user_id: &str,
        offset: u64,
        limit: u64,
        sort_by_start_time_desc: bool,
    ) -> anyhow::Result<Vec<TimeEntryView>> {
        let state = self.store.state().await?.unwrap_or_default();

        let mut items: Vec<_> = state
            .rows
            .iter()
            .filter(|((uid, _), _)| uid == user_id)
            .map(|(_, row)| row.clone())
            .collect();

        items.sort_by_key(|r| r.start_time);
        if sort_by_start_time_desc {
            items.reverse();
        }

        let start = offset as usize;
        let end = start.saturating_add(limit as usize).min(items.len());
        if start >= items.len() {
            return Ok(Vec::new());
        }

        Ok(items[start..end].iter().cloned().map(TimeEntryView::from).collect())
    }
}
```

**Rules for the query handler:**
- Never read from the event store.
- Never call command handlers.
- Never write anything — no events, no intents, no store updates.
- Propagate store errors with `?` — the inbound adapter maps them to 500.

---

## Step 4 — Wiring via AppState

`AppState` holds the queries as a trait object so inbound adapters stay decoupled from the
concrete implementation:

```rust
// shell/state.rs

pub struct AppState {
    pub queries: Arc<dyn TimeEntryQueries + Send + Sync>,
    // ...
}
```

In `shell/main.rs`, create the concrete handler and inject it:

```rust
let projection_store = Arc::new(InMemoryProjectionStore::<ListTimeEntriesState>::new());
let query_handler = Arc::new(ListTimeEntriesQueryHandler::new(projection_store));

let state = AppState {
    queries: query_handler,
    // ...
};
```

When you switch to a database `ProjectionStore`, only this line in `main.rs` changes — the
query handler and all inbound adapters are unaffected.

---

## Step 5 — HTTP Inbound Adapter

Create `use_cases/<use_case>/inbound/http.rs`. Use a typed `Query<Params>` struct with
`Option<T>` for optional parameters so Axum enforces required fields automatically.

```rust
// inbound/http.rs

#[derive(Deserialize)]
pub struct ListTimeEntriesParams {
    pub user_id: String,           // required — missing returns 400 BAD REQUEST
    pub offset: Option<u64>,       // defaults to 0
    pub limit: Option<u64>,        // defaults to 20
    pub sort_desc: Option<bool>,   // defaults to true
}

pub async fn handle(
    State(state): State<AppState>,
    Query(params): Query<ListTimeEntriesParams>,
) -> impl IntoResponse {
    match state.queries.list_by_user_id(
        &params.user_id,
        params.offset.unwrap_or(0),
        params.limit.unwrap_or(20),
        params.sort_desc.unwrap_or(true),
    ).await {
        Ok(entries) => Json(entries).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
```

**Error mapping:**

| Situation | HTTP status |
|-----------|-------------|
| Query succeeds | `200 OK` with JSON array |
| Store fails / handler error | `500 INTERNAL_SERVER_ERROR` |
| Required param missing (e.g. `user_id`) | `400 BAD_REQUEST` (Axum implicit) |

`TimeEntryView` derives `Serialize` so `Json(entries)` works directly — no mapping needed.

Register the route in `shell/http.rs`:

```rust
.route("/list-time-entries", get(list_http::handle))
```

---

## Step 6 — GraphQL Inbound Adapter

Create `use_cases/<use_case>/inbound/graphql.rs`. Three things are needed: a GQL type,
a `From<View>` impl, and the resolver on `QueryRoot`.

```rust
// inbound/graphql.rs

// 1. The GQL response type — derives SimpleObject so async-graphql generates the schema
#[derive(async_graphql::SimpleObject, Clone)]
pub struct GqlTimeEntry {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub created_by: String,
    pub updated_at: i64,
    pub updated_by: String,
    pub deleted_at: Option<i64>,
}

// 2. Map from the domain view to the GQL type
impl From<TimeEntryView> for GqlTimeEntry {
    fn from(v: TimeEntryView) -> Self {
        Self {
            time_entry_id: v.time_entry_id,
            user_id: v.user_id,
            // ... map fields ...
        }
    }
}

// 3. The resolver
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn list_time_entries_by_user_id(
        &self,
        context: &Context<'_>,
        user_id: String,
        offset: Option<i64>,
        limit: Option<i64>,
        sort_desc: Option<bool>,
    ) -> GqlResult<Vec<GqlTimeEntry>> {
        let state = context.data_unchecked::<AppState>();
        let list = state.queries.list_by_user_id(
            &user_id,
            offset.unwrap_or(0).max(0) as u64,
            limit.unwrap_or(20).max(0) as u64,
            sort_desc.unwrap_or(true),
        ).await?;
        Ok(list.into_iter().map(Into::into).collect())
    }
}
```

**`GqlTimeEntry` vs `TimeEntryView`:** The GQL type is a separate struct because the GraphQL
schema is a transport concern — `TimeEntryView` is a domain concern. The `From` impl is the
boundary between them. This lets the domain view evolve independently from the GQL schema.

Compose `QueryRoot` into the schema in `shell/graphql.rs`:

```rust
pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;
```

---

## Step 7 — `lib.rs` Module Declarations

```rust
pub mod list_time_entries_by_user {
    pub mod inbound {
        pub mod graphql;
        pub mod http;
    }
    pub mod projection;
    pub mod projector;
    pub mod queries;
    pub mod queries_port;
}
```

---

## Checklist

- [ ] `projection.rs` — `Row` (internal, with `last_event_id`), `View` (public), `From<Row> for View`
- [ ] `View` derives `Serialize` for JSON serialisation
- [ ] `queries_port.rs` — trait with one method per query operation
- [ ] `queries.rs` — handler implements trait, reads from `ProjectionStore<State>` only
- [ ] `queries.rs` — never reads from event store, never writes anything
- [ ] `shell/state.rs` — `Arc<dyn MyQueries + Send + Sync>` in `AppState`
- [ ] `shell/main.rs` — concrete handler constructed and injected
- [ ] `inbound/http.rs` — typed `Query<Params>` struct; required params are non-`Option`; optional params use `unwrap_or` defaults
- [ ] `inbound/http.rs` — success → 200 + JSON array; store error → 500
- [ ] `inbound/graphql.rs` — `GqlType` with `#[derive(SimpleObject)]`; `From<View> for GqlType`; resolver on `QueryRoot`
- [ ] `shell/http.rs` — route registered
- [ ] `shell/graphql.rs` — `QueryRoot` composed into schema
- [ ] `lib.rs` — module declarations
- [ ] Tests for `queries.rs` and `inbound/http.rs` at 100% coverage
- [ ] Inbound adapter tests use a stub `queries` implementation (implement the trait on a test struct) to test error paths
