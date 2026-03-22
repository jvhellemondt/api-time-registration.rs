use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::core::intents::TimeEntryIntent;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecideError {
    #[error("interval is invalid: started_at must be less than ended_at")]
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
