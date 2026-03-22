use crate::modules::tags::core::events::TagEvent;
use crate::modules::tags::use_cases::create_tag::handler::CreateTagHandler;
use crate::modules::tags::use_cases::delete_tag::handler::DeleteTagHandler;
use crate::modules::tags::use_cases::list_tags::projection::ListTagsState;
use crate::modules::tags::use_cases::list_tags::queries::ListTagsQueryHandler;
use crate::modules::tags::use_cases::set_tag_color::handler::SetTagColorHandler;
use crate::modules::tags::use_cases::set_tag_description::handler::SetTagDescriptionHandler;
use crate::modules::tags::use_cases::set_tag_name::handler::SetTagNameHandler;
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
    pub tag_event_store: InMemoryEventStore<TagEvent>,
    pub create_tag_handler: CreateTagHandler<InMemoryEventStore<TagEvent>>,
    pub delete_tag_handler: DeleteTagHandler<InMemoryEventStore<TagEvent>>,
    pub set_tag_name_handler: SetTagNameHandler<InMemoryEventStore<TagEvent>>,
    pub set_tag_color_handler: SetTagColorHandler<InMemoryEventStore<TagEvent>>,
    pub set_tag_description_handler: SetTagDescriptionHandler<InMemoryEventStore<TagEvent>>,
    pub list_tags_handler: ListTagsQueryHandler<InMemoryProjectionStore<ListTagsState>>,
    pub tag_projection_store: InMemoryProjectionStore<ListTagsState>,
}
