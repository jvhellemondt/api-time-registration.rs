# core/time_entry/event folder

Purpose
- Holds the root event enumeration (in event.rs) and versioned event payload modules (in event/ subfolders).

What belongs here
- event.rs: root enumeration and re-exports for convenience.
- event/v1: first version of concrete event payload types.

Versioning
- Prefer adding fields when evolving events.
- For breaking changes, add a new version under a new folder and a new variant in the root enumeration.

