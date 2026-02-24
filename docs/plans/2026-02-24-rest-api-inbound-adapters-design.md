# Design: REST API Inbound Adapters

Date: 2026-02-24

## Summary

Add HTTP REST inbound adapters for two existing use cases: `register_time_entry` and `list_time_entries_by_user`. The adapters live in `inbound/http.rs` alongside the existing `inbound/graphql.rs`. A new `shell/http.rs` assembles the Axum router, mirroring the existing `shell/graphql.rs` pattern.

## Endpoints

| Method | Path | Handler |
|--------|------|---------|
| `POST` | `/register-time-entry` | `register_time_entry::inbound::http::handle` |
| `GET`  | `/list-time-entries`   | `list_time_entries_by_user::inbound::http::handle` |

### POST /register-time-entry

Request body (JSON):
```json
{
  "user_id": "string",
  "start_time": 1234567890000,
  "end_time": 1234567890000,
  "tags": ["Work"],
  "description": "string"
}
```

Response `201 Created`:
```json
{ "time_entry_id": "uuid-v7" }
```

Error responses: `422 Unprocessable Entity` for invalid JSON, `409 Conflict` for domain rejections, `500 Internal Server Error` for infrastructure failures.

### GET /list-time-entries

Query parameters: `user_id` (required), `offset` (default 0), `limit` (default 20), `sort_desc` (default true).

Response `200 OK`: JSON array of `TimeEntryView` objects.

## Architecture

### No inline projection

The HTTP `register_time_entry` adapter does not inline-apply the projection after writing. It writes the command and returns immediately. The projector runner catches up asynchronously. This matches proper FCIS separation and differs from the existing GraphQL adapter (which does inline-project as a shortcut).

### No technical events (deferred)

Technical events (ADR-0004) are deferred. Neither HTTP adapter wires `TechnicalEventStore` for now, consistent with the existing GraphQL adapters.

## New Files

```
src/
  modules/time_entries/use_cases/
    register_time_entry/inbound/
      http.rs           ← NEW: POST /register-time-entry handler
    list_time_entries_by_user/inbound/
      http.rs           ← NEW: GET /list-time-entries handler
  shell/
    http.rs             ← NEW: assembles Router with both HTTP routes
```

## Modified Files

- `src/lib.rs` — add `pub mod http;` inside `register_time_entry::inbound` and `list_time_entries_by_user::inbound`
- `src/shell/mod.rs` — add `pub mod http;`
- `src/shell/main.rs` — merge `http::router(state.clone())` into the Axum app
