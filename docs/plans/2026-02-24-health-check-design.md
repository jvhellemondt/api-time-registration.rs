# Design: Health Check Endpoint

Date: 2026-02-24

## Summary

Add `GET /health` to the existing HTTP router in `shell/http.rs`. Returns `200 OK` with `{"status":"ok"}`. Shallow ping only — confirms the process is alive, no dependency checks.

## Design

**Handler** — stateless, no `AppState` needed:

```rust
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok"}))
}
```

**Route** — added to `router()` in `shell/http.rs`:

```rust
.route("/health", get(health))
```

**File modified:** `src/shell/http.rs` only. No new files.

**Coverage:** `shell/http.rs` is excluded from coverage by the existing regex — no test required, but one will be added for completeness.
