# CLAUDE.md

This file provides guidance to CLAUDE when working with code in this repository.

## Commands

Scripts are defined in `[package.metadata.scripts]` in `Cargo.toml`. Run from the repo root:

```bash
cargo build                      # Build the project
cargo run-script fmt             # Check formatting
cargo run-script fmt-fix         # Auto-fix formatting
cargo run-script lint            # Lint with clippy
cargo run-script test            # Run all tests
cargo run-script coverage        # Run tests with coverage
```

Run a single package:

```bash
cargo nextest run -p time_entries
```

Run a single test:

```bash
cargo nextest run -p time_entries list_time_entries_by_user
```

Coverage thresholds are enforced at 100% (functions, lines, regions).

## Architecture

This is a Rust API implementing **Functional Core Imperative Shell (FCIS)** within a **modular vertical slice** structure. The system is fully event-centric — domain events, intents, and technical events are all modelled explicitly as structured types. There is no logging.

### Project Structure

```
src/
  modules/             # Bounded contexts (e.g. time_entries)
  shared/              # Cross-cutting primitives and infrastructure
  shell/               # Wiring, startup, workers
  tests/               # E2E tests and fixtures
```

### Layer Hierarchy

- **`core/`** — Pure functions: events, state, `evolve`, decider (`decide`), projector mappings. No I/O, no side effects.
- **`application/`** — Imperative handlers: command handlers, query handlers, projector runner. Orchestrates core functions with ports.
- **`shell/`** — Wiring: instantiates infrastructure, runs workers (e.g. projector runner).
- **`adapters/`** — Concrete implementations of ports (in-memory event store, outbox, projections).
- **`tests/`** — E2E tests, fixtures.

### Key Patterns

**Decider** (functional event sourcing): `decide(command, state) → Decision` and `evolve(state, event) → state`. Pure functions, co-located with their use case.

**Decision type**: Two variants — `Accepted` carrying events and intents, or `Rejected` carrying a typed domain rejection reason (never a string). Rejection notification intents are added by the command handler, not the decider.

**Command handling lifecycle**: Load past events → fold through `evolve` → call `decide` → persist events + write outbox intents atomically. Rejection writes `InformCallerOfRejection` intent as cross-cutting policy.

**Query handling**: Reads from pre-built projections only. Never touches the event store or command side. Projections are eventually consistent, maintained by a projector that tails the event store.

**Projector**: Tails the event store and applies projection mappings to build read models. Each query use case owns its own projection.

**Three stores**: domain event store (source of truth), intent outbox (at-least-once delivery), technical event store (fire-and-forget I/O observation).

**Outbound adapters**: Named by function. Ports defined in `core/ports.rs`. Implementations named by technology under `adapters/`.

**Inbound adapters**: Named by technology (HTTP, GraphQL). Co-located with their use case.

### Tooling

- **Formatter**: `rustfmt` via `cargo fmt`
- **Linter**: `cargo clippy`
- **Test runner**: `cargo-nextest`
- **Coverage**: `cargo-llvm-cov`

## Rule: always use qmd before reading files

Always search qmd (`qmd` + `search` or `query` or `vsearch` or `get`) before reading files or exploring directories. Fall back to Read/Glob only if qmd returns insufficient results.

## Rule: after completing a plan, run checks

Run `fmt`, `lint`, `test`, and `coverage` (see Commands above) after completing any plan.

## Rule: use Context7 for up-to-date documentation

Always use Context7 MCP for library/API docs, code generation, and setup steps.

## Rule: store prompts in docs/prompts

Store all prompts in `docs/prompts/` with descriptive filenames.

## Rule: use ADRs for architectural decisions

Document significant architectural decisions in `docs/adr/` using the MADR template.
