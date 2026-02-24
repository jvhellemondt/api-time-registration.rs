# REST API HTTP Inbound Adapters Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add HTTP REST inbound adapters for `register_time_entry` (`POST /register-time-entry`) and `list_time_entries_by_user` (`GET /list-time-entries`), wired through a new `shell/http.rs` that mirrors the existing `shell/graphql.rs` pattern.

**Architecture:** Each use case gets an `inbound/http.rs` alongside its existing `inbound/graphql.rs`. Handlers use Axum's `State<AppState>` extractor. `shell/http.rs` assembles the `Router` and passes `.with_state(state)`. `main.rs` merges the HTTP router before the GraphQL route. No inline projection (fire-and-forget). No technical events (deferred per ADR-0004).

**Tech Stack:** Axum 0.8 (`State`, `Json`, `Query` extractors), tower 0.5 (`ServiceExt::oneshot` in tests), http-body-util 0.1 (`BodyExt::collect` for reading response bodies in tests), serde (`Deserialize`/`Serialize`).

---

### Task 1: Add test dependencies and update coverage exclusion

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add dev-dependencies**

In `Cargo.toml`, append to `[dev-dependencies]`:

```toml
tower = { version = "0.5", features = ["util"] }
http-body-util = "0.1"
```

**Step 2: Extend coverage exclusion to include shell/http.rs**

In `Cargo.toml`, find the `coverage` script line and change the regex from:

```
--ignore-filename-regex \"(shell/main\\.rs|/graphql\\.rs)\"
```

to:

```
--ignore-filename-regex \"(shell/main\\.rs|/graphql\\.rs|shell/http\\.rs)\"
```

The full updated line becomes:

```toml
coverage = "cargo llvm-cov nextest --workspace --ignore-filename-regex \"(shell/main\\.rs|/graphql\\.rs|shell/http\\.rs)\" --fail-under-functions 100 --fail-under-lines 100 --fail-under-regions 100 --show-missing-lines"
```

**Step 3: Verify it compiles**

```bash
cargo build
```

Expected: compiles with no errors.

**Step 4: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add tower and http-body-util dev-deps for HTTP adapter tests"
```

---

### Task 2: HTTP inbound adapter for register_time_entry

**Files:**
- Create: `src/modules/time_entries/use_cases/register_time_entry/inbound/http.rs`
- Modify: `src/lib.rs` (add `pub mod http;` inside `register_time_entry::inbound`)

**Step 1: Write the failing tests**

Create `src/modules/time_entries/use_cases/register_time_entry/inbound/http.rs` with only the test module (the handler does not exist yet):

```rust
#[cfg(test)]
mod register_time_entry_http_inbound_tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::post,
    };
    use http_body_util::BodyExt;
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::modules::time_entries::adapters::outbound::projections_in_memory::InMemoryProjections;
    use crate::modules::time_entries::core::events::TimeEntryEvent;
    use crate::modules::time_entries::use_cases::list_time_entries_by_user::handler::Projector;
    use crate::modules::time_entries::use_cases::register_time_entry::handler::RegisterTimeEntryHandler;
    use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
    use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
    use crate::shell::state::AppState;

    use super::handle;

    fn make_test_state() -> AppState {
        let event_store = Arc::new(InMemoryEventStore::<TimeEntryEvent>::new());
        let outbox = Arc::new(InMemoryDomainOutbox::new());
        let projections = Arc::new(InMemoryProjections::new());
        let projector = Arc::new(Projector::new(
            "test",
            projections.clone(),
            projections.clone(),
        ));
        let register_handler = Arc::new(RegisterTimeEntryHandler::new(
            "time-entries",
            event_store.clone(),
            outbox,
        ));
        AppState {
            queries: projections,
            register_handler,
            event_store,
            projector,
        }
    }

    fn app(state: AppState) -> Router {
        Router::new()
            .route("/register-time-entry", post(handle))
            .with_state(state)
    }

    #[tokio::test]
    async fn it_should_return_201_with_time_entry_id_on_valid_request() {
        let body = r#"{"user_id":"u-1","start_time":1000,"end_time":2000,"tags":["Work"],"description":"test"}"#;

        let response = app(make_test_state())
            .oneshot(
                Request::post("/register-time-entry")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json.get("time_entry_id").is_some());
    }

    #[tokio::test]
    async fn it_should_return_409_when_domain_rejects_invalid_interval() {
        // end_time < start_time triggers DecideError::InvalidInterval -> ApplicationError::Domain -> 409
        let body = r#"{"user_id":"u-1","start_time":2000,"end_time":1000,"tags":[],"description":"test"}"#;

        let response = app(make_test_state())
            .oneshot(
                Request::post("/register-time-entry")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn it_should_return_422_on_invalid_json() {
        let response = app(make_test_state())
            .oneshot(
                Request::post("/register-time-entry")
                    .header("content-type", "application/json")
                    .body(Body::from("not-json"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
```

**Step 2: Register the module in lib.rs**

In `src/lib.rs`, inside `register_time_entry`, change:

```rust
pub mod inbound {
    pub mod graphql;
}
```

to:

```rust
pub mod inbound {
    pub mod graphql;
    pub mod http;
}
```

**Step 3: Run tests to confirm they fail to compile**

```bash
cargo nextest run -p time_entries register_time_entry_http_inbound_tests
```

Expected: compile error — `use of undeclared function 'handle'` (or similar). This confirms the tests are wired correctly.

**Step 4: Implement the handler**

Add the following before the `#[cfg(test)]` block in `src/modules/time_entries/use_cases/register_time_entry/inbound/http.rs`:

```rust
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::time_entries::use_cases::register_time_entry::command::RegisterTimeEntry;
use crate::modules::time_entries::use_cases::register_time_entry::handler::ApplicationError;
use crate::shell::state::AppState;

#[derive(Deserialize)]
pub struct RegisterTimeEntryBody {
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
}

#[derive(Serialize)]
pub struct RegisterTimeEntryResponse {
    pub time_entry_id: String,
}

pub async fn handle(
    State(state): State<AppState>,
    Json(body): Json<RegisterTimeEntryBody>,
) -> impl IntoResponse {
    let time_entry_id = Uuid::now_v7();
    let stream_id = format!("TimeEntry-{time_entry_id}");

    let command = RegisterTimeEntry {
        time_entry_id: time_entry_id.to_string(),
        user_id: body.user_id,
        start_time: body.start_time,
        end_time: body.end_time,
        tags: body.tags,
        description: body.description,
        created_at: Utc::now().timestamp_millis(),
        created_by: "user-from-auth".into(),
    };

    match state.register_handler.handle(&stream_id, command).await {
        Ok(()) => (
            StatusCode::CREATED,
            Json(RegisterTimeEntryResponse {
                time_entry_id: time_entry_id.to_string(),
            }),
        )
            .into_response(),
        Err(ApplicationError::Domain(_)) => StatusCode::CONFLICT.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
```

**Step 5: Run the tests and verify they pass**

```bash
cargo nextest run -p time_entries register_time_entry_http_inbound_tests
```

Expected: 3 tests PASS.

**Step 6: Commit**

```bash
git add src/modules/time_entries/use_cases/register_time_entry/inbound/http.rs src/lib.rs
git commit -m "feat: add HTTP inbound adapter for register_time_entry"
```

---

### Task 3: HTTP inbound adapter for list_time_entries_by_user

**Files:**
- Create: `src/modules/time_entries/use_cases/list_time_entries_by_user/inbound/http.rs`
- Modify: `src/lib.rs` (add `pub mod http;` inside `list_time_entries_by_user::inbound`)

**Step 1: Write the failing tests**

Create `src/modules/time_entries/use_cases/list_time_entries_by_user/inbound/http.rs` with only the test module:

```rust
#[cfg(test)]
mod list_time_entries_by_user_http_inbound_tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use http_body_util::BodyExt;
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::modules::time_entries::adapters::outbound::projections_in_memory::InMemoryProjections;
    use crate::modules::time_entries::core::events::TimeEntryEvent;
    use crate::modules::time_entries::use_cases::list_time_entries_by_user::handler::Projector;
    use crate::modules::time_entries::use_cases::register_time_entry::handler::RegisterTimeEntryHandler;
    use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
    use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
    use crate::shell::state::AppState;

    use super::handle;

    fn make_test_state() -> AppState {
        let event_store = Arc::new(InMemoryEventStore::<TimeEntryEvent>::new());
        let outbox = Arc::new(InMemoryDomainOutbox::new());
        let projections = Arc::new(InMemoryProjections::new());
        let projector = Arc::new(Projector::new(
            "test",
            projections.clone(),
            projections.clone(),
        ));
        let register_handler = Arc::new(RegisterTimeEntryHandler::new(
            "time-entries",
            event_store.clone(),
            outbox,
        ));
        AppState {
            queries: projections,
            register_handler,
            event_store,
            projector,
        }
    }

    fn app(state: AppState) -> Router {
        Router::new()
            .route("/list-time-entries", get(handle))
            .with_state(state)
    }

    #[tokio::test]
    async fn it_should_return_200_with_empty_list_when_no_entries_exist() {
        let response = app(make_test_state())
            .oneshot(
                Request::get("/list-time-entries?user_id=u-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json, serde_json::json!([]));
    }

    #[tokio::test]
    async fn it_should_return_400_when_user_id_is_missing() {
        let response = app(make_test_state())
            .oneshot(
                Request::get("/list-time-entries")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
```

**Step 2: Register the module in lib.rs**

In `src/lib.rs`, inside `list_time_entries_by_user`, change:

```rust
pub mod inbound {
    pub mod graphql;
}
```

to:

```rust
pub mod inbound {
    pub mod graphql;
    pub mod http;
}
```

**Step 3: Run tests to confirm they fail to compile**

```bash
cargo nextest run -p time_entries list_time_entries_by_user_http_inbound_tests
```

Expected: compile error — `handle` is not defined yet.

**Step 4: Implement the handler**

Add the following before the `#[cfg(test)]` block in `src/modules/time_entries/use_cases/list_time_entries_by_user/inbound/http.rs`:

```rust
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

use crate::shell::state::AppState;

#[derive(Deserialize)]
pub struct ListTimeEntriesParams {
    pub user_id: String,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub sort_desc: Option<bool>,
}

pub async fn handle(
    State(state): State<AppState>,
    Query(params): Query<ListTimeEntriesParams>,
) -> impl IntoResponse {
    match state
        .queries
        .list_by_user_id(
            &params.user_id,
            params.offset.unwrap_or(0),
            params.limit.unwrap_or(20),
            params.sort_desc.unwrap_or(true),
        )
        .await
    {
        Ok(entries) => Json(entries).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
```

**Step 5: Run the tests and verify they pass**

```bash
cargo nextest run -p time_entries list_time_entries_by_user_http_inbound_tests
```

Expected: 2 tests PASS.

**Step 6: Commit**

```bash
git add src/modules/time_entries/use_cases/list_time_entries_by_user/inbound/http.rs src/lib.rs
git commit -m "feat: add HTTP inbound adapter for list_time_entries_by_user"
```

---

### Task 4: shell/http.rs and wire into main.rs

**Files:**
- Create: `src/shell/http.rs`
- Modify: `src/shell/mod.rs` (add `pub mod http;`)
- Modify: `src/shell/main.rs` (import and merge HTTP router)

**Step 1: Create src/shell/http.rs**

```rust
use axum::{Router, routing::{get, post}};

use crate::modules::time_entries::use_cases::list_time_entries_by_user::inbound::http as list_http;
use crate::modules::time_entries::use_cases::register_time_entry::inbound::http as register_http;
use crate::shell::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/register-time-entry", post(register_http::handle))
        .route("/list-time-entries", get(list_http::handle))
        .with_state(state)
}
```

**Step 2: Register in shell/mod.rs**

In `src/shell/mod.rs`, add:

```rust
pub mod http;
```

**Step 3: Update shell/main.rs**

In `src/shell/main.rs`:

Add the import at the top alongside the existing `use time_entries::shell::graphql::...` import:

```rust
use time_entries::shell::http as shell_http;
```

Then, before the `schema` is built, clone `state` and build the HTTP router. Change the block that looks like:

```rust
let state = AppState { ... };

let schema: AppSchema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
    .data(state)
    .finish();

let app = Router::new()
    .route("/gql", get(graphiql).post(graphql))
    .layer(Extension(schema))
    .layer(TraceLayer::new_for_http());
```

to:

```rust
let state = AppState { ... };

let http_router = shell_http::router(state.clone());

let schema: AppSchema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
    .data(state)
    .finish();

let app = Router::new()
    .merge(http_router)
    .route("/gql", get(graphiql).post(graphql))
    .layer(Extension(schema))
    .layer(TraceLayer::new_for_http());
```

**Step 4: Run all checks**

```bash
cargo run-script fmt
```

Expected: exits 0 (no formatting issues).

```bash
cargo run-script lint
```

Expected: exits 0 (no clippy warnings).

```bash
cargo run-script test
```

Expected: all tests PASS including the 5 new HTTP adapter tests.

```bash
cargo run-script coverage
```

Expected: exits 0, 100% coverage on functions/lines/regions (shell/http.rs excluded by regex).

**Step 5: Commit**

```bash
git add src/shell/http.rs src/shell/mod.rs src/shell/main.rs
git commit -m "feat: wire HTTP router into shell"
```
