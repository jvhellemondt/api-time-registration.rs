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
use crate::shell::state::AppState;

pub fn make_test_app_state() -> AppState {
    let event_store = InMemoryEventStore::<TimeEntryEvent>::new();
    let outbox = InMemoryDomainOutbox::new();
    let time_entry_projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
    let register_time_entry_handler =
        RegisterTimeEntryHandler::new("time-entries", event_store.clone(), outbox.clone());
    let list_time_entries_handler = ListTimeEntriesQueryHandler::new(time_entry_projection_store);

    let tag_event_store = InMemoryEventStore::<TagEvent>::new();
    let create_tag_handler = CreateTagHandler::new(tag_event_store.clone());
    let delete_tag_handler = DeleteTagHandler::new(tag_event_store.clone());
    let set_tag_name_handler = SetTagNameHandler::new(tag_event_store.clone());
    let set_tag_color_handler = SetTagColorHandler::new(tag_event_store.clone());
    let set_tag_description_handler = SetTagDescriptionHandler::new(tag_event_store.clone());
    let tag_projection_store = InMemoryProjectionStore::<ListTagsState>::new();
    let list_tags_handler = ListTagsQueryHandler::new(tag_projection_store.clone());

    AppState {
        register_time_entry_handler,
        event_store,
        outbox,
        list_time_entries_handler,
        tag_event_store,
        create_tag_handler,
        delete_tag_handler,
        set_tag_name_handler,
        set_tag_color_handler,
        set_tag_description_handler,
        list_tags_handler,
        tag_projection_store,
    }
}
