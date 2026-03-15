# Auth Provider Integration (Auth0 / Clerk)

**Target file:** `docs/plans/2026-03-15-auth-provider-jwt-integration.md`

## Context

The IAM bounded context plan (`docs/plans/2026-03-15-iam-bounded-context.md`) defines the full authorization architecture. That plan assumes a specific provider (Auth0) in one place: `shared/infrastructure/auth/jwt.rs`. This plan documents what the Rust backend must do to validate JWTs from either Auth0 or Clerk, why the architecture looks the way it does, and what was rejected.

Both providers issue RS256-signed JWTs verified via a JWKS endpoint. The `sub` claim is an opaque user ID string in both cases. At the Rust level the providers are nearly identical — the only meaningful difference is audience validation and JWKS cache invalidation strategy.

---

## Step 1 — Add `jsonwebtoken` dependency

**File:** `Cargo.toml`

```toml
jsonwebtoken = "9"
```

**Why `jsonwebtoken`:** The de facto standard JWT crate in the Rust ecosystem. Supports RS256 natively, handles JWKS key decoding, and has active maintenance. Both Auth0 and Clerk use RS256 by default.

**Alternative considered — `jwt-simple`:**
- Pro: simpler, more ergonomic API surface
- Con: smaller community, fewer published integrations, less battle-tested on RS256/JWKS workflows
- Con: `jsonwebtoken` is the crate docs and examples from both Auth0 and Clerk reference for Rust
- **Rejected** — not a strong opinion, but `jsonwebtoken` has a larger adoption signal

**Alternative considered — `josekit`:**
- Pro: full JOSE spec support
- Con: significantly more complex API than needed for JWT Bearer validation
- **Rejected** — over-engineered for this use case

---

## Step 2 — Add shared primitives

**File:** `src/shared/core/primitives.rs`

```rust
pub struct UserId(pub String);    // sub claim — opaque string from auth provider
pub struct TenantId(pub String);

pub enum Role { Admin, Manager, Employee }

/// Resolved at the HTTP boundary for the lifetime of one request.
pub struct AuthenticatedUser {
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub role: Role,
}
```

**Why `UserId(pub String)`:** The sub claim format differs between providers (`auth0|64a...` for Auth0, `user_2abc...` for Clerk) but this system treats it as an opaque identifier either way. A newtype wrapper makes the identity explicit in function signatures without encoding provider-specific format assumptions.

**Why `AuthenticatedUser` is request-scoped:** It is a resolved snapshot — not domain state. The IAM module holds the authoritative domain state (membership, roles). `AuthenticatedUser` is the shell's view of one authenticated, authorized request.

**Alternative considered — embed provider-specific `UserId` types per module:**
- Pro: no shared dependency between modules
- Con: shell cannot reference a common identity type when building middleware; cross-module events cannot refer to a common actor
- **Rejected**

**Alternative considered — store roles in the JWT (Auth0 RBAC or Clerk metadata):**
- Pro: no IAM projection lookup per request; `AuthenticatedUser.role` comes free
- Con: roles become a property of the auth provider, not this system. Tenant-specific role changes require Auth0/Clerk Management API round-trips rather than domain events in the IAM module. Authorization logic is split across two systems.
- **Rejected** — this system owns authorization; the auth provider owns authentication only

---

## Step 3 — Define `IamAuthorizationPort` in shared infrastructure

**File:** `src/shared/infrastructure/auth/mod.rs`

```rust
#[async_trait]
pub trait IamAuthorizationPort: Send + Sync {
    async fn resolve(&self, user_id: &UserId, tenant_id: &TenantId) -> Option<Role>;
}
```

**Why a port in `shared/infrastructure/`:** The shell middleware must query the IAM module's projection to resolve a role, but the shell must not depend on IAM's concrete types. A port in `shared/infrastructure/` is the established pattern for exactly this — the same as `EventStore` and `DomainOutbox`. The shell wires the concrete IAM implementation in; the middleware depends only on the port.

**Why not import IAM's query handler directly in the shell:**
- The shell would then compile-depend on IAM's internal types. Any IAM refactor could break the shell. The port decouples them at the type level.

**Alternative considered — put the port in `src/modules/iam/`:**
- Pro: co-located with its implementation
- Con: other modules (or the shell) importing `iam::IamAuthorizationPort` creates an upward dependency from infrastructure back into a module — violates the layer hierarchy
- **Rejected**

---

## Step 4 — Implement JWT validation

**File:** `src/shared/infrastructure/auth/jwt.rs`

This is the only file where the provider matters.

### Provider-agnostic design

Use three environment variables so the file is identical for both providers:

```
JWT_JWKS_URL=...      # JWKS endpoint URL
JWT_ISSUER=...        # expected `iss` claim
JWT_AUDIENCE=...      # expected `aud` claim — leave empty to skip
```

```rust
pub struct JwtValidator {
    jwks_url: String,
    issuer: String,
    audience: Option<String>,
    cached_keys: RwLock<HashMap<String, DecodingKey>>,
}

impl JwtValidator {
    pub fn validate(&self, token: &str) -> Result<UserId, JwtError> {
        let header = decode_header(token)?;
        let kid = header.kid.ok_or(JwtError::MissingKid)?;
        let key = self.get_or_fetch_key(&kid)?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        if let Some(aud) = &self.audience {
            validation.set_audience(&[aud]);
        }

        let data = decode::<Claims>(token, &key, &validation)?;
        Ok(UserId(data.claims.sub))
    }

    fn get_or_fetch_key(&self, kid: &str) -> Result<DecodingKey, JwtError> {
        // Check cache first
        if let Some(key) = self.cached_keys.read().unwrap().get(kid) {
            return Ok(key.clone());
        }
        // Cache miss: re-fetch JWKS (handles Clerk rotation, not just TTL expiry)
        let keys = fetch_jwks(&self.jwks_url)?;
        let mut cache = self.cached_keys.write().unwrap();
        for (id, key) in keys {
            cache.insert(id, key);
        }
        cache.get(kid).cloned().ok_or(JwtError::UnknownKid)
    }
}
```

### Auth0 configuration

```env
JWT_JWKS_URL=https://your-tenant.auth0.com/.well-known/jwks.json
JWT_ISSUER=https://your-tenant.auth0.com/
JWT_AUDIENCE=https://api.your-app.com
```

Notes:
- Issuer trailing slash is required by Auth0
- Audience must match the API identifier configured in the Auth0 dashboard
- JWKS keys rotate infrequently; `kid`-based cache invalidation is sufficient

### Clerk configuration

```env
JWT_JWKS_URL=https://your-domain.clerk.accounts.dev/.well-known/jwks.json
JWT_ISSUER=https://your-domain.clerk.accounts.dev
JWT_AUDIENCE=
```

Notes:
- No audience claim is needed for Clerk backend API JWTs
- Tokens are short-lived (~60s); Clerk rotates signing keys more aggressively than Auth0
- The `kid`-based re-fetch strategy handles this correctly — on unknown `kid`, re-fetch immediately rather than waiting for a TTL

**Why `kid`-based invalidation instead of TTL:**
- A TTL cache could serve stale keys for the duration of the TTL window during a key rotation
- Re-fetching on unknown `kid` is zero-cost for the happy path (cache hit) and handles rotation correctly for both providers
- Clerk's documentation explicitly recommends this strategy

**Alternative considered — TTL-only cache (e.g. 1-hour expiry):**
- Pro: simpler implementation; acceptable for Auth0 where rotation is rare
- Con: during a Clerk key rotation, tokens signed with the new key would be rejected until the TTL expires
- **Rejected** — `kid`-based invalidation is strictly better and not significantly more complex

**Alternative considered — no caching (fetch JWKS on every request):**
- Pro: always up-to-date
- Con: adds a network round-trip to every authenticated request; JWKS endpoints have rate limits
- **Rejected**

---

## Step 5 — Auth middleware in the shell

**File:** `src/shell/auth_middleware.rs` (new)

Tower middleware that runs on every authenticated route:

1. Extracts `Authorization: Bearer <token>` header → validates JWT via `JwtValidator` → extracts `UserId`
2. Reads `X-Tenant-Id` header → wraps as `TenantId`
3. Calls `IamAuthorizationPort::resolve(user_id, tenant_id)` → gets `Role`
4. Stores `AuthenticatedUser` in Axum request extensions
5. Returns `401` for invalid/missing JWT; `403` if no membership found

**File:** `src/shell/http.rs`

```rust
Router::new()
    .nest("/time-entries",
        Router::new()
            .route("/register", post(register_http::handle))
            .layer(RequireRole::new(Role::Employee))
    )
    .nest("/iam/tenants/:id/members",
        Router::new()
            .route("/", post(add_member_http::handle))
            .layer(RequireRole::new(Role::Admin))
    )
```

**Why coarse-grained access control at the route layer:**
The question of "does this user have any access to this tenant at all?" is infrastructure policy, not domain logic. It applies uniformly across every command in a route group. Moving it into individual deciders would mean every decider across every module must receive and validate IAM data — duplicated boilerplate that mixes transport policy with domain logic.

Fine-grained rules (e.g. "a Manager can only delegate within their own permission scope") remain in the IAM decider where they belong.

**Alternative considered — check role entirely inside each decider:**
- Pro: all authorization logic in one consistent, testable place (pure core)
- Con: every decider in every module must accept role data; command structs must carry `AuthenticatedUser`; boilerplate scales with module count
- Con: mixes transport-level access control (coarse) with domain rules (fine-grained)
- **Rejected for coarse-grained access**; accepted for fine-grained rules inside deciders

**Alternative considered — per-handler role check (no middleware):**
- Pro: explicit and visible in the handler
- Con: easily forgotten; not enforced by the type system; becomes inconsistent as the handler count grows
- **Rejected**

---

## Step 6 — Update `time_entries` inbound adapters

**Files:** `src/modules/time_entries/use_cases/*/inbound/http.rs`

Replace `created_by: "user-from-auth".into()` with extraction from Axum request extensions:

```rust
pub async fn handle(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    // ...
) -> impl IntoResponse {
    let command = RegisterTimeEntry {
        // ...
        created_by: user.user_id.0,
    };
}
```

**Why this is the last step:** The middleware must be wired and `AuthenticatedUser` must be in scope before inbound adapters can use it. Changing adapters first would break the build.

---

## Files to create / modify

| Path | Action |
|---|---|
| `Cargo.toml` | Add `jsonwebtoken = "9"` |
| `src/shared/core/primitives.rs` | Add `UserId`, `TenantId`, `Role`, `AuthenticatedUser` |
| `src/shared/infrastructure/auth/mod.rs` | Add `IamAuthorizationPort` trait |
| `src/shared/infrastructure/auth/jwt.rs` | New — provider-agnostic JWT validation |
| `src/shell/auth_middleware.rs` | New — Tower middleware |
| `src/shell/http.rs` | Add `RequireRole` layer to route groups |
| `src/shell/state.rs` | Add `jwt_validator`, `iam_queries` fields |
| `src/shell/main.rs` | Wire JWT validator and IAM authorization port |
| `src/modules/time_entries/**/inbound/http.rs` | Extract `AuthenticatedUser` from extensions |
| `.env.example` | Document `JWT_JWKS_URL`, `JWT_ISSUER`, `JWT_AUDIENCE` |

---

## Verification

```bash
cargo run-script fmt
cargo run-script lint
cargo run-script test
cargo run-script coverage   # 100% function / line / region thresholds
```

Manual smoke test (swap in Auth0 or Clerk JWT as appropriate):

1. Obtain a test JWT from the configured provider
2. `POST /iam/tenants` with bearer token → `201`, `TenantCreated` event persisted
3. `POST /iam/tenants/:id/members` with Admin JWT → `201`
4. `POST /time-entries/register` with `X-Tenant-Id` header and Employee JWT → `201`
5. Same request without tenant membership → `403`
6. Request with expired or tampered JWT → `401`
7. Request without `Authorization` header → `401`
8. Second `register_user` for same `sub` → non-error rejection, no duplicate event
