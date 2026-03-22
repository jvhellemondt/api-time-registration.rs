use crate::modules::tags::core::events::TagEvent;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DecideError {
    #[error("tag not found")]
    TagNotFound,

    #[error("tag is deleted")]
    TagDeleted,
}

pub enum Decision {
    Accepted { events: Vec<TagEvent> },
    Rejected { reason: DecideError },
}
