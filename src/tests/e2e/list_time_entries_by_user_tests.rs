use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::ListTimeEntriesState;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::projector::{
    ListTimeEntriesProjector, ProjectionTechnicalEvent,
};
use crate::modules::time_entries::use_cases::list_time_entries_by_user::queries::ListTimeEntriesQueryHandler;
use crate::modules::time_entries::use_cases::register_time_entry::handler::RegisterTimeEntryHandler;
use crate::shared::infrastructure::event_store::StoredEvent;
use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
use crate::shared::infrastructure::projection_store::ProjectionStore;
use crate::shared::infrastructure::projection_store::in_memory::InMemoryProjectionStore;
use crate::tests::fixtures::commands::register_time_entry::RegisterTimeEntryBuilder;
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
        "list_time_entries_by_user",
        projection_store.clone(),
        store.clone(),
        tech_tx,
    );
    let receiver = event_tx.subscribe();
    tokio::spawn(projector.run(receiver));

    let handler = RegisterTimeEntryHandler::new("time-entries", store.clone(), outbox);

    let commands: Vec<_> = [1000i64, 2000, 1500]
        .into_iter()
        .map(|start| {
            RegisterTimeEntryBuilder::new()
                .time_entry_id(format!("te-{start}"))
                .start_time(start)
                .end_time(start + 60_000)
                .build()
        })
        .collect();

    for (iteration, command) in commands.iter().cloned().enumerate() {
        handler
            .handle(&format!("TimeEntry-te-{iteration}"), command)
            .await
            .unwrap();
    }

    let expected_checkpoint = commands.len() as u64;
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
    assert!(list[0].start_time >= list[1].start_time);
    assert_eq!(list[0].time_entry_id, commands[1].time_entry_id);
    assert_eq!(list[0].start_time, commands[1].start_time);
}
