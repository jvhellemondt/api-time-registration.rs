# IAM Bounded Context

**Target file:** `docs/plans/2026-03-15-iam-bounded-context.md`

## Context

Auth0 is used for **authentication only** (JWT, `sub` claim = identity). This system owns **authorization**. The system is **multi-tenant**: data belongs to exactly one tenant, multiple users can have permissions per tenant, a company may optionally own tenants but is not required to.

Tenant membership, roles, delegation, user settings, and tenant settings are all domain state this system must own. The empty placeholder `src/modules/iam/` already exists. `shared/core/primitives.rs` is currently empty. All inbound adapters currently hardcode `"user-from-auth"` as a placeholder.

---

## Decision: IAM is its own bounded context

**Why:** IAM has its own aggregates (`Tenant`, `User`), its own lifecycle of domain events, and business rules that live in a decider. It is not merely infrastructure. Authorization decisions — who can add members, who can delegate — are domain logic, not configuration. A module boundary makes IAM independently evolvable without touching other modules.

**Alternative considered — put everything in `shared/`:**
- Pro: simpler dependency graph; no inter-module wiring needed
- Pro: IAM types available everywhere without ports
- Con: `shared/` is supposed to hold cross-cutting primitives, not a full domain with aggregates and business rules. Putting a domain in `shared/` collapses the bounded context boundary and creates a god module.
- Con: domain rules become unreachable by tests without importing the entire shared layer
- **Rejected** because IAM has genuine domain complexity and the architecture explicitly disallows domain logic in `shared/`

---

## Step 1 — Add shared primitives

**File:** `src/shared/core/primitives.rs`

```rust
pub struct UserId(pub String);    // Auth0 sub claim — opaque to this system
pub struct TenantId(pub String);

pub enum Role { Admin, Manager, Employee }

/// Resolved at the HTTP boundary. Passed into every command that needs identity.
pub struct AuthenticatedUser {
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub role: Role,
}
```

**Why:** These types are used by every module's commands and by the auth middleware. They belong in `shared/core/primitives` — exactly the case the ADR describes ("a type needed by two or more modules"). They carry no domain logic; they are value objects.

**Why not a richer `User` type here:** User settings and delegation are IAM domain state, not primitives. `AuthenticatedUser` is a resolved snapshot for the lifetime of one request only.

**Alternative considered — each module defines its own `UserId`:**
- Pro: no shared dependency
- Con: cross-module events and the shell cannot refer to a common identity without re-typing
- **Rejected**

---

## Step 2 — Add `IamAuthorizationPort` to shared infrastructure

**File:** `src/shared/infrastructure/auth/mod.rs`

```rust
#[async_trait]
pub trait IamAuthorizationPort: Send + Sync {
    async fn resolve(&self, user_id: &UserId, tenant_id: &TenantId) -> Option<Role>;
}
```

**File:** `src/shared/infrastructure/auth/jwt.rs` — validates Auth0 JWT, extracts `sub` as `UserId`

**Why:** The auth middleware (in the shell) must query IAM to resolve a user's role without depending on the IAM module directly. A port in `shared/infrastructure/` is the established pattern for exactly this: the shell wires the concrete IAM implementation in, keeping modules independent. This is the same pattern as `EventStore` and `DomainOutbox`.

**Why not import the IAM query handler directly in the shell:**
- The shell would then need to know IAM internals, coupling it to IAM's concrete types. Port abstraction keeps that coupling at the wiring boundary only.

**Alternative considered — embed IAM resolution in the JWT extractor (stateless, claims-only):**
- Pro: no IAM query on every request; faster
- Con: requires roles to be embedded in Auth0 JWT claims, which contradicts the decision to use Auth0 for authentication only. Putting roles in Auth0 tokens re-centralizes authorization in Auth0 and makes this system dependent on Auth0 for authorization decisions.
- **Rejected**

---

## Step 3 — Create the IAM module

### Folder structure

```
src/modules/iam/
  mod.rs
  core/
    events.rs           # IamEvent enum
    events/v1/          # one file per versioned event
    state.rs            # TenantState, UserState
    evolve.rs
    intents.rs
    projections.rs
  use_cases/
    create_tenant/
      command.rs
      decision.rs
      decide.rs
      handler.rs
      inbound/http.rs
    add_member/
    remove_member/
    change_role/
    grant_delegation/
    revoke_delegation/
    update_tenant_settings/
    register_user/        # triggered auto on first request
    update_user_settings/
    resolve_permissions/  # query: (user_id, tenant_id) → Role
      queries.rs
      queries_port.rs     # IamQueries trait; implements IamAuthorizationPort
      projection.rs       # PermissionsProjection
      projector.rs
      handler.rs
  adapters/
    outbound/
      event_store.rs
      intent_outbox.rs
```

### Events

```
TenantCreated          { tenant_id, name, created_by, created_at }
MemberAdded            { tenant_id, user_id, role, added_by, added_at }
MemberRemoved          { tenant_id, user_id, removed_by, removed_at }
RoleChanged            { tenant_id, user_id, new_role, changed_by, changed_at }
DelegationGranted      { tenant_id, delegator_id, delegate_id, expires_at }
DelegationRevoked      { tenant_id, delegator_id, delegate_id }
TenantSettingUpdated   { tenant_id, key, value, updated_by, updated_at }
UserRegistered         { user_id, auth0_sub, registered_at }
UserSettingUpdated     { user_id, key, value, updated_at }
```

**Why two aggregate streams:**
- `tenant:{tenant_id}` — membership and settings decisions are per-tenant
- `user:{user_id}` — user settings and delegation grants are per-user
- Keeping them separate avoids a wide state that mixes unrelated concerns and allows independent evolution

**Alternative considered — one aggregate stream per (user, tenant) pair:**
- Pro: membership and user-specific state co-located
- Con: user settings (locale, notification prefs) are not tenant-scoped. A user setting update would require a tenant context that doesn't make sense. Also makes queries that need all members of a tenant expensive (must scan all user streams).
- **Rejected**

### `register_user` auto-registration

**Why:** Users come from Auth0. This system must create a `UserRegistered` event the first time a known `sub` is seen, so user-specific domain state (settings, delegations) has a stream to attach to. This is idempotent — a duplicate `RegisterUser` command results in a non-error rejection from the decider.

**Alternative considered — no explicit `UserRegistered` event; treat user existence as implicit:**
- Pro: simpler
- Con: user settings and delegation have no stream to anchor to. Without a `UserRegistered` event, querying user state is undefined. Event sourcing requires an explicit aggregate creation event.
- **Rejected**

---

## Step 4 — Route-level authorization middleware

**Files:** `src/shell/http.rs`, new `src/shell/auth_middleware.rs`

The middleware runs on every authenticated route:
1. Validates Auth0 JWT (`shared/infrastructure/auth/jwt.rs`)
2. Reads `X-Tenant-Id` header
3. Calls `IamAuthorizationPort::resolve(user_id, tenant_id)`
4. Stores `AuthenticatedUser` in request extensions
5. Returns `401` if JWT invalid, `403` if no membership found

`RequireRole` is a typed Tower layer applied per route group:

```rust
// shell/http.rs
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

**Why route-level rather than in the decider:**
- Coarse-grained access control (does this user have *any* access to this tenant?) is infrastructure policy, not domain logic. Enforcing it at the route layer avoids leaking it into deciders across every module.
- Fine-grained rules (e.g. can a Manager only delegate within their own permission scope) remain in the IAM decider where they belong.

**Alternative considered — check role entirely inside each decider:**
- Pro: all authorization logic in one consistent place (pure core)
- Con: every decider in every module must receive and validate IAM data. As modules grow, this creates duplicated authorization boilerplate and mixes transport-level access control with domain logic.
- **Rejected for coarse-grained access.** Fine-grained rules (delegation scope, tenant-specific policies) stay in deciders.

**Alternative considered — use Auth0 RBAC (roles in JWT claims):**
- Pro: no IAM projection lookup per request; roles come free with the JWT
- Con: contradicts the decision to use Auth0 for authentication only. Distributing authorization to Auth0 makes the auth model split across two systems and makes tenant-specific role changes dependent on Auth0 Management API round-trips.
- **Rejected**

---

## Step 5 — Wire in the shell

**Files:** `src/shell/main.rs`, `src/shell/state.rs`

- Add IAM event store (`InMemoryEventStore<IamEvent>`)
- Add IAM outbox
- Instantiate IAM command handlers and query handler
- Add `iam_queries: Arc<dyn IamAuthorizationPort>` to `AppState`
- Pass into auth middleware via `AppState`
- Mount IAM HTTP routes

**Why AppState carries the port, not the concrete type:**
- Consistent with existing pattern (`queries: Arc<dyn TimeEntryQueries>`)
- Allows swapping in-memory with a real DB implementation without touching handlers

---

## Step 6 — Update `time_entries` inbound adapters

**Files:** `src/modules/time_entries/use_cases/*/inbound/http.rs`

Extract `AuthenticatedUser` from request extensions (placed by auth middleware) instead of the current hardcoded `"user-from-auth"`. Pass `user_id` and `tenant_id` into commands.

**Why this is step 6 and not step 1:** IAM must exist and the middleware must be wired before inbound adapters can rely on it. Changing adapters first would break the build.

---

## Step 7 — Add dependency

**File:** `Cargo.toml`

```toml
jsonwebtoken = "9"
```

**Why `jsonwebtoken`:** Industry-standard Rust JWT library. Supports RS256 (Auth0's default signing algorithm) and JWKS key rotation.

**Alternative considered — `jwt-simple`:**
- Pro: simpler API
- Con: less widely used; `jsonwebtoken` is the de facto standard in the Rust ecosystem with Context7-verified docs
- **Not a strong opinion; either works**

---

## New Dependency to Add

```toml
jsonwebtoken = "9"
```

No password hashing is needed — Auth0 owns credentials.

---

## Files to Create / Modify

| Path | Action |
|---|---|
| `src/shared/core/primitives.rs` | Add `UserId`, `TenantId`, `Role`, `AuthenticatedUser` |
| `src/shared/infrastructure/auth/mod.rs` | Add `IamAuthorizationPort` trait |
| `src/shared/infrastructure/auth/jwt.rs` | Auth0 JWT validation |
| `src/modules/iam/**` | Create full module (see structure above) |
| `src/shell/http.rs` | Add `RequireRole` middleware, route groups |
| `src/shell/auth_middleware.rs` | New — Tower middleware impl |
| `src/shell/state.rs` | Add IAM handler fields |
| `src/shell/main.rs` | Wire IAM infrastructure |
| `src/modules/time_entries/**/inbound/http.rs` | Extract `AuthenticatedUser` from extensions |
| `Cargo.toml` | Add `jsonwebtoken` |

---

## Verification

```bash
cargo run-script fmt
cargo run-script lint
cargo run-script test
cargo run-script coverage   # 100% function / line / region thresholds
```

Manual smoke:
1. Obtain Auth0 JWT for a test user
2. `POST /iam/tenants` → `201`, `TenantCreated` event in store
3. `POST /iam/tenants/:id/members` with Admin JWT → `201`
4. `POST /time-entries/register` with `X-Tenant-Id` header and Employee JWT → `201`
5. Same request without membership → `403`
6. Request with expired/invalid JWT → `401`
7. Second `register_user` command for same `sub` → non-error rejection, no duplicate event
