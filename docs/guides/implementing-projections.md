# Implementing Projections for Queries

This guide explains how the projection stack works and walks through the steps to:

1. Add a new query use case with its own projection
2. Implement a database-backed `ProjectionStore`

---

## How It Works

A query use case is served by five co-located pieces inside `use_cases/<use_case>/`:

```
projection.rs      — State shape, Row, View, SCHEMA_VERSION
queries_port.rs    — Trait that inbound adapters depend on
queries.rs         — Query handler: reads from ProjectionStore<State>
projector.rs       — Background worker: folds events into ProjectionStore<State>
inbound/           — HTTP / GraphQL handlers
```

Plus two shared pieces:

```
core/projections.rs                        — Pure apply(event) → Vec<Mutation>
shared/infrastructure/projection_store/    — ProjectionStore<P> trait + implementations
```

### Data flow

```
Command side ──► EventStore ──► broadcast::Sender<StoredEvent<E>>
                                        │
                                        ▼
                              Projector (background task)
                                        │
                              core::projections::apply()
                                        │
                                        ▼
                              ProjectionStore<State>
                                        │
                                        ▼
                              QueryHandler ──► inbound adapter ──► caller
```

### The `ProjectionStore<P>` trait

```rust
// shared/infrastructure/projection_store/mod.rs

pub trait ProjectionStore<P: Clone + Send + Sync + 'static>: Send + Sync {
    async fn state(&self) -> anyhow::Result<Option<P>>;
    async fn checkpoint(&self) -> anyhow::Result<u64>;
    async fn schema_version(&self) -> anyhow::Result<Option<u32>>;
    async fn save(&self, state: P, checkpoint: u64) -> anyhow::Result<()>;
    async fn save_schema_version(&self, version: u32) -> anyhow::Result<()>;
    async fn clear(&self) -> anyhow::Result<()>;
}
```

`checkpoint` is a `u64` global event position. Events with `global_position < checkpoint` are skipped by the projector; the projector calls `clear()` + full replay when `schema_version` mismatches.

### Schema versioning and rebuild

`SCHEMA_VERSION` is a `u32` constant in `projection.rs`. The projector compares the stored version against this constant on startup:

- **Match** → enter the live-event loop.
- **Mismatch (or None)** → call `store.clear()`, replay all events from position 0, write the new `SCHEMA_VERSION`.

Bump `SCHEMA_VERSION` whenever the `State` shape changes in a way that requires a full replay.

### `core/projections.rs` — pure mappings

```rust
pub enum Mutation { Upsert(Row), }

pub fn apply(stream_id: &str, version: i64, event: &DomainEvent) -> Vec<Mutation> {
    match event {
        DomainEvent::SomethingHappened(e) => vec![Mutation::Upsert(Row { ... })],
    }
}
```

This is a pure function — no I/O, no `async`. It belongs in `core/` because it maps between domain events and the read-model row shape. The projector calls it; the query handler does not.

---

## Adding a New Query Use Case

### Step 1 — `projection.rs`

Define the in-memory state, the stored row, the view DTO, and the schema version.

```rust
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Default)]
pub struct MyState {
    pub rows: HashMap<String, MyRow>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct MyRow { /* all fields, including last_event_id */ }

#[derive(Clone)]
pub struct MyView { /* fields exposed to callers — drop internal ones */ }

impl From<MyRow> for MyView { ... }
```

`MyState` must derive `Clone` and `Default`. It must be `Serialize + DeserializeOwned` if you store it as JSON in a database.

### Step 2 — `queries_port.rs`

Define the trait the inbound adapter will depend on.

```rust
#[async_trait]
pub trait MyQueries {
    async fn find_by_x(&self, x: &str, ...) -> anyhow::Result<Vec<MyView>>;
}
```

### Step 3 — `queries.rs`

Implement the trait. Read from `ProjectionStore<MyState>` only — never from the event store.

```rust
pub struct MyQueryHandler<TStore>
where
    TStore: ProjectionStore<MyState> + Send + Sync + 'static,
{
    store: Arc<TStore>,
}

#[async_trait]
impl<TStore> MyQueries for MyQueryHandler<TStore>
where
    TStore: ProjectionStore<MyState> + Send + Sync + 'static,
{
    async fn find_by_x(&self, x: &str, ...) -> anyhow::Result<Vec<MyView>> {
        let state = self.store.state().await?.unwrap_or_default();
        // filter / sort / paginate from state.rows
        // return Vec<MyView>
    }
}
```

### Step 4 — `core/projections.rs`

Add or extend `apply()` to handle any new event variants that this projection cares about.

```rust
pub fn apply(stream_id: &str, version: i64, event: &DomainEvent) -> Vec<Mutation> {
    match event {
        DomainEvent::Existing(e) => { ... }
        DomainEvent::New(e)      => vec![Mutation::Upsert(MyRow { ... })],
    }
}
```

### Step 5 — `projector.rs`

Copy the pattern from `list_time_entries_by_user/projector.rs`. The projector is generic over `TStore: ProjectionStore<MyState>`.

Key responsibilities:
- `run(receiver)` — check schema version on startup; loop on the broadcast channel.
- `rebuild()` — clear state, replay all events from position 0, write new schema version.
- `apply_stored_event()` — load current state, call `core::projections::apply()`, save updated state + checkpoint.
- Emit `ProjectionTechnicalEvent`s for observability.

Note: the projector currently takes `Arc<InMemoryEventStore<E>>` directly for `rebuild()` (to replay all events). When you add a real event store, extract an `EventStore` port and depend on that instead.

### Step 6 — `lib.rs`

Add module declarations inside the use case block:

```rust
pub mod my_use_case {
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

### Step 7 — `shell/main.rs`

Wire it all together:

```rust
let projection_store = Arc::new(InMemoryProjectionStore::<MyState>::new());

let (tech_tx, _) = broadcast::channel::<ProjectionTechnicalEvent>(256);
let projector = MyProjector::new("my_use_case", projection_store.clone(), event_store.clone(), tech_tx);
let receiver = event_tx.subscribe();
projector_runner::spawn(projector, receiver);

let query_handler = Arc::new(MyQueryHandler::new(projection_store));
```

---

## Implementing a Database `ProjectionStore`

### What to store

Each projection needs one logical row, keyed by projection name:

| Column | Type | Notes |
|--------|------|-------|
| `name` | `TEXT PRIMARY KEY` | e.g. `"list_time_entries_by_user"` |
| `state` | `TEXT` / `JSONB` | `serde_json::to_string(&state)` |
| `checkpoint` | `BIGINT` | global event position |
| `schema_version` | `INT` | compared against `SCHEMA_VERSION` constant |

The projector calls `save(state, checkpoint)` on every event and `save_schema_version` once after a rebuild — these must be atomic or at least ordered correctly (write state + checkpoint together, schema version only after a successful full replay).

### Step-by-step

**1. Add the file**

```
shared/infrastructure/projection_store/postgres.rs   # or sqlite, dynamodb, etc.
```

**2. Implement the trait**

```rust
pub struct PostgresProjectionStore<P> {
    pool: sqlx::PgPool,
    name: String,
    _p: std::marker::PhantomData<P>,
}

#[async_trait]
impl<P> ProjectionStore<P> for PostgresProjectionStore<P>
where
    P: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned + Default + 'static,
{
    async fn state(&self) -> anyhow::Result<Option<P>> {
        let row = sqlx::query!("SELECT state FROM projections WHERE name = $1", self.name)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            None => Ok(None),
            Some(r) => Ok(Some(serde_json::from_str(&r.state)?)),
        }
    }

    async fn checkpoint(&self) -> anyhow::Result<u64> {
        let row = sqlx::query!("SELECT checkpoint FROM projections WHERE name = $1", self.name)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.checkpoint as u64).unwrap_or(0))
    }

    async fn schema_version(&self) -> anyhow::Result<Option<u32>> {
        let row = sqlx::query!("SELECT schema_version FROM projections WHERE name = $1", self.name)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.and_then(|r| r.schema_version.map(|v| v as u32)))
    }

    async fn save(&self, state: P, checkpoint: u64) -> anyhow::Result<()> {
        let json = serde_json::to_string(&state)?;
        sqlx::query!(
            "INSERT INTO projections (name, state, checkpoint)
             VALUES ($1, $2, $3)
             ON CONFLICT (name) DO UPDATE SET state = $2, checkpoint = $3",
            self.name,
            json,
            checkpoint as i64,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn save_schema_version(&self, version: u32) -> anyhow::Result<()> {
        sqlx::query!(
            "UPDATE projections SET schema_version = $1 WHERE name = $2",
            version as i32,
            self.name,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn clear(&self) -> anyhow::Result<()> {
        sqlx::query!(
            "UPDATE projections SET state = '{}', checkpoint = 0, schema_version = NULL
             WHERE name = $1",
            self.name,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
```

> The `State` type must implement `serde::Serialize + serde::Deserialize`. Add those derives to `MyState` and all nested types before wiring the database store.

**3. Register the module**

```rust
// shared/infrastructure/projection_store/mod.rs
pub mod postgres;
```

**4. Swap the implementation in `shell/main.rs`**

```rust
// Before:
let projection_store = Arc::new(InMemoryProjectionStore::<ListTimeEntriesState>::new());

// After:
let projection_store = Arc::new(PostgresProjectionStore::<ListTimeEntriesState>::new(pool, "list_time_entries_by_user"));
```

Nothing else changes — `ListTimeEntriesProjector` and `ListTimeEntriesQueryHandler` are generic over `TStore: ProjectionStore<State>`, so they accept the new implementation without modification.

**5. Migration**

Create the projections table before the application starts:

```sql
CREATE TABLE IF NOT EXISTS projections (
    name            TEXT PRIMARY KEY,
    state           TEXT    NOT NULL DEFAULT '{}',
    checkpoint      BIGINT  NOT NULL DEFAULT 0,
    schema_version  INT
);
```

Insert a seed row for each projection, or let `save()` upsert it on first use.

---

## Checklist for a New Query Use Case

- [ ] `projection.rs` — `SCHEMA_VERSION`, `State` (Clone + Default), `Row`, `View`, `From<Row> for View`
- [ ] `queries_port.rs` — query trait
- [ ] `queries.rs` — handler reading from `ProjectionStore<State>`
- [ ] `core/projections.rs` — `apply()` returns mutations for new event variants
- [ ] `projector.rs` — projector generic over `TStore`
- [ ] `lib.rs` — module declarations
- [ ] `shell/main.rs` — store, projector, runner, query handler wired
- [ ] Tests for all of the above (100% coverage enforced)

## Checklist for a Database `ProjectionStore`

- [ ] `State` derives `serde::Serialize + serde::Deserialize`
- [ ] Database file in `shared/infrastructure/projection_store/<technology>.rs`
- [ ] Implements all six methods: `state`, `checkpoint`, `schema_version`, `save`, `save_schema_version`, `clear`
- [ ] `save` is atomic (state + checkpoint written together)
- [ ] `schema_version` column starts as `NULL` (forces rebuild on first boot)
- [ ] Module registered in `projection_store/mod.rs`
- [ ] Swapped in `shell/main.rs` — no other files need to change
- [ ] Migration runs before the application starts
