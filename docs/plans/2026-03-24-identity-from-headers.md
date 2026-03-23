# Plan: Extract userId and tenantId from HTTP Headers

## Context

Currently `userId` is passed in request bodies (mutations) or query params (queries) in REST, and as explicit GraphQL arguments in GQL. `tenantId` is hardcoded as `"tenant-hardcoded"` everywhere. `created_by`/`updated_by`/`deleted_by` audit fields are hardcoded as `"user-from-auth"`.

The goal is to read both `userId` and `tenantId` from HTTP headers `X-USER-ID` and `X-TENANT-ID` for all REST and GQL endpoints. Missing headers → 401 (REST) or GQL error.

---

## Step 1 — Add `RequestContext` to `src/shared/core/primitives.rs`

Add struct + axum `FromRequestParts` impl:

```rust
use axum::extract::FromRequestParts;
use axum::http::{StatusCode, request::Parts};

pub struct RequestContext {
    pub user_id: String,
    pub tenant_id: String,
}

impl<S: Send + Sync> FromRequestParts<S> for RequestContext {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, StatusCode> {
        let user_id = parts.headers.get("x-user-id")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let tenant_id = parts.headers.get("x-tenant-id")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
            .ok_or(StatusCode::UNAUTHORIZED)?;
        Ok(RequestContext { user_id, tenant_id })
    }
}
```

Add unit tests in the same file:
- Both headers present → extracts successfully
- Missing `X-USER-ID` → `StatusCode::UNAUTHORIZED`
- Missing `X-TENANT-ID` → `StatusCode::UNAUTHORIZED`

---

## Step 2 — Update GQL HTTP handler in `src/shell/main.rs`

Change `graphql()` signature to extract `HeaderMap`, build `RequestContext`, inject it into per-request context data:

```rust
async fn graphql(
    Extension(schema): Extension<AppSchema>,
    headers: HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let user_id = headers.get("x-user-id").and_then(|v| v.to_str().ok()).map(str::to_string);
    let tenant_id = headers.get("x-tenant-id").and_then(|v| v.to_str().ok()).map(str::to_string);
    match (user_id, tenant_id) {
        (Some(user_id), Some(tenant_id)) => {
            let ctx = RequestContext { user_id, tenant_id };
            schema.execute(req.into_inner().data(ctx)).await.into()
        }
        _ => GraphQLResponse::from(async_graphql::Response::from_errors(vec![
            async_graphql::ServerError::new("Unauthorized: missing X-USER-ID or X-TENANT-ID header", None),
        ])),
    }
}
```

`HeaderMap` is a `FromRequestParts` extractor, `GraphQLRequest` consumes the body — axum handles both correctly.

---

## Step 3 — Update REST inbound handlers (9 files)

Add `request_ctx: RequestContext` as handler parameter. Remove `user_id` from request body structs and query param structs. Replace hardcoded values.

Pattern (after change):
```rust
pub async fn handle_put(
    State(state): State<AppState>,
    request_ctx: RequestContext,  // ← new
    Path(time_entry_id): Path<String>,
    body: Result<Json<SetStartedAtBody>, JsonRejection>,
) -> impl IntoResponse {
    let command = SetStartedAt {
        user_id: request_ctx.user_id.clone(),   // ← from header
        updated_by: request_ctx.user_id,        // ← from header
        ...
    };
}
```

Files and what changes:

| File | Remove from struct | Add to command |
|------|--------------------|---------------|
| `time_entries/.../set_started_at/inbound/http.rs` | `user_id` from body | `user_id`, `updated_by` from ctx |
| `time_entries/.../set_ended_at/inbound/http.rs` | `user_id` from body | `user_id`, `updated_by` from ctx |
| `time_entries/.../set_time_entry_tags/inbound/http.rs` | `user_id` from body | `user_id`, `updated_by` from ctx |
| `time_entries/.../list_time_entries_by_user/inbound/http.rs` | `user_id` from query params | `user_id` from ctx |
| `tags/.../create_tag/inbound/http.rs` | — | `tenant_id`, `created_by` from ctx |
| `tags/.../delete_tag/inbound/http.rs` | — | `tenant_id`, `deleted_by` from ctx |
| `tags/.../set_tag_name/inbound/http.rs` | — | `tenant_id`, `updated_by` from ctx |
| `tags/.../set_tag_color/inbound/http.rs` | — | `tenant_id`, `updated_by` from ctx |
| `tags/.../set_tag_description/inbound/http.rs` | — | `tenant_id`, `updated_by` from ctx |

---

## Step 4 — Update GQL inbound resolvers (9 files)

Remove `user_id: String` argument from resolver method signatures (breaking schema change, intentional). Read from `RequestContext` injected into per-request context data.

Pattern:
```rust
async fn set_started_at(
    &self,
    context: &Context<'_>,
    time_entry_id: String,
    // user_id: String,  ← removed
    started_at: i64,
) -> GqlResult<bool> {
    let req_ctx = context.data::<RequestContext>()
        .map_err(|_| async_graphql::Error::new("Unauthorized"))?;
    let command = SetStartedAt {
        user_id: req_ctx.user_id.clone(),
        updated_by: req_ctx.user_id.clone(),
        ...
    };
}
```

Files:
- `time_entries/.../set_started_at/inbound/graphql.rs`
- `time_entries/.../set_ended_at/inbound/graphql.rs`
- `time_entries/.../set_time_entry_tags/inbound/graphql.rs`
- `time_entries/.../list_time_entries_by_user/inbound/graphql.rs`
- `tags/.../create_tag/inbound/graphql.rs`
- `tags/.../delete_tag/inbound/graphql.rs`
- `tags/.../set_tag_name/inbound/graphql.rs`
- `tags/.../set_tag_color/inbound/graphql.rs`
- `tags/.../set_tag_description/inbound/graphql.rs`

---

## Step 5 — Update tests

**REST handler tests** (inline `#[cfg(test)]` in each handler file):
- Add `.header("x-user-id", "u-1")` and `.header("x-tenant-id", "tenant-test")` to every `Request::builder()` call
- Remove `user_id` from JSON bodies and query strings
- Add a test for missing header → 401 in each handler (for coverage)

**GQL resolver tests** (inline `#[cfg(test)]` in each graphql.rs):
- Change `schema.execute(format!(...))` to:
  ```rust
  schema.execute(
      async_graphql::Request::new(format!(...))
          .data(RequestContext { user_id: "u-1".to_string(), tenant_id: "tenant-test".to_string() })
  ).await
  ```
- Remove `userId: "u-1"` from mutation strings

---

## Files Modified

- `src/shared/core/primitives.rs` — `RequestContext` struct + extractor + tests
- `src/shell/main.rs` — `graphql()` handler header extraction
- 9 REST handlers under `src/modules/`
- 9 GQL resolvers under `src/modules/`

**No changes needed:**
- `src/lib.rs` — `primitives` already declared
- `src/shell/graphql.rs` — schema wiring unchanged
- `src/shell/http.rs` — router unchanged; extractors are per-handler
- Command structs — already accept plain `String` fields, only source changes

---

## Verification

```bash
cargo run-script fmt-fix
cargo run-script lint
cargo run-script test
cargo run-script coverage
```

Manually test (when server is running):
```bash
# REST — success
curl -X PUT http://localhost:8080/time-entries/{id}/start \
  -H "X-USER-ID: u-1" -H "X-TENANT-ID: t-1" \
  -H "Content-Type: application/json" \
  -d '{"started_at": 1000}'

# REST — missing header → 401
curl -X PUT http://localhost:8080/time-entries/{id}/start \
  -H "Content-Type: application/json" \
  -d '{"started_at": 1000}'

# GQL — success
curl -X POST http://localhost:8080/gql \
  -H "X-USER-ID: u-1" -H "X-TENANT-ID: t-1" \
  -H "Content-Type: application/json" \
  -d '{"query":"mutation { setStartedAt(timeEntryId: \"...\", startedAt: 1000) }"}'
```
