use crate::modules::time_entries::core::events::v1::time_entry_registered::TimeEntryRegisteredV1;

/// Domain intents produced by the decider as part of an Accepted decision.
/// The outbound intent_outbox adapter translates these into OutboxRows.
pub enum TimeEntryIntent {
    PublishTimeEntryRegistered { payload: TimeEntryRegisteredV1 },
}
