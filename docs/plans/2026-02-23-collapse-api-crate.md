# Collapse time_entries_api into time_entries

## Goal

Remove the separate `time_entries_api` crate. Move the GraphQL inbound adapter into
`modules/time_entries/adapters/inbound/graphql.rs` (library) and the binary entry point
into `src/main.rs` (binary target of the `time_entries` crate).

## Rust crate model

A crate can have both `lib.rs` (library) and `main.rs` (binary). The binary compiles as a
separate unit and accesses the library via `use time_entries::...`. Modules declared only
in `main.rs` are private to the binary.

## Tasks

### Task 1 — Add deps to time_entries/Cargo.toml

Edit `crates/time_entries/Cargo.toml`:
- Add to `[dependencies]`:
  ```toml
  axum = "0.8.8"
  async-graphql = "7.2.1"
  async-graphql-axum = "7.2.1"
  tower-http = { version = "0.6.8", features = ["trace"] }
  tracing = "0.1.44"
  tracing-subscriber = { version = "0.3.22", features = ["fmt", "env-filter"] }
  ```
- Update `tokio` features to add `rt-multi-thread`:
  ```toml
  tokio = { version = "1.49.0", features = ["rt", "rt-multi-thread", "macros", "sync", "time"] }
  ```

Verification: `cargo build -p time_entries` compiles (no new Rust files yet, just dep
resolution).

### Task 2 — Create inbound graphql adapter

Create `crates/time_entries/src/modules/time_entries/adapters/inbound/mod.rs`:
```rust
pub mod graphql;
```

Create `crates/time_entries/src/modules/time_entries/adapters/inbound/graphql.rs`:
- Copy content from `crates/time_entries_api/src/schema.rs`
- Replace every `use time_entries::` with `use crate::` (it's now inside the library)

### Task 3 — Declare inbound module in lib.rs

Edit `crates/time_entries/src/lib.rs`, under `pub mod adapters {`, add alongside
`pub mod outbound { ... }`:
```rust
pub mod inbound {
    pub mod graphql;
}
```

Verification: `cargo build -p time_entries` compiles.

### Task 4 — Create src/main.rs

Create `crates/time_entries/src/main.rs`:
- Copy content from `crates/time_entries_api/src/main.rs`
- Update import for schema types:
  ```rust
  use time_entries::modules::time_entries::adapters::inbound::graphql::{AppSchema, AppState, MutationRoot, QueryRoot};
  ```
- Remove the `mod schema;` / `use crate::schema::...` lines that referenced the old crate
- Keep all other `use time_entries::...` imports unchanged (they already use the right paths)
- Remove `mod schema;` and `use crate::schema::AppState;` etc. — replace with the new
  `use time_entries::modules::time_entries::adapters::inbound::graphql::AppState;`

Verification: `cargo build` (workspace) compiles. `cargo run -p time_entries` starts the
server.

### Task 5 — Update shell/mod.rs comment

Edit `crates/time_entries/src/shell/mod.rs`:
Remove the stale line `// - Expose the HTTP router to time_entries_api.`

### Task 6 — Remove time_entries_api from workspace

Edit root `Cargo.toml`:
Remove `"crates/time_entries_api"` from the `members` array.

### Task 7 — Delete crates/time_entries_api

Delete the entire `crates/time_entries_api/` directory.

Verification: `cargo build` (workspace) still compiles. `cargo nextest run -p time_entries`
passes all 37 tests.

### Task 8 — Run full checks

```bash
cargo run-script fmt-fix   # from crates/time_entries/
cargo run-script lint
cargo run-script test
cargo run-script coverage
```
