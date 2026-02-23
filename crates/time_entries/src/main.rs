use async_graphql::{EmptySubscription, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{Extension, Router, routing::get};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, fmt};

use time_entries::modules::time_entries::adapters::inbound::graphql::{
    AppSchema, AppState, MutationRoot, QueryRoot,
};
use time_entries::modules::time_entries::adapters::outbound::projections_in_memory::InMemoryProjections;
use time_entries::modules::time_entries::core::events::TimeEntryEvent;
use time_entries::modules::time_entries::use_cases::list_time_entries_by_user::handler::Projector;
use time_entries::modules::time_entries::use_cases::register_time_entry::handler::RegisterTimeEntryHandler;
use time_entries::shared::infrastructure::event_store::in_memory::InMemoryEventStore;
use time_entries::shared::infrastructure::intent_outbox::in_memory::InMemoryDomainOutbox;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    // In-memory deps for now
    let event_store = Arc::new(InMemoryEventStore::<TimeEntryEvent>::new());
    let outbox = Arc::new(InMemoryDomainOutbox::new());
    let projections = Arc::new(InMemoryProjections::new());

    let projector = Arc::new(Projector {
        name: "time_entry_summary".to_string(),
        repository: projections.clone(),
        watermark_repository: projections.clone(),
    });

    let register_handler = Arc::new(RegisterTimeEntryHandler::new(
        "time-entries.v1",
        event_store.clone(),
        outbox,
    ));

    let state = AppState {
        queries: projections,
        register_handler,
        event_store,
        projector,
    };

    let schema: AppSchema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(state)
        .finish();

    let app = Router::new()
        .route("/gql", get(graphiql).post(graphql))
        .layer(Extension(schema))
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    tracing::info!("GraphQL endpoint: http://{}/gql", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await?;
    Ok(())
}

async fn graphql(Extension(schema): Extension<AppSchema>, req: GraphQLRequest) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn graphiql() -> axum::response::Html<String> {
    use async_graphql::http::GraphiQLSource;
    axum::response::Html(GraphiQLSource::build().endpoint("/gql").finish())
}
