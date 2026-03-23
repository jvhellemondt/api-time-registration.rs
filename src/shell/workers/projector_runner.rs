// Spawns each projector as an independent async task.
//
// The shell calls `spawn` once per projector at startup, passing the projector
// its broadcast receiver. Each projector runs its own loop independently.

use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::use_cases::list_time_entries::projection::ListTimeEntriesState;
use crate::modules::time_entries::use_cases::list_time_entries::projector::ListTimeEntriesProjector;
use crate::shared::infrastructure::event_store::StoredEvent;
use crate::shared::infrastructure::projection_store::ProjectionStore;
use tokio::sync::broadcast;

pub fn spawn<TStore>(
    projector: ListTimeEntriesProjector<TStore>,
    receiver: broadcast::Receiver<StoredEvent<TimeEntryEvent>>,
) where
    TStore: ProjectionStore<ListTimeEntriesState> + Send + Sync + 'static,
{
    tokio::spawn(projector.run(receiver));
}

#[cfg(test)]
mod project_runner {
    use super::*;
    use crate::modules::time_entries::core::events::TimeEntryEvent;
    use crate::modules::time_entries::use_cases::list_time_entries::projection::{
        ListTimeEntriesState, SCHEMA_VERSION,
    };
    use crate::modules::time_entries::use_cases::list_time_entries::projector::{
        ListTimeEntriesProjector, ProjectionTechnicalEvent,
    };
    use crate::modules::time_entries::use_cases::set_started_at::handler::SetStartedAtHandler;
    use crate::shared::infrastructure::event_store::StoredEvent;
    use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
    use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
    use crate::shared::infrastructure::projection_store::in_memory::InMemoryProjectionStore;
    use crate::tests::fixtures::commands::set_started_at::SetStartedAtBuilder;
    use rstest::rstest;

    #[rstest]
    #[tokio::test]
    async fn it_should_spawn_projector_and_apply_events() {
        let (event_tx, _) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(1024);
        let event_store = InMemoryEventStore::<TimeEntryEvent>::new_with_sender(event_tx.clone());
        let outbox = InMemoryDomainOutbox::new();

        let projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
        projection_store
            .save_schema_version(SCHEMA_VERSION)
            .await
            .unwrap();

        let (tech_tx, _) = broadcast::channel::<ProjectionTechnicalEvent>(256);
        let projector = ListTimeEntriesProjector::new(
            "list_time_entries",
            projection_store.clone(),
            event_store.clone(),
            tech_tx,
        );
        let receiver = event_tx.subscribe();
        spawn(projector, receiver);

        let handler = SetStartedAtHandler::new("t", event_store, outbox);
        handler
            .handle(
                "TimeEntry-te-1",
                SetStartedAtBuilder::new()
                    .time_entry_id("te-1".to_string())
                    .build(),
            )
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let state = projection_store.state().await.unwrap().unwrap();
        assert_eq!(state.rows.len(), 1);
    }
}
