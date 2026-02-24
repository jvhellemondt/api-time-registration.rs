# Health Check Endpoint Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `GET /health` to the existing HTTP router that returns `200 OK` with `{"status":"ok"}`.

**Architecture:** A single stateless handler is added directly to `src/shell/http.rs` alongside the existing routes. No new files, no `AppState` dependency. `shell/http.rs` is excluded from coverage by the existing regex so the test is for confidence only.

**Tech Stack:** Axum 0.8 (`Json`, `IntoResponse`), serde_json (`json!` macro), tower 0.5 + http-body-util (test utilities already in dev-dependencies).

---

### Task 1: Add the health handler and route

**Files:**
- Modify: `src/shell/http.rs`

**Step 1: Note the current file content**

`src/shell/http.rs` currently looks like:

```rust
use axum::{
    Router,
    routing::{get, post},
};

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

**Step 2: Write the complete updated file**

Replace the contents of `src/shell/http.rs` with:

```rust
use axum::{Json, Router, response::IntoResponse, routing::{get, post}};

use crate::modules::time_entries::use_cases::list_time_entries_by_user::inbound::http as list_http;
use crate::modules::time_entries::use_cases::register_time_entry::inbound::http as register_http;
use crate::shell::state::AppState;

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok"}))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/register-time-entry", post(register_http::handle))
        .route("/list-time-entries", get(list_http::handle))
        .with_state(state)
}
```

**Step 3: Verify it builds**

```bash
cargo build
```

Expected: exits 0, no errors.

**Step 4: Run fmt and lint**

```bash
cargo run-script fmt
cargo run-script lint
```

Expected: both exit 0. If fmt fails, run `cargo run-script fmt-fix` first.

**Step 5: Run all tests**

```bash
cargo run-script test
```

Expected: all existing tests still pass (the new handler has no test of its own since `shell/http.rs` is excluded from coverage).

**Step 6: Commit**

```bash
git add src/shell/http.rs
git commit -m "feat: add GET /health endpoint"
```
