# use_cases/register_time_entry folder

Purpose
- Command and decision rules for registering a new time entry, plus the handler that
  orchestrates the write path.

What belongs here
- `command.rs`: the RegisterTimeEntry command data structure.
- `decision.rs`: Decision type (Accepted/Rejected) and DecideError.
- `decide.rs`: pure function that validates the command and produces events or a rejection.
- `handler.rs`: loads past events, folds to state, calls decide, appends events, enqueues intents.

Boundaries
- `decide.rs` is pure logic only. No input or output.
- `handler.rs` coordinates I/O using ports from adapters.
