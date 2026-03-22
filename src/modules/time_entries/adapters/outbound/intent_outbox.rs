use crate::modules::time_entries::core::intents::TimeEntryIntent;
use crate::shared::infrastructure::intent_outbox::{DomainOutbox, OutboxError, OutboxRow};

/// Translate a list of domain intents into outbox rows and enqueue them.
/// `starting_version` is the event store stream version before the append.
/// `events_len` is the total number of events appended in this decision.
/// The (events_len - intents.len()) offset ensures each intent maps to
/// the correct event version when multiple events precede the intents.
pub async fn dispatch_intents(
    outbox: &impl DomainOutbox,
    stream_id: &str,
    starting_version: i64,
    events_len: usize,
    topic: &str,
    intents: Vec<TimeEntryIntent>,
) -> Result<(), OutboxError> {
    let intent_offset = events_len - intents.len();
    for (i, intent) in intents.into_iter().enumerate() {
        let stream_version = starting_version + (intent_offset + i) as i64 + 1;
        match intent {
            TimeEntryIntent::PublishTimeEntryRegistered {
                time_entry_id,
                occurred_at,
            } => {
                outbox
                    .enqueue(OutboxRow {
                        topic: topic.to_string(),
                        event_type: "TimeEntryRegistered".to_string(),
                        event_version: 1,
                        stream_id: stream_id.to_string(),
                        stream_version,
                        occurred_at,
                        payload: serde_json::json!({
                            "time_entry_id": time_entry_id,
                            "occurred_at": occurred_at
                        }),
                    })
                    .await?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod dispatch_intents_tests {
    use super::*;
    use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
    use rstest::rstest;

    #[rstest]
    #[tokio::test]
    async fn it_should_enqueue_single_intent_successfully() {
        let outbox = InMemoryDomainOutbox::new();
        let intents = vec![TimeEntryIntent::PublishTimeEntryRegistered {
            time_entry_id: "te-0001".to_string(),
            occurred_at: 1_000,
        }];
        dispatch_intents(&outbox, "stream-0001", 0, 1, "time-entries", intents)
            .await
            .unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_compute_correct_stream_version_when_multiple_events_precede_intent() {
        // 3 events, 1 intent → intent is at starting_version + 3 = 3
        // Pre-seed outbox at version 3 to prove the duplicate occurs at that exact version
        let outbox = InMemoryDomainOutbox::new();
        let pre_seed_row = OutboxRow {
            topic: "time-entries".to_string(),
            event_type: "TimeEntryRegistered".to_string(),
            event_version: 1,
            stream_id: "stream-0001".to_string(),
            stream_version: 3,
            occurred_at: 0,
            payload: serde_json::json!({}),
        };
        outbox.enqueue(pre_seed_row).await.unwrap();

        let intents = vec![TimeEntryIntent::PublishTimeEntryRegistered {
            time_entry_id: "te-0001".to_string(),
            occurred_at: 2_000,
        }];
        let result = dispatch_intents(&outbox, "stream-0001", 0, 3, "time-entries", intents).await;
        assert!(
            matches!(
                result,
                Err(OutboxError::Duplicate {
                    stream_version: 3,
                    ..
                })
            ),
            "expected duplicate at version 3"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_return_ok_when_no_intents() {
        let outbox = InMemoryDomainOutbox::new();
        dispatch_intents(&outbox, "stream-0001", 0, 0, "time-entries", vec![])
            .await
            .unwrap();
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_propagate_outbox_duplicate_error() {
        let outbox = InMemoryDomainOutbox::new();
        let intents = vec![TimeEntryIntent::PublishTimeEntryRegistered {
            time_entry_id: "te-0001".to_string(),
            occurred_at: 1_000,
        }];
        dispatch_intents(
            &outbox,
            "stream-0001",
            0,
            1,
            "time-entries",
            intents.clone(),
        )
        .await
        .unwrap();
        let result = dispatch_intents(&outbox, "stream-0001", 0, 1, "time-entries", intents).await;
        assert!(matches!(result, Err(OutboxError::Duplicate { .. })));
    }
}
