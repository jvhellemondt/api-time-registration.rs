# modules/time_entries/core/events folder

Purpose
- Holds versioned event payload modules.

What belongs here
- `v1/`: first version of concrete event payload types.

Versioning
- Prefer adding fields when evolving events.
- For breaking changes, add a new version under a new folder and a new variant in the root
  enumeration in `events.rs`.
