use async_graphql::{Context, EmptySubscription, Object, Result as GqlResult, Schema, ID};
use std::sync::Arc;
use chrono::Utc;
use uuid::Uuid;
use time_entries::adapters::in_memory::in_memory_domain_outbox::InMemoryDomainOutbox;
use time_entries::adapters::in_memory::in_memory_event_store::InMemoryEventStore;
use time_entries::adapters::in_memory::in_memory_projections::InMemoryProjections;
use time_entries::application::command_handlers::register_handler::TimeEntryRegisteredCommandHandler;
use time_entries::application::projector::runner::Projector;
use time_entries::application::query_handlers::time_entries_queries::{
    TimeEntryQueries, TimeEntryView,
};
use time_entries::core::ports::EventStore;
use time_entries::core::time_entry::decider::register::command::RegisterTimeEntry;
use time_entries::core::time_entry::event::TimeEntryEvent;

#[derive(async_graphql::SimpleObject, Clone)]
pub struct GqlTimeEntry {
    pub time_entry_id: String,
    pub user_id: String,
    pub start_time: i64,
    pub end_time: i64,
    pub tags: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub created_by: String,
    pub updated_at: i64,
    pub updated_by: String,
    pub deleted_at: Option<i64>,
}

impl From<TimeEntryView> for GqlTimeEntry {
    fn from(v: TimeEntryView) -> Self {
        Self {
            time_entry_id: v.time_entry_id,
            user_id: v.user_id,
            start_time: v.start_time,
            end_time: v.end_time,
            tags: v.tags,
            description: v.description,
            created_at: v.created_at,
            created_by: v.created_by,
            updated_at: v.updated_at,
            updated_by: v.updated_by,
            deleted_at: v.deleted_at,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub queries: Arc<dyn TimeEntryQueries + Send + Sync>,
    pub register_handler: Arc<TimeEntryRegisteredCommandHandler<InMemoryEventStore<TimeEntryEvent>, InMemoryDomainOutbox>>,
    pub event_store: Arc<InMemoryEventStore<TimeEntryEvent>>,
    pub projector: Arc<Projector<InMemoryProjections, InMemoryProjections>>,
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn list_time_entries_by_user_id(
        &self,
        context: &Context<'_>,
        user_id: String,
        offset: Option<i64>,
        limit: Option<i64>,
        sort_desc: Option<bool>,
    ) -> GqlTimeEntryResult<Vec<GqlTimeEntry>> {
        let state = context.data_unchecked::<AppState>();
        let list = state
            .queries
            .list_by_user_id(
                &user_id,
                offset.unwrap_or(0).max(0) as u64,
                limit.unwrap_or(20).max(0) as u64,
                sort_desc.unwrap_or(true),
            )
            .await?;
        Ok(list.into_iter().map(Into::into).collect())
    }
}

type GqlTimeEntryResult<T> = GqlResult<T>;

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn register_time_entry(
        &self,
        context: &Context<'_>,
        user_id: String,
        start_time: i64,
        end_time: i64,
        tags: Vec<String>,
        description: String,
    ) -> GqlTimeEntryResult<ID> {
        let time_entry_id = Uuid::now_v7();
        let state = context.data_unchecked::<AppState>();

        let command = RegisterTimeEntry {
            time_entry_id: time_entry_id.to_string(),
            user_id,
            start_time,
            end_time,
            tags,
            description,
            created_at: Utc::now().timestamp_millis(),
            created_by: "user-from-auth".into(),
        };

        let stream_id = format!("TimeEntry-{time_entry_id}");

        state.register_handler.handle(&stream_id, command).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Inline projection so queries see the new row immediately
        let loaded = state.event_store.load(&stream_id).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        if let Some(last) = loaded.events.last() {
            state.projector.apply_one(&stream_id, loaded.version, last).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }

        Ok(ID(time_entry_id.to_string()))
    }
}

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;
