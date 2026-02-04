use crate::adapters::in_memory::in_memory_domain_outbox::InMemoryDomainOutbox;
use crate::adapters::in_memory::in_memory_event_store::InMemoryEventStore;
use crate::adapters::in_memory::in_memory_projections::InMemoryProjections;
use crate::application::command_handlers::register_handler::TimeEntryRegisteredCommandHandler;
use crate::application::projector::runner::Projector;
use crate::application::query_handlers::time_entries_queries::TimeEntryQueries;
use crate::core::ports::EventStore;
use crate::core::time_entry::event::TimeEntryEvent;
use crate::tests::fixtures::commands::register_time_entry::RegisterTimeEntryBuilder;

#[tokio::test]
async fn lists_time_entries_by_user() {
    let store = InMemoryEventStore::<TimeEntryEvent>::new();
    let outbox = InMemoryDomainOutbox::new();
    let projections = InMemoryProjections::new();
    let projector = Projector {
        name: "time_entry_summary".into(),
        repository: &projections,
        watermark_repository: &projections,
    };
    let handler = TimeEntryRegisteredCommandHandler::new("time-entries", &store, &outbox);

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
