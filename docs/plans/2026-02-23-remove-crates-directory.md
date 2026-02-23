# Remove Crates Directory Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Flatten the workspace structure by removing the `crates/` indirection, making the root `Cargo.toml` the single package manifest.

**Architecture:** The workspace currently has one member (`crates/time_entries`). Since there is no benefit to a workspace with a single crate, we collapse it: move `src/` and `Cargo.toml` up to the repo root, then delete `crates/`. The module structure inside `src/` is unchanged.

**Tech Stack:** Rust, Cargo workspace → single-crate project

---

### Task 1: Replace root Cargo.toml with crate Cargo.toml

**Files:**
- Modify: `Cargo.toml` (root)

**Step 1: Replace contents**

Replace the root `Cargo.toml` (which is currently a workspace manifest) with the contents of `crates/time_entries/Cargo.toml`. The new root `Cargo.toml` should be exactly:

```toml
[package]
name = "time_entries"
version = "0.1.0"
description = "Backend API to register time entries."
edition = "2024"
readme = "README.md"
repository = "https://github.com/jvhellemondt/api-time-registration.rs"

[package.metadata.scripts]
fmt = "cargo fmt --all -- --check"
fmt-fix = "cargo fmt --all"
lint = "cargo clippy --all-targets --all-features"
test = "cargo nextest run --workspace --retries 2"
test-integration = "cargo nextest run --workspace --retries 2 -- --ignored integration"
coverage = "cargo llvm-cov nextest --workspace --ignore-filename-regex \"(src/main\\.rs|/graphql\\.rs)\" --fail-under-functions 100 --fail-under-lines 100 --fail-under-regions 100 --show-missing-lines"

[dev-dependencies]
dotenvy = "0.15.7"
rstest = "0.26.1"

[dependencies]
anyhow = "1.0.100"
async-trait = "0.1.89"
chrono = "0.4.43"
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
thiserror = "2.0.18"
uuid = { version = "1.20.0", features = ["v7", "serde"] }
axum = "0.8.8"
async-graphql = "7.2.1"
async-graphql-axum = "7.2.1"
tower-http = { version = "0.6.8", features = ["trace"] }
tracing = "0.1.44"
tracing-subscriber = { version = "0.3.22", features = ["fmt", "env-filter"] }
tokio = { version = "1.49.0", features = ["rt", "rt-multi-thread", "macros", "sync", "time"] }
```

**Step 2: Verify build**

```bash
cargo build
```

Expected: compile error about missing `src/` (it still lives in `crates/time_entries/src/`). That's fine — proceed.

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "refactor: replace workspace Cargo.toml with single-crate manifest"
```

---

### Task 2: Move src/ to root

**Files:**
- Move: `crates/time_entries/src/` → `src/`

**Step 1: Move the directory**

```bash
mv crates/time_entries/src src
```

**Step 2: Verify build**

```bash
cargo build
```

Expected: successful build with no errors.

**Step 3: Run tests**

```bash
cargo nextest run --workspace --retries 2
```

Expected: all tests pass.

**Step 4: Commit**

```bash
git add src/
git commit -m "refactor: move src/ from crates/time_entries/ to root"
```

---

### Task 3: Delete crates/ directory

**Files:**
- Delete: `crates/`

**Step 1: Remove the directory**

```bash
rm -rf crates
```

**Step 2: Verify build and tests still pass**

```bash
cargo build && cargo nextest run --workspace --retries 2
```

Expected: successful build, all tests pass.

**Step 3: Commit**

```bash
git add -A
git commit -m "refactor: delete crates/ directory"
```

---

### Task 4: Update CLAUDE.md

**Files:**
- Modify: `CLAUDE.md`

**Step 1: Update the Commands section**

Change the line:

```
Scripts are defined in `[package.metadata.scripts]` in `crates/time_entries/Cargo.toml`. Run from `crates/time_entries/`:
```

To:

```
Scripts are defined in `[package.metadata.scripts]` in `Cargo.toml`. Run from the repo root:
```

Also update the Workspace Structure section — change:

```
crates/
  time_entries/        # Functional core + application + shell (bounded context)
  time_entries_api/    # HTTP entry point (Axum + async-graphql)
```

To:

```
src/
  modules/             # Bounded contexts (e.g. time_entries)
  shared/              # Cross-cutting primitives and infrastructure
  shell/               # Wiring, startup, workers
  tests/               # E2E tests and fixtures
```

**Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md for flat project structure"
```

---

### Task 5: Update README.md

**Files:**
- Modify: `README.md`

**Step 1: Remove the outdated crates structure reference**

Remove the line:

```
- crates/time_entries: the time entries bounded context.
```

And replace the "Structure" section with the flat layout:

```
Structure
- src/modules/: bounded contexts (currently time_entries)
- src/shared/: cross-cutting primitives and infrastructure
- src/shell/: wiring and startup
- src/tests/: E2E tests and fixtures
```

Also update the scripts section — remove the instruction to `cd` into the crate:

Change:
```
`cd` into the crate in which the `cargo.toml` that contains the "package.metadata.scripts"-block and run:
```

To:
```
From the repo root, run:
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: update README for flat project structure"
```

---

### Task 6: Run full checks

**Step 1: Format check**

```bash
cargo run-script fmt
```

Expected: no formatting issues.

**Step 2: Lint**

```bash
cargo run-script lint
```

Expected: no clippy warnings.

**Step 3: Tests with coverage**

```bash
cargo run-script coverage
```

Expected: 100% functions, lines, regions.
