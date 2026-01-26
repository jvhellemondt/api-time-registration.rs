#!/usr/bin/env bash
set -euo pipefail

# Base repo and crate paths
REPO_DIR="."
CRATE_DIR="$REPO_DIR/crates/time_entries"

# Create directories
mkdir -p "$CRATE_DIR/src"

# Core structure
mkdir -p "$CRATE_DIR/src/core/ports"
mkdir -p "$CRATE_DIR/src/core/time_entry/state"
mkdir -p "$CRATE_DIR/src/core/time_entry/event/v1"
mkdir -p "$CRATE_DIR/src/core/time_entry/decider/register"
mkdir -p "$CRATE_DIR/src/core/time_entry/projector"
mkdir -p "$CRATE_DIR/src/core/queries"
mkdir -p "$CRATE_DIR/src/core/query_handlers"

# Application structure
mkdir -p "$CRATE_DIR/src/application/projector"

# Adapters (in-memory)
mkdir -p "$CRATE_DIR/src/adapters/inmemory"

# Shell workers
mkdir -p "$CRATE_DIR/src/shell/workers"

# Tests
mkdir -p "$CRATE_DIR/tests"

# Top-level files
touch "$REPO_DIR/Cargo.toml"
touch "$CRATE_DIR/Cargo.toml"
touch "$CRATE_DIR/src/lib.rs"

# Core files
# Ports
touch "$CRATE_DIR/src/core/ports.rs"

# Time entry state, events, evolve
touch "$CRATE_DIR/src/core/time_entry/state.rs"
touch "$CRATE_DIR/src/core/time_entry/event/mod.rs"
touch "$CRATE_DIR/src/core/time_entry/event/v1/time_entry_registered.rs"
touch "$CRATE_DIR/src/core/time_entry/evolve.rs"

# Decider (register intent)
touch "$CRATE_DIR/src/core/time_entry/decider/register/command.rs"
touch "$CRATE_DIR/src/core/time_entry/decider/register/decide.rs"
touch "$CRATE_DIR/src/core/time_entry/decider/register/handler.rs"
touch "$CRATE_DIR/src/core/time_entry/decider/register/tests.rs"

# Projector (pure mapping)
touch "$CRATE_DIR/src/core/time_entry/projector/model.rs"
touch "$CRATE_DIR/src/core/time_entry/projector/apply.rs"

# Queries
touch "$CRATE_DIR/src/core/queries/get_time_entry.rs"
touch "$CRATE_DIR/src/core/query_handlers/get_time_entry_handler.rs"

# Application projector
touch "$CRATE_DIR/src/application/projector/repository.rs"
touch "$CRATE_DIR/src/application/projector/runner.rs"

# In-memory adapters
touch "$CRATE_DIR/src/adapters/inmemory/inmem_event_store.rs"
touch "$CRATE_DIR/src/adapters/inmemory/inmem_domain_outbox.rs"
touch "$CRATE_DIR/src/adapters/inmemory/inmem_projections.rs"

# Shell workers (dev helper)
touch "$CRATE_DIR/src/shell/workers/projector_runner.rs"

# Crate tests
touch "$CRATE_DIR/tests/register_decide_tests.rs"
touch "$CRATE_DIR/tests/register_flow_inmem_tests.rs"
touch "$CRATE_DIR/tests/projector_inmem_tests.rs"

echo "Scaffold created under $REPO_DIR"
