# Plan: Remove Arc from Application Layer

## Context

The codebase uses FCIS (Functional Core, Imperative Shell). Currently, `Arc` leaks into the application layer — handlers store `Arc<TStore>` fields and require callers to pass `Arc<T>` at construction. This means the application layer is making shared-ownership decisions that belong to the shell.

The goal is to make `Arc` invisible above the infrastructure layer. The shell (`main.rs`, `AppState`) is the only place that should know or care about how dependencies are shared across concurrent request handlers.

---

## Principle: Arc belongs inside infra structs, not outside them

All three in-memory stores (`InMemoryEventStore`, `InMemoryDomainOutbox`, `InMemoryProjectionStore`) currently hold a bare `RwLock`/`Mutex`. This means they are not `Clone`, which forces every caller to wrap them in `Arc` externally.

The fix: wrap the internal state in `Arc<Inner>` inside each struct. The struct becomes a thin, cheaply-cloneable handle — cloning it bumps a reference count, not the data. Callers no longer need to know about `Arc` at all.

This is the "newtype holds the Arc" pattern. The struct *is* the shared handle.

---

## Changes

### 1. `InMemoryEventStore` — Arc inside, make Clone

**Why:** The store is currently not `Clone` because `RwLock` is not `Clone`. Every consumer must wrap it in `Arc` externally. Moving the `Arc` inside makes the store self-contained and removes the coupling between callers and ownership mechanics.

**What:**
- Introduce a private `struct Inner<Event>` that holds everything that was previously in the top-level struct: the `RwLock<InnerState<Event>>`, `is_offline`, `delay_append_ms`, and `sender`
- Change `InMemoryEventStore` to a single-field struct: `inner: Arc<Inner<Event>>`
- Derive `Clone` — cloning just increments the refcount on the `Arc`
- `is_offline` changes from `bool` to `AtomicBool` so it can be mutated through a shared `Arc` via `&self`. `toggle_offline` drops the `&mut self` requirement and uses `AtomicBool::fetch_xor(true, Ordering::SeqCst)`
- `delay_append_ms` is already `AtomicU64` so it stays the same, just moved inside `Inner`
- `sender` moves inside `Inner` as well

**File:** `src/shared/infrastructure/event_store/in_memory.rs`

---

### 2. `InMemoryDomainOutbox` — Arc inside, make Clone

**Why:** Same reason as above. Callers currently must `Arc::new()` it before passing to the handler. If the outbox is cheaply `Clone`, the handler and `AppState` can hold it by value.

**What:**
- Introduce private `struct Inner` containing the two `Mutex` fields (`rows`, `seen`)
- Change `InMemoryDomainOutbox` to `inner: Arc<Inner>`
- Derive `Clone`

**File:** `src/shared/infrastructure/intent_outbox/in_memory.rs`

---

### 3. `InMemoryProjectionStore` — Arc inside, make Clone

**Why:** Same pattern. The projection store is shared between the projector (writer) and the query handler (reader). Currently this sharing is expressed via `Arc` at the call site in `main.rs`. After this change, cloning the store gives you another handle to the same data — no explicit `Arc` needed.

**What:**
- Introduce private `struct Inner<P>` containing `RwLock<InnerState<P>>` and `is_offline: AtomicBool`
- Change `InMemoryProjectionStore` to `inner: Arc<Inner<P>>`
- Derive `Clone`
- `toggle_offline` drops `&mut self`, uses `AtomicBool`

**File:** `src/shared/infrastructure/projection_store/in_memory.rs`

---

### 4. `RegisterTimeEntryHandler` — receive deps at call site, not stored

**Why:** The handler currently stores `Arc<TEventStore>` and `Arc<TOutbox>`. This means the handler owns infrastructure, which is the shell's responsibility. In FCIS, a command handler is an orchestrator — it takes a command and some infrastructure, runs the core functions, and persists the result. It shouldn't hold a long-lived reference to infrastructure. Passing deps at the call site makes this explicit: the handler has no memory between calls, and all its inputs are visible at the call site.

**What:**
- Remove the generic params `TEventStore` and `TOutbox` from the struct definition
- Remove `event_store: Arc<TEventStore>` and `outbox: Arc<TOutbox>` fields
- The struct becomes `RegisterTimeEntryHandler { topic: String }` — trivially `Clone`
- Remove the `Arc` import
- Change `handle` to accept the stores as parameters:
  ```rust
  pub async fn handle(
      &self,
      event_store: &impl EventStore<TimeEntryEvent>,
      outbox: &impl DomainOutbox,
      stream_id: &str,
      command: RegisterTimeEntry,
  ) -> Result<(), ApplicationError>
  ```
- The body stays the same — just replace `self.event_store` with `event_store` and `self.outbox` with `outbox`

**Note on tests:** Tests that previously did `RegisterTimeEntryHandler::new(TOPIC, Arc::new(event_store), Arc::new(outbox))` will now do `RegisterTimeEntryHandler::new(TOPIC)` and pass stores directly to `handle()`.

**File:** `src/modules/time_entries/use_cases/register_time_entry/handler.rs`

---

### 5. `ListTimeEntriesQueryHandler` — hold store by value, derive Clone

**Why:** The handler stores `Arc<TStore>` only because `TStore` was not `Clone`. Now that `InMemoryProjectionStore` is cheaply `Clone`, the handler can hold `TStore` directly. Unlike the command handler, the query handler reasonably holds its store — it's tied to exactly one projection. There's no benefit to passing the store at every call site.

**What:**
- Change `store: Arc<TStore>` → `store: TStore`
- Constructor takes `store: TStore` (not `Arc<TStore>`)
- Derive `Clone` — works because `TStore: Clone` now

**File:** `src/modules/time_entries/use_cases/list_time_entries_by_user/queries.rs`

---

### 6. `ListTimeEntriesProjector` — hold stores by value

**Why:** Same as the query handler. The projector held `Arc<TStore>` and `Arc<InMemoryEventStore>` because those types weren't `Clone`. Now they are. The projector is passed by value to `projector_runner::spawn()` which moves it into an async task — this works fine as long as the stores are `Send + 'static`, which they are (the `Arc<Inner>` inside is both).

**What:**
- Change `store: Arc<TStore>` → `store: TStore`
- Change `event_store: Arc<InMemoryEventStore<TimeEntryEvent>>` → `event_store: InMemoryEventStore<TimeEntryEvent>`
- Constructor takes values, not `Arc<T>`

**File:** `src/modules/time_entries/use_cases/list_time_entries_by_user/projector.rs`

---

### 7. `AppState` — concrete types, add outbox field

**Why:** With the above changes, the outer `Arc` wrappers on `AppState` fields are no longer necessary. Each field is either directly `Clone` or justified.

The one `Arc` that stays: `queries: Arc<dyn TimeEntryQueries + Send + Sync>`. This is unavoidable — `dyn Trait` objects are not `Clone`, and `AppState` must be `Clone` for Axum's `State<T>` extractor. `Arc<dyn Trait>` is the standard way to make a trait object cloneable.

The `outbox` becomes a new field because `RegisterTimeEntryHandler` no longer holds it — it must live somewhere that's accessible at the call site.

**What:**
```rust
#[derive(Clone)]
pub struct AppState {
    pub queries: Arc<dyn TimeEntryQueries + Send + Sync>, // Arc required: dyn + Clone
    pub register_handler: RegisterTimeEntryHandler,       // just { topic: String }
    pub event_store: InMemoryEventStore<TimeEntryEvent>,  // Clone via Arc<Inner> inside
    pub outbox: InMemoryDomainOutbox,                     // Clone via Arc<Inner> inside (new)
}
```

**File:** `src/shell/state.rs`

---

### 8. Inbound adapters — pass stores at call site

**Why:** The HTTP and GraphQL handlers for `register_time_entry` called `state.register_handler.handle(stream_id, command)`. Now the handler needs stores passed in.

**What (http.rs and graphql.rs for register_time_entry):**
```rust
state.register_handler.handle(
    &state.event_store,
    &state.outbox,
    &stream_id,
    command,
).await
```

**Tests:** Remove `Arc::new()` wrapping around stores and handlers. Add `outbox` to `AppState` construction.

**Files:**
- `src/modules/time_entries/use_cases/register_time_entry/inbound/http.rs`
- `src/modules/time_entries/use_cases/register_time_entry/inbound/graphql.rs`
- `src/modules/time_entries/use_cases/list_time_entries_by_user/inbound/http.rs` (tests only)

---

### 9. `main.rs` — simplified wiring

**Why:** The shell is where things are wired together. With Arc inside the infra types, `main.rs` just constructs values and moves them into the structs that need them. The `projection_store` is cloned once to share between the projector and query handler — that clone is just a refcount bump.

**What:**
- Remove all `Arc::new()` calls around stores and handlers
- Clone `projection_store` to pass to both projector and query handler (cheap)
- Add `outbox` to `AppState { ... }`

**File:** `src/shell/main.rs`

---

## Verification

```bash
cargo run-script fmt
cargo run-script lint
cargo run-script test
cargo run-script coverage
```

Coverage is enforced at 100% — all tests must pass including the offline/error path tests that use `toggle_offline`.
