use crate::modules::time_entries::core::intents::TimeEntryIntent;
use crate::shared::infrastructure::intent_outbox::{DomainOutbox, OutboxError, OutboxRow};

/// Translate a list of domain intents into outbox rows and enqueue them.
/// `starting_version` is the event store stream version before the append.
/// Each intent corresponds to one new version: starting_version + index + 1.
pub async fn dispatch_intents(
    outbox: &impl DomainOutbox,
    stream_id: &str,
    starting_version: i64,
    topic: &str,
    intents: Vec<TimeEntryIntent>,
) -> Result<(), OutboxError> {
    for (i, intent) in intents.into_iter().enumerate() {
        let stream_version = starting_version + i as i64 + 1;
        match intent {
            TimeEntryIntent::PublishTimeEntryRegistered { payload } => {
                outbox
                    .enqueue(OutboxRow {
                        topic: topic.to_string(),
                        event_type: "TimeEntryRegistered".to_string(),
                        event_version: 1,
                        stream_id: stream_id.to_string(),
                        stream_version,
                        occurred_at: payload.created_at,
                        payload: serde_json::to_value(payload).unwrap(),
                    })
                    .await?;
            }
        }
    }
    Ok(())
}
