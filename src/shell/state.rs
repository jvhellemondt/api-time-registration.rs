use crate::modules::time_entries::adapters::outbound::projections_in_memory::InMemoryProjections;
use crate::modules::time_entries::core::events::TimeEntryEvent;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::handler::Projector;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::queries_port::TimeEntryQueries;
use crate::modules::time_entries::use_cases::register_time_entry::handler::RegisterTimeEntryHandler;
use crate::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
use crate::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub queries: Arc<dyn TimeEntryQueries + Send + Sync>,
    pub register_handler:
        Arc<RegisterTimeEntryHandler<InMemoryEventStore<TimeEntryEvent>, InMemoryDomainOutbox>>,
    pub event_store: Arc<InMemoryEventStore<TimeEntryEvent>>,
    pub projector: Arc<Projector<InMemoryProjections, InMemoryProjections>>,
}
