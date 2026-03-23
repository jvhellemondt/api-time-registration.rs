use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::use_cases::list_time_entries::projection::ListTimeEntriesState;
use crate::modules::time_entries::use_cases::list_time_entries::projector::{
    ListTimeEntriesProjector, ProjectionTechnicalEvent,
};
use crate::modules::time_entries::use_cases::list_time_entries::queries::ListTimeEntriesQueryHandler;
use crate::modules::time_entries::use_cases::set_ended_at::handler::SetEndedAtHandler;
use crate::modules::time_entries::use_cases::set_started_at::handler::SetStartedAtHandler;
use crate::shared::infrastructure::event_store::StoredEvent;
use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
use crate::shared::infrastructure::projection_store::ProjectionStore;
use crate::shared::infrastructure::projection_store::in_memory::InMemoryProjectionStore;
use crate::tests::fixtures::commands::set_ended_at::SetEndedAtBuilder;
use crate::tests::fixtures::commands::set_started_at::SetStartedAtBuilder;
use tokio::sync::broadcast;

#[tokio::test]
async fn lists_time_entries_by_user() {
    let (event_tx, _) = broadcast::channel::<StoredEvent<TimeEntryEvent>>(64);
    let store = InMemoryEventStore::<TimeEntryEvent>::new_with_sender(event_tx.clone());
    let outbox = InMemoryDomainOutbox::new();

    let projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
    let query_handler = ListTimeEntriesQueryHandler::new(projection_store.clone());

    let (tech_tx, _) = broadcast::channel::<ProjectionTechnicalEvent>(16);
    let projector = ListTimeEntriesProjector::new(
        "list_time_entries",
        projection_store.clone(),
        store.clone(),
        tech_tx,
    );
    let receiver = event_tx.subscribe();
    tokio::spawn(projector.run(receiver));

    let set_started_at = SetStartedAtHandler::new("time-entries", store.clone(), outbox.clone());
    let set_ended_at = SetEndedAtHandler::new("time-entries", store.clone(), outbox);

    // Three entries with different started_at values
    let entries: Vec<(i64, i64)> = vec![(1_000, 61_000), (2_000, 62_000), (1_500, 61_500)];

    for (i, (started_at, ended_at)) in entries.iter().enumerate() {
        let te_id = format!("te-e2e-{i}");
        let stream_id = format!("TimeEntry-{te_id}");
        set_started_at
            .handle(
                &stream_id,
                SetStartedAtBuilder::new()
                    .time_entry_id(te_id.clone())
                    .started_at(*started_at)
                    .build(),
            )
            .await
            .unwrap();
        set_ended_at
            .handle(
                &stream_id,
                SetEndedAtBuilder::new()
                    .time_entry_id(te_id.clone())
                    .ended_at(*ended_at)
                    .build(),
            )
            .await
            .unwrap();
    }

    // Each entry emits: Initiated + StartSet + EndSet + Registered = 4 events
    let expected_checkpoint = (entries.len() * 4) as u64;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if projection_store.checkpoint().await.unwrap() >= expected_checkpoint {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "projector did not catch up in time"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    let list = query_handler
        .list_by_user_id("user-fixed-0001", 0, 10, true)
        .await
        .unwrap();

    assert_eq!(list.len(), 3);
    // Descending by started_at: 2000 > 1500 > 1000
    assert!(list[0].started_at >= list[1].started_at);
    assert_eq!(list[0].started_at, Some(2_000));
    assert_eq!(list[1].started_at, Some(1_500));
    assert_eq!(list[2].started_at, Some(1_000));
}
