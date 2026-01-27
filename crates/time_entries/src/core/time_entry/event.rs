// Root event enumeration for time entry and re-exports of versioned payloads.
//
// Purpose
// - Provide a single type to pattern match in evolve and projectors.
//
// Versioning and evolution
// - Prefer additive changes. If a breaking change is needed, add a new version and a new variant.
// - Do not change the meaning of historical events.
//
// Structure
// - This file defines the root event enumeration (later).
// - The sibling folder 'event/' contains versioned payload modules (for example: v1/).

pub mod v1 {
    pub mod time_entry_registered;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum TimeEntryEvent {
    TimeEntryRegisteredV1(v1::time_entry_registered::TimeEntryRegisteredV1),
}
