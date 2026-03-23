use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::intents::TimeEntryIntent;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecideError {}

pub enum Decision {
    Accepted {
        events: Vec<TimeEntryEvent>,
        intents: Vec<TimeEntryIntent>,
    },
    Rejected {
        reason: DecideError,
    },
}
