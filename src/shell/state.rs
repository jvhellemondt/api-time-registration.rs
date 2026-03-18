use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::projection::ListTimeEntriesState;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::queries::ListTimeEntriesQueryHandler;
use crate::modules::time_entries::use_cases::register_time_entry::handler::RegisterTimeEntryHandler;
use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
use crate::shared::infrastructure::projection_store::in_memory::InMemoryProjectionStore;

#[derive(Clone)]
pub struct AppState {
    pub register_time_entry_handler:
        RegisterTimeEntryHandler<InMemoryEventStore<TimeEntryEvent>, InMemoryDomainOutbox>,
    pub event_store: InMemoryEventStore<TimeEntryEvent>,
    pub outbox: InMemoryDomainOutbox,
    pub list_time_entries_handler:
        ListTimeEntriesQueryHandler<InMemoryProjectionStore<ListTimeEntriesState>>,
}
