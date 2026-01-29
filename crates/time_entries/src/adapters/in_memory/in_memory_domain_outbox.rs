// In memory implementation of the DomainOutbox port.
//
// Purpose
// - Support tests and development for verifying that command handlers enqueue domain events.
//
// Responsibilities
// - Collect enqueued domain events in a list for inspection.

use crate::core::ports::{DomainOutbox, OutboxError, OutboxRow};
use std::collections::HashSet;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct InMemoryDomainOutbox {
    pub rows: Mutex<Vec<OutboxRow>>,
    seen: Mutex<HashSet<(String, i64)>>,
}

impl InMemoryDomainOutbox {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl DomainOutbox for InMemoryDomainOutbox {
    async fn enqueue(&self, row: OutboxRow) -> Result<(), OutboxError> {
        let key = (row.stream_id.clone(), row.stream_version);
        {
            let mut s = self.seen.lock().await;
            if !s.insert(key) {
                return Err(OutboxError::Duplicate {
                    stream_id: row.stream_id,
                    stream_version: row.stream_version,
                });
            }
        }
        let mut guard = self.rows.lock().await;
        guard.push(row);
        Ok(())
    }
}

#[cfg(test)]
mod time_entry_in_memory_domain_outbox_tests {
    use super::*;
    use rstest::rstest;
    use crate::test_support::fixtures::events::domain_event::DomainEvent;

    #[rstest]
    #[tokio::test]
    async fn it_should_enqueue_the_event() {
        let outbox = InMemoryDomainOutbox::new();
        let event = DomainEvent { name: "Teddy Test" };
        let row = OutboxRow {
            topic: "test_topic".to_string(),
            event_type: "test_event_type".to_string(),
            event_version: 0,
            stream_id: "123".to_string(),
            stream_version: 0,
            occurred_at: 0,
            payload: serde_json::to_value(&event).unwrap(),
        };
        let enqueue_result = outbox.enqueue(row).await;
        assert!(enqueue_result.is_ok());
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_fail_to_enqueue_if_duplicate_event() {
        let outbox = InMemoryDomainOutbox::new();
        let event = DomainEvent { name: "Teddy Test" };
        let row = OutboxRow {
            topic: "test_topic".to_string(),
            event_type: "test_event_type".to_string(),
            event_version: 0,
            stream_id: "123".to_string(),
            stream_version: 0,
            occurred_at: 0,
            payload: serde_json::to_value(&event).unwrap(),
        };
        outbox.enqueue(row.clone()).await.unwrap();
        let enqueue_result = outbox.enqueue(row).await;
        assert!(enqueue_result.is_err());
        assert!(matches!(
            enqueue_result,
            Err(OutboxError::Duplicate {
                stream_id: _,
                stream_version: 0,
            })
        ));
    }
}
