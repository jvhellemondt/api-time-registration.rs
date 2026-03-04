# Implementing a Command Use Case

This guide walks through the full write-side pipeline using `register_time_entry` as the
reference implementation. A command use case changes system state: it validates intent, produces
domain events, and queues intents for at-least-once delivery.

---

## How It Works

```
Inbound adapter (HTTP / GraphQL)
        │ builds Command struct
        ▼
Command handler
        │ load event stream from EventStore
        │ fold events → current State via evolve()
        │ call pure decide() function
        ├─ Decision::Accepted → append events + enqueue intents (atomic)
        └─ Decision::Rejected → return typed error (no side effects)
```

The handler is the only imperative piece. Everything inside `core/` is pure.

---

## Step 1 — The Command Type

Create `use_cases/<use_case>/command.rs`. A command is a plain struct — no validation,
no methods, no traits. It carries all the data the decider needs to make a decision.

```rust
// use_cases/register_time_entry/command.rs

pub struct RegisterTimeEntry {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub created_by: String,
}
```

All fields are resolved by the inbound adapter before the command is constructed — the decider
never generates IDs or timestamps.

---

## Step 2 — The Domain Event(s)

Add a new variant to `core/events.rs`. Events are versioned (`V1` suffix) and must be
`Serialize` + `Deserialize` for persistence.

```rust
// core/events.rs

pub enum TimeEntryEvent {
    TimeEntryRegisteredV1(v1::time_entry_registered::TimeEntryRegisteredV1),
    // new variants go here
}
```

The event struct lives in `core/events/v1/<event_name>.rs` and carries only the data that
actually changed — no derived fields, no IDs of other aggregates.

```rust
// core/events/v1/time_entry_registered.rs

#[derive(Clone, Serialize, Deserialize)]
pub struct TimeEntryRegisteredV1 {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub created_by: String,
}
```

---

## Step 3 — The Decision Type

Create `use_cases/<use_case>/decision.rs`. Two variants only — `Accepted` carries produced
events and intents; `Rejected` carries a typed error reason, never a string.

```rust
// use_cases/register_time_entry/decision.rs

pub enum Decision {
    Accepted {
        events: Vec<TimeEntryEvent>,
        intents: Vec<TimeEntryIntent>,
    },
    Rejected {
        reason: DecideError,
    },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DecideError {
    #[error("time entry already exists")]
    AlreadyExists,
    #[error("end time must be after start time")]
    InvalidInterval,
}
```

`DecideError` variants are exhaustive domain reasons — the handler maps them to transport-level
errors (HTTP status codes, GraphQL errors) without leaking domain strings to callers.

---

## Step 4 — The Decider

Create `use_cases/<use_case>/decide.rs`. A pure function: takes state and command, returns
a `Decision`. No I/O, no `async`, no side effects. Match on the current state variant first,
then validate the command.

```rust
// use_cases/register_time_entry/decide.rs

pub fn decide_register(state: &TimeEntryState, command: RegisterTimeEntry) -> Decision {
    match state {
        TimeEntryState::None => {
            if command.end_time <= command.start_time {
                return Decision::Rejected {
                    reason: DecideError::InvalidInterval,
                };
            }
            let payload = TimeEntryRegisteredV1 {
                time_entry_id: command.time_entry_id.clone(),
                // ... map fields ...
            };
            Decision::Accepted {
                events: vec![TimeEntryEvent::TimeEntryRegisteredV1(payload.clone())],
                intents: vec![TimeEntryIntent::PublishTimeEntryRegistered { payload }],
            }
        }
        _ => Decision::Rejected {
            reason: DecideError::AlreadyExists,
        },
    }
}
```

**Rules for the decider:**
- Match on `state` first, not the command. State determines what transitions are valid.
- Validate command fields only inside a valid state arm.
- Never call `evolve`. Never read from any store. Never produce side effects.
- Return `Accepted` with **both** events and intents. The handler dispatches them.

---

## Step 5 — State and Evolve

See the [state-and-evolve guide](state-and-evolve.md) for the full pattern. In brief:

```rust
// core/state.rs — add a variant per lifecycle stage
pub enum TimeEntryState {
    None,
    Registered { time_entry_id: String, /* ... */ },
}

// core/evolve.rs — pure fold function
pub fn evolve(state: TimeEntryState, event: TimeEntryEvent) -> TimeEntryState {
    match (state, event) {
        (TimeEntryState::None, TimeEntryEvent::TimeEntryRegisteredV1(e)) => {
            TimeEntryState::Registered { time_entry_id: e.time_entry_id, /* ... */ }
        }
        (state, _) => state, // unknown combination — no-op
    }
}
```

The handler reconstructs state by folding all past events:

```rust
let state = stream.events.iter().cloned().fold(TimeEntryState::None, evolve);
```

---

## Step 6 — The Intents

Add a variant to `core/intents.rs` for each intent the decider can produce. An intent carries
the payload the relay needs to act — usually the same data as the event.

```rust
// core/intents.rs

pub enum TimeEntryIntent {
    PublishTimeEntryRegistered { payload: TimeEntryRegisteredV1 },
    // new intents go here
}
```

See the [intents-and-outbox guide](intents-and-outbox.md) for how intents flow to relays.

---

## Step 7 — The Command Handler

Create `use_cases/<use_case>/handler.rs`. The handler is the imperative shell — the only place
with I/O. It orchestrates the pipeline and maps infrastructure errors to `ApplicationError`.

```rust
// use_cases/register_time_entry/handler.rs

pub struct RegisterTimeEntryHandler<TEventStore, TOutbox> {
    topic: String,
    event_store: Arc<TEventStore>,
    outbox: Arc<TOutbox>,
}

impl<TEventStore, TOutbox> RegisterTimeEntryHandler<TEventStore, TOutbox>
where
    TEventStore: EventStore<TimeEntryEvent> + Send + Sync + 'static,
    TOutbox: DomainOutbox + Send + Sync + 'static,
{
    pub async fn handle(
        &self,
        stream_id: &str,
        command: RegisterTimeEntry,
    ) -> Result<(), ApplicationError> {
        // 1. Load past events (includes stream version for optimistic concurrency)
        let stream = self.event_store.load(stream_id).await
            .map_err(ApplicationError::VersionConflict)?;

        // 2. Reconstruct current state by folding past events
        let state = stream.events.iter().cloned().fold(TimeEntryState::None, evolve);

        // 3. Call the pure decider
        match decide_register(&state, command) {
            Decision::Accepted { events, intents } => {
                // 4. Append events to the event store (version-checked, atomic)
                self.event_store
                    .append(stream_id, stream.version, &events)
                    .await
                    .map_err(ApplicationError::VersionConflict)?;

                // 5. Enqueue intents in the outbox (at-least-once delivery)
                dispatch_intents(&*self.outbox, stream_id, stream.version, &self.topic, intents)
                    .await
                    .map_err(ApplicationError::Outbox)?;

                Ok(())
            }
            Decision::Rejected { reason } => {
                Err(ApplicationError::Domain(reason.to_string()))
            }
        }
    }
}
```

**`ApplicationError`** maps infrastructure failures to typed handler errors:

```rust
pub enum ApplicationError {
    #[error(transparent)] VersionConflict(#[from] EventStoreError),
    #[error(transparent)] Outbox(#[from] OutboxError),
    #[error("domain rejected: {0}")] Domain(String),
    #[error("unexpected: {0}")] Unexpected(String),
}
```

**Why no `InformCallerOfRejection` here?** This codebase handles rejection by returning
`ApplicationError::Domain` directly to the inbound adapter. If the architecture requires
an explicit cross-cutting `InformCallerOfRejection` intent (e.g., for async flows), add it
in the `Decision::Rejected` arm before returning the error.

---

## Step 8 — `dispatch_intents` in `adapters/outbound/intent_outbox.rs`

Add a match arm for each new intent variant. This translates a domain intent into an
`OutboxRow` and enqueues it.

```rust
// adapters/outbound/intent_outbox.rs

pub async fn dispatch_intents(
    outbox: &impl DomainOutbox,
    stream_id: &str,
    starting_version: i64,
    topic: &str,
    intents: Vec<TimeEntryIntent>,
) -> Result<(), OutboxError> {
    for (i, intent) in intents.into_iter().enumerate() {
        let stream_version = starting_version + i as i64 + 1;
        match intent {
            TimeEntryIntent::PublishTimeEntryRegistered { payload } => {
                outbox.enqueue(OutboxRow {
                    topic: topic.to_string(),
                    event_type: "TimeEntryRegistered".to_string(),
                    event_version: 1,
                    stream_id: stream_id.to_string(),
                    stream_version,
                    occurred_at: payload.created_at,
                    payload: serde_json::to_value(payload).unwrap(),
                }).await?;
            }
            // new intent variants go here
        }
    }
    Ok(())
}
```

---

## Step 9 — HTTP Inbound Adapter

Create `use_cases/<use_case>/inbound/http.rs`. The inbound adapter owns:
- Parsing the transport request
- Generating IDs and timestamps
- Building the command struct
- Mapping `ApplicationError` to HTTP status codes

```rust
// inbound/http.rs

#[derive(Deserialize)]
pub struct RegisterTimeEntryBody {
    pub start_time: i64,
    pub end_time: i64,
}

#[derive(Serialize)]
pub struct RegisterTimeEntryResponse {
    pub time_entry_id: String,
}

pub async fn handle(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    body: Result<Json<RegisterTimeEntryBody>, JsonRejection>,
) -> impl IntoResponse {
    let Json(body) = match body {
        Ok(b) => b,
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };

    let user_id = match params.get("user_id") {
        Some(id) => id,
        None => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };

    let time_entry_id = Uuid::now_v7();
    let stream_id = format!("TimeEntry-{time_entry_id}");

    let command = RegisterTimeEntry {
        time_entry_id: time_entry_id.to_string(),
        user_id: user_id.to_owned(),
        start_time: body.start_time,
        end_time: body.end_time,
        tags: vec!["work".to_string()],
        description: "Work work work work work".to_string(),
        created_at: Utc::now().timestamp_millis(),
        created_by: "user-from-auth".into(),
    };

    match state.register_handler.handle(&stream_id, command).await {
        Ok(()) => (StatusCode::CREATED, Json(RegisterTimeEntryResponse {
            time_entry_id: time_entry_id.to_string(),
        })).into_response(),
        Err(ApplicationError::Domain(_)) => StatusCode::CONFLICT.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
```

**Error mapping:**

| `ApplicationError` variant | HTTP status |
|----------------------------|-------------|
| `Domain(_)` | `409 CONFLICT` |
| `VersionConflict(_)` / `Outbox(_)` / `Unexpected(_)` | `500 INTERNAL_SERVER_ERROR` |
| JSON parse failure | `422 UNPROCESSABLE_ENTITY` |
| Missing required query param | `422 UNPROCESSABLE_ENTITY` |

Register the route in `shell/http.rs`:

```rust
.route("/register-time-entry", post(register_http::handle))
```

---

## Step 10 — GraphQL Inbound Adapter

Create `use_cases/<use_case>/inbound/graphql.rs`. Mutations live on `MutationRoot`.
All parameters are explicit GraphQL arguments. All `ApplicationError` variants become
GraphQL errors (no status codes).

```rust
// inbound/graphql.rs

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn register_time_entry(
        &self,
        context: &Context<'_>,
        user_id: String,
        start_time: i64,
        end_time: i64,
        tags: Vec<String>,
        description: String,
    ) -> GqlResult<ID> {
        let time_entry_id = Uuid::now_v7();
        let state = context.data_unchecked::<AppState>();

        let command = RegisterTimeEntry {
            time_entry_id: time_entry_id.to_string(),
            user_id,
            start_time,
            end_time,
            tags,
            description,
            created_at: Utc::now().timestamp_millis(),
            created_by: "user-from-auth".into(),
        };

        state.register_handler
            .handle(&format!("TimeEntry-{time_entry_id}"), command)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(ID(time_entry_id.to_string()))
    }
}
```

Compose `MutationRoot` into the schema in `shell/graphql.rs`:

```rust
pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;
```

---

## Step 11 — Wiring

**`lib.rs`** — add module declarations inside the use case block:

```rust
pub mod register_time_entry {
    pub mod command;
    pub mod decide;
    pub mod decision;
    pub mod handler;
    pub mod inbound {
        pub mod graphql;
        pub mod http;
    }
}
```

**`shell/state.rs`** — add the handler to `AppState`:

```rust
pub struct AppState {
    pub register_handler: Arc<RegisterTimeEntryHandler<InMemoryEventStore<TimeEntryEvent>, InMemoryDomainOutbox>>,
    // ...
}
```

**`shell/main.rs`** — wire the handler:

```rust
let register_handler = Arc::new(RegisterTimeEntryHandler::new(
    "time-entries.v1",
    event_store.clone(),
    outbox,
));
```

---

## Checklist

- [ ] `command.rs` — plain struct, all fields resolved by inbound adapter
- [ ] `core/events.rs` — new event variant added to the enum
- [ ] Event struct in `core/events/v1/<name>.rs` — derives `Clone, Serialize, Deserialize`
- [ ] `decision.rs` — `Decision` enum with `Accepted { events, intents }` / `Rejected { reason }`
- [ ] `DecideError` — `thiserror::Error` enum with one variant per domain rejection reason
- [ ] `decide.rs` — pure function, no I/O, matches on state variant first
- [ ] `core/state.rs` — new lifecycle variant if the event changes aggregate state
- [ ] `core/evolve.rs` — new arm for the new event variant
- [ ] `core/intents.rs` — new variant for each intent produced by the decider
- [ ] `handler.rs` — load → fold → decide → append + dispatch atomically
- [ ] `adapters/outbound/intent_outbox.rs` — new match arm in `dispatch_intents`
- [ ] `inbound/http.rs` — parse → build command → call handler → map errors
- [ ] `inbound/graphql.rs` — `#[Object]` mutation on `MutationRoot`
- [ ] `lib.rs` — module declarations
- [ ] `shell/state.rs` — handler field in `AppState`
- [ ] `shell/main.rs` — handler constructed and injected
- [ ] `shell/http.rs` — route registered
- [ ] `shell/graphql.rs` — `MutationRoot` composed into schema
- [ ] Tests for `decide.rs`, `handler.rs`, `inbound/http.rs` at 100% coverage
