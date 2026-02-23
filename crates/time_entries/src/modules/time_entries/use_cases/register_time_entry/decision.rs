use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::intents::TimeEntryIntent;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DecideError {
    #[error("time entry already exists")]
    AlreadyExists,

    #[error("end time must be after start time")]
    InvalidInterval,
}

pub enum Decision {
    Accepted {
        events: Vec<TimeEntryEvent>,
        intents: Vec<TimeEntryIntent>,
    },
    Rejected {
        reason: DecideError,
    },
}
