use crate::modules::time_entries::adapters::outbound::projections_in_memory::InMemoryProjections;
use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::handler::Projector;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::queries_port::TimeEntryQueries;
use crate::modules::time_entries::use_cases::register_time_entry::handler::RegisterTimeEntryHandler;
use crate::shared::infrastructure::event_store::EventStore;
use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
use crate::tests::fixtures::commands::register_time_entry::RegisterTimeEntryBuilder;
use std::sync::Arc;

#[tokio::test]
async fn lists_time_entries_by_user() {
    let store = Arc::new(InMemoryEventStore::<TimeEntryEvent>::new());
    let outbox = Arc::new(InMemoryDomainOutbox::new());
    let projections = Arc::new(InMemoryProjections::new());
    let projector = Projector {
        name: "time_entry_summary".into(),
        repository: projections.clone(),
        watermark_repository: projections.clone(),
    };
    let handler = RegisterTimeEntryHandler::new("time-entries", store.clone(), outbox);

    let commands: Vec<_> = [1000, 2000, 1500]
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

        let loaded = store
            .load(&format!("TimeEntry-te-{iteration}"))
            .await
            .unwrap();
        projector
            .apply_one(
                &format!("TimeEntry-te-{iteration}"),
                1,
                loaded.events.first().unwrap(),
            )
            .await
            .unwrap();
    }

    let list = projections
        .list_by_user_id("user-fixed-0001", 0, 10, true)
        .await
        .unwrap();

    assert_eq!(list.len(), 3);
    assert!(list[0].start_time >= list[1].start_time);
    assert_eq!(list[0].time_entry_id, commands[1].time_entry_id);
    assert_eq!(list[0].start_time, commands[1].start_time);
}
