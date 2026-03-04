# Adding a New Module

This guide covers creating a new bounded context (vertical slice) from scratch. Use it when
you are introducing a completely new domain concept — for example `projects`, `approvals`,
or `users` — that owns its own events, state, and use cases.

For adding a use case to an existing module, see:
- [implementing-a-command-use-case.md](implementing-a-command-use-case.md) — write side
- [serving-a-query.md](serving-a-query.md) + [implementing-projections.md](implementing-projections.md) — read side

---

## What a Module Is

A module is a bounded context. It owns:
- Its domain events and intents (`core/`)
- Its aggregate state and transitions (`core/`)
- Its use cases (command and query, each with inbound adapters)
- Its outbound adapters (event store, outbox bridges)

Modules do not import from each other. If two modules need to share data, they do so through
integration events via the outbox and relay — never through direct struct imports.

---

## Folder Structure

```
src/modules/<name>/
  core/
    events.rs          ← domain event enum (all versions)
    events/
      v1/
        <event_name>.rs
    intents.rs         ← domain intent enum
    state.rs           ← aggregate state enum
    evolve.rs          ← pure evolve(state, event) → state
    projections.rs     ← pure apply(stream_id, version, event) → Vec<Mutation>
  use_cases/
    <command_use_case>/
      command.rs
      decide.rs
      decision.rs
      handler.rs
      inbound/
        http.rs
        graphql.rs
    <query_use_case>/
      projection.rs    ← SCHEMA_VERSION, State, Row, View
      projector.rs
      queries.rs
      queries_port.rs
      inbound/
        http.rs
        graphql.rs
  adapters/
    outbound/
      event_store.rs   ← placeholder comment; inject concrete impl in shell
      intent_outbox.rs ← dispatch_intents() for this module's intent enum
```

The `time_entries` module is the canonical reference — read its structure before creating a new one.

---

## Step 1 — core/events.rs

Define the domain event enum. All event types for the module live here regardless of which
use case produces them.

```rust
// core/events.rs

pub enum ProjectEvent {
    ProjectCreatedV1(v1::project_created::ProjectCreatedV1),
}
```

Each concrete event struct lives in a versioned submodule (`core/events/v1/<name>.rs`) and
derives `Clone`, `Serialize`, `Deserialize`.

---

## Step 2 — core/intents.rs

Define the domain intent enum. Intents produced by deciders go here.

```rust
// core/intents.rs

use crate::modules::projects::core::events::v1::project_created::ProjectCreatedV1;

pub enum ProjectIntent {
    PublishProjectCreated { payload: ProjectCreatedV1 },
}
```

---

## Step 3 — core/state.rs and core/evolve.rs

Define the aggregate state and the pure evolve function. See [state-and-evolve.md](state-and-evolve.md).

```rust
// core/state.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectState {
    None,
    Active { project_id: String, name: String, owner_id: String },
}

// core/evolve.rs

pub fn evolve(state: ProjectState, event: ProjectEvent) -> ProjectState {
    match (state, event) {
        (ProjectState::None, ProjectEvent::ProjectCreatedV1(e)) => {
            ProjectState::Active { project_id: e.project_id, name: e.name, owner_id: e.owner_id }
        }
        (state, _) => state,
    }
}
```

---

## Step 4 — core/projections.rs

Define the pure `apply()` function that maps domain events to read-model mutations. This is
called by the projector. See [implementing-projections.md](implementing-projections.md).

```rust
// core/projections.rs

pub enum Mutation {
    Upsert(ProjectRow),
}

pub fn apply(stream_id: &str, version: i64, event: &ProjectEvent) -> Vec<Mutation> {
    let stream_key = format!("{stream_id}:{version}");
    match event {
        ProjectEvent::ProjectCreatedV1(e) => vec![Mutation::Upsert(ProjectRow {
            project_id: e.project_id.clone(),
            name: e.name.clone(),
            owner_id: e.owner_id.clone(),
            last_event_id: Some(stream_key),
        })],
    }
}
```

---

## Step 5 — Command Use Cases

For each command use case, create the files listed in
[implementing-a-command-use-case.md](implementing-a-command-use-case.md):

```
use_cases/create_project/
  command.rs
  decide.rs
  decision.rs
  handler.rs
  inbound/
    http.rs
    graphql.rs
```

---

## Step 6 — Query Use Cases

For each query use case, create the files listed in
[serving-a-query.md](serving-a-query.md) and [implementing-projections.md](implementing-projections.md):

```
use_cases/list_projects/
  projection.rs     ← SCHEMA_VERSION, ProjectState, ProjectRow, ProjectView
  projector.rs
  queries.rs
  queries_port.rs
  inbound/
    http.rs
    graphql.rs
```

---

## Step 7 — adapters/outbound/intent_outbox.rs

Implement `dispatch_intents` for this module's intent enum. See [intents-and-outbox.md](intents-and-outbox.md).

```rust
// adapters/outbound/intent_outbox.rs

pub async fn dispatch_intents(
    outbox: &impl DomainOutbox,
    stream_id: &str,
    starting_version: i64,
    topic: &str,
    intents: Vec<ProjectIntent>,
) -> Result<(), OutboxError> {
    for (i, intent) in intents.into_iter().enumerate() {
        let stream_version = starting_version + i as i64 + 1;
        match intent {
            ProjectIntent::PublishProjectCreated { payload } => {
                outbox.enqueue(OutboxRow {
                    topic: topic.to_string(),
                    event_type: "ProjectCreated".to_string(),
                    event_version: 1,
                    stream_id: stream_id.to_string(),
                    stream_version,
                    occurred_at: payload.created_at,
                    payload: serde_json::to_value(payload).unwrap(),
                }).await?;
            }
        }
    }
    Ok(())
}
```

`adapters/outbound/event_store.rs` can remain a placeholder comment until you have a concrete
event store implementation to inject.

---

## Step 8 — lib.rs Module Declarations

Add the new module to `src/lib.rs` inside the `pub mod modules` block:

```rust
pub mod modules {
    pub mod time_entries { /* ... existing ... */ }

    pub mod projects {
        pub mod core {
            pub mod events;
            pub mod evolve;
            pub mod intents;
            pub mod projections;
            pub mod state;
        }
        pub mod use_cases {
            pub mod create_project {
                pub mod command;
                pub mod decide;
                pub mod decision;
                pub mod handler;
                pub mod inbound {
                    pub mod graphql;
                    pub mod http;
                }
            }
            pub mod list_projects {
                pub mod inbound {
                    pub mod graphql;
                    pub mod http;
                }
                pub mod projection;
                pub mod projector;
                pub mod queries;
                pub mod queries_port;
            }
        }
        pub mod adapters {
            pub mod outbound {
                pub mod event_store;
                pub mod intent_outbox;
            }
        }
    }
}
```

Each file must exist before the compiler will accept the module declaration.

---

## Step 9 — shell/state.rs

Add the new module's handler and query fields to `AppState`. Keep infrastructure type params
concrete at the shell boundary.

```rust
// shell/state.rs

pub struct AppState {
    // existing time_entries fields ...
    pub create_project_handler: Arc<CreateProjectHandler<InMemoryEventStore<ProjectEvent>, InMemoryDomainOutbox>>,
    pub project_queries: Arc<dyn ProjectQueries + Send + Sync>,
    pub project_event_store: Arc<InMemoryEventStore<ProjectEvent>>,
}
```

---

## Step 10 — shell/main.rs

Wire everything together:

```rust
// New event store + broadcast channel for the module
let (project_tx, _) = tokio::sync::broadcast::channel::<StoredEvent<ProjectEvent>>(1024);
let project_event_store = Arc::new(InMemoryEventStore::<ProjectEvent>::new_with_sender(project_tx.clone()));
let project_outbox = Arc::new(InMemoryDomainOutbox::new());

// Command handler
let create_project_handler = Arc::new(CreateProjectHandler::new(
    "projects.v1",
    project_event_store.clone(),
    project_outbox,
));

// Projector
let project_projection_store = Arc::new(InMemoryProjectionStore::<ListProjectsState>::new());
let (tech_tx, _) = broadcast::channel::<ProjectionTechnicalEvent>(256);
let projector = ListProjectsProjector::new(
    "list_projects",
    project_projection_store.clone(),
    project_event_store.clone(),
    tech_tx,
);
projector_runner::spawn(projector, project_tx.subscribe());

// Query handler
let project_queries = Arc::new(ListProjectsQueryHandler::new(project_projection_store));

// Inject into AppState
let state = AppState {
    // existing fields ...
    create_project_handler,
    project_queries,
    project_event_store,
};
```

---

## Step 11 — shell/http.rs

Register the new routes:

```rust
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        // existing routes ...
        .route("/create-project", post(create_project_http::handle))
        .route("/list-projects", get(list_projects_http::handle))
        .with_state(state)
}
```

---

## Step 12 — shell/graphql.rs

Add the new module's `QueryRoot` and `MutationRoot` to the schema. Since `async-graphql`
uses separate `#[Object]` impls, you merge them by extending the same `QueryRoot` and
`MutationRoot` types, or introduce new ones and merge with `#[MergedObject]`:

```rust
// shell/graphql.rs

use async_graphql::MergedObject;

#[derive(MergedObject, Default)]
pub struct QueryRoot(TimeEntryQueryRoot, ProjectQueryRoot);

#[derive(MergedObject, Default)]
pub struct MutationRoot(TimeEntryMutationRoot, ProjectMutationRoot);

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;
```

---

## Naming Conventions

| Concept | Convention | Example |
|---------|-----------|---------|
| Stream ID | `TypeName-{uuid}` | `Project-018f…` |
| Event name | `PastTenseV1` | `ProjectCreatedV1` |
| Intent name | `VerbNoun` | `PublishProjectCreated` |
| Module folder | `snake_case` | `projects` |
| Use case folder | `snake_case` | `create_project`, `list_projects` |
| Outbox `event_type` | `PastTense` | `ProjectCreated` |
| Topic | `<module>.v<n>` | `projects.v1` |

---

## Checklist

**Core**
- [ ] `core/events.rs` — event enum with all variants; event structs in `core/events/v1/`
- [ ] `core/intents.rs` — intent enum
- [ ] `core/state.rs` — state enum with `None` variant as starting point
- [ ] `core/evolve.rs` — pure `evolve()` function with fallback arm
- [ ] `core/projections.rs` — pure `apply()` returning `Vec<Mutation>`

**Command use cases** (per use case)
- [ ] `command.rs`, `decide.rs`, `decision.rs`, `handler.rs`
- [ ] `inbound/http.rs` and `inbound/graphql.rs`

**Query use cases** (per use case)
- [ ] `projection.rs` — `SCHEMA_VERSION`, `State`, `Row`, `View`, `From<Row> for View`
- [ ] `projector.rs`, `queries.rs`, `queries_port.rs`
- [ ] `inbound/http.rs` and `inbound/graphql.rs`

**Outbound adapters**
- [ ] `adapters/outbound/intent_outbox.rs` — `dispatch_intents` with match on intent enum

**Wiring**
- [ ] `lib.rs` — full module tree declared
- [ ] `shell/state.rs` — new fields in `AppState`
- [ ] `shell/main.rs` — stores, handlers, projectors, query handlers instantiated
- [ ] `shell/http.rs` — routes registered
- [ ] `shell/graphql.rs` — roots merged into schema

**Tests**
- [ ] All `core/` functions tested (no async, no mocks)
- [ ] All handlers and inbound adapters tested at 100% coverage
