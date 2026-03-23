use async_graphql::{EmptySubscription, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::http::StatusCode;
use axum::{Extension, Router, http::HeaderMap, routing::get};
use std::net::SocketAddr;
use time_entries::shared::infrastructure::request_context::RequestContext;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, fmt};

use time_entries::modules::tags::core::events::TagEvent;
use time_entries::modules::tags::use_cases::create_tag::handler::CreateTagHandler;
use time_entries::modules::tags::use_cases::delete_tag::handler::DeleteTagHandler;
use time_entries::modules::tags::use_cases::list_tags::projection::ListTagsState;
use time_entries::modules::tags::use_cases::list_tags::projector::{
    ListTagsProjector, ProjectionTechnicalEvent as TagProjectionTechnicalEvent,
};
use time_entries::modules::tags::use_cases::list_tags::queries::ListTagsQueryHandler;
use time_entries::modules::tags::use_cases::set_tag_color::handler::SetTagColorHandler;
use time_entries::modules::tags::use_cases::set_tag_description::handler::SetTagDescriptionHandler;
use time_entries::modules::tags::use_cases::set_tag_name::handler::SetTagNameHandler;
use time_entries::modules::time_entries::core::events::TimeEntryEvent;
use time_entries::modules::time_entries::use_cases::list_time_entries_by_user::projection::ListTimeEntriesState;
use time_entries::modules::time_entries::use_cases::list_time_entries_by_user::projector::{
    ListTimeEntriesProjector, ProjectionTechnicalEvent,
};
use time_entries::modules::time_entries::use_cases::list_time_entries_by_user::queries::ListTimeEntriesQueryHandler;
use time_entries::modules::time_entries::use_cases::set_ended_at::handler::SetEndedAtHandler;
use time_entries::modules::time_entries::use_cases::set_started_at::handler::SetStartedAtHandler;
use time_entries::modules::time_entries::use_cases::set_time_entry_tags::handler::SetTimeEntryTagsHandler;
use time_entries::shared::infrastructure::event_store::StoredEvent;
use time_entries::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
use time_entries::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;
use time_entries::shared::infrastructure::projection_store::in_memory::InMemoryProjectionStore;
use time_entries::shell::graphql::{AppSchema, AppState, MutationRoot, QueryRoot};
use time_entries::shell::http as shell_http;
use time_entries::shell::workers::projector_runner;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    // Time entries event store + projector
    let (event_tx, _) = tokio::sync::broadcast::channel::<StoredEvent<TimeEntryEvent>>(1024);
    let event_store = InMemoryEventStore::<TimeEntryEvent>::new_with_sender(event_tx.clone());
    let outbox = InMemoryDomainOutbox::new();

    let projection_store = InMemoryProjectionStore::<ListTimeEntriesState>::new();
    let (tech_tx, _) = tokio::sync::broadcast::channel::<ProjectionTechnicalEvent>(256);
    let projector = ListTimeEntriesProjector::new(
        "list_time_entries_by_user",
        projection_store.clone(),
        event_store.clone(),
        tech_tx,
    );
    let receiver = event_tx.subscribe();
    projector_runner::spawn(projector, receiver);
    let list_time_entries_handler = ListTimeEntriesQueryHandler::new(projection_store);
    let set_started_at_handler =
        SetStartedAtHandler::new("time-entries.v1", event_store.clone(), outbox.clone());
    let set_ended_at_handler =
        SetEndedAtHandler::new("time-entries.v1", event_store.clone(), outbox.clone());
    let set_time_entry_tags_handler =
        SetTimeEntryTagsHandler::new("time-entries.v1", event_store.clone(), outbox.clone());

    // Tags event store + projector
    let (tag_event_tx, _) = tokio::sync::broadcast::channel::<StoredEvent<TagEvent>>(1024);
    let tag_event_store = InMemoryEventStore::<TagEvent>::new_with_sender(tag_event_tx.clone());

    let tag_projection_store = InMemoryProjectionStore::<ListTagsState>::new();
    let (tag_tech_tx, _) = tokio::sync::broadcast::channel::<TagProjectionTechnicalEvent>(256);
    let tag_projector = ListTagsProjector::new(
        "list_tags",
        tag_projection_store.clone(),
        tag_event_store.clone(),
        tag_tech_tx,
    );
    let tag_receiver = tag_event_tx.subscribe();
    tokio::spawn(tag_projector.run(tag_receiver));

    let list_tags_handler = ListTagsQueryHandler::new(tag_projection_store.clone());
    let create_tag_handler = CreateTagHandler::new(tag_event_store.clone());
    let delete_tag_handler = DeleteTagHandler::new(tag_event_store.clone());
    let set_tag_name_handler = SetTagNameHandler::new(tag_event_store.clone());
    let set_tag_color_handler = SetTagColorHandler::new(tag_event_store.clone());
    let set_tag_description_handler = SetTagDescriptionHandler::new(tag_event_store.clone());

    let state = AppState {
        list_time_entries_handler,
        set_started_at_handler,
        set_ended_at_handler,
        set_time_entry_tags_handler,
        event_store,
        outbox,
        tag_event_store,
        create_tag_handler,
        delete_tag_handler,
        set_tag_name_handler,
        set_tag_color_handler,
        set_tag_description_handler,
        list_tags_handler,
        tag_projection_store,
    };

    let http_router = shell_http::router(state.clone());

    let schema: AppSchema = Schema::build(
        QueryRoot::default(),
        MutationRoot::default(),
        EmptySubscription,
    )
    .data(state)
    .finish();

    let app = Router::new()
        .merge(http_router)
        .route("/gql", get(graphiql).post(graphql))
        .layer(Extension(schema))
        .layer(TraceLayer::new_for_http())
        .layer(tower_http::cors::CorsLayer::permissive());

    let addr: SocketAddr = "[::]:8080".parse().unwrap();
    tracing::info!("Server running: http://{}/*", addr);
    tracing::info!("GraphQL endpoint: http://{}/gql", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await?;
    Ok(())
}

async fn graphql(
    Extension(schema): Extension<AppSchema>,
    headers: HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let user_id = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let tenant_id = headers
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    match (user_id, tenant_id) {
        (Some(user_id), Some(tenant_id)) => {
            let ctx = RequestContext { user_id, tenant_id };
            schema.execute(req.into_inner().data(ctx)).await.into()
        }
        _ => GraphQLResponse::from(async_graphql::Response::from_errors(vec![
            async_graphql::ServerError::new(
                StatusCode::UNAUTHORIZED
                    .canonical_reason()
                    .unwrap()
                    .to_string(),
                None,
            ),
        ])),
    }
}

async fn graphiql() -> axum::response::Html<String> {
    use async_graphql::http::GraphiQLSource;
    axum::response::Html(GraphiQLSource::build().endpoint("/gql").finish())
}
