/// Domain intents produced by the decider as part of an Accepted decision.
/// The outbound intent_outbox adapter translates these into OutboxRows.
#[derive(Clone)]
pub enum TimeEntryIntent {
    NotifyUser {
        time_entry_id: String,
        occurred_at: i64,
    },
}
