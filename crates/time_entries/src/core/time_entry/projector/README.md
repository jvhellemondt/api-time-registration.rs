# core/time_entry/projector folder

Purpose
- Contains pure mapping from domain events to read model mutations.

What belongs here
- model.rs: read model data shape for a single time entry row.
- apply.rs: functions that translate events into upsert or patch mutations.

Boundaries
- No database writes. Only shape data for the application projector runner to persist.

