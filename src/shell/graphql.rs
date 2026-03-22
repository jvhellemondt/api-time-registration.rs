use async_graphql::{EmptySubscription, MergedObject, Schema};

use crate::modules::tags::use_cases::create_tag::inbound::graphql::CreateTagMutation;
use crate::modules::tags::use_cases::delete_tag::inbound::graphql::DeleteTagMutation;
use crate::modules::tags::use_cases::list_tags::inbound::graphql::ListTagsQuery;
use crate::modules::tags::use_cases::set_tag_color::inbound::graphql::SetTagColorMutation;
use crate::modules::tags::use_cases::set_tag_description::inbound::graphql::SetTagDescriptionMutation;
use crate::modules::tags::use_cases::set_tag_name::inbound::graphql::SetTagNameMutation;
use crate::modules::time_entries::use_cases::list_time_entries_by_user::inbound::graphql::TimeEntryQueries;
use crate::modules::time_entries::use_cases::register_time_entry::inbound::graphql::TimeEntryMutations;
pub use crate::shell::state::AppState;

#[derive(MergedObject, Default)]
pub struct MutationRoot(
    TimeEntryMutations,
    CreateTagMutation,
    DeleteTagMutation,
    SetTagNameMutation,
    SetTagColorMutation,
    SetTagDescriptionMutation,
);

#[derive(MergedObject, Default)]
pub struct QueryRoot(TimeEntryQueries, ListTagsQuery);

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;
