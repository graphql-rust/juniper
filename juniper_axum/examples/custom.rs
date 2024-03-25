//! This example demonstrates custom [`Handler`]s with [`axum`], using the [`starwars::schema`].
//!
//! [`Handler`]: axum::handler::Handler
//! [`starwars::schema`]: juniper::tests::fixtures::starwars::schema

use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::WebSocketUpgrade,
    response::{Html, Response},
    routing::{get, on, MethodFilter},
    Extension, Router,
};
use juniper::{
    tests::fixtures::starwars::schema::{Database, Query, Subscription},
    EmptyMutation, RootNode,
};
use juniper_axum::{
    extract::JuniperRequest, graphiql, playground, response::JuniperResponse, subscriptions,
};
use juniper_graphql_ws::ConnectionConfig;
use tokio::net::TcpListener;

type Schema = RootNode<Query, EmptyMutation<Database>, Subscription>;

async fn homepage() -> Html<&'static str> {
    "<html><h1>juniper_axum/custom example</h1>\
           <div>visit <a href=\"/graphiql\">GraphiQL</a></div>\
           <div>visit <a href=\"/playground\">GraphQL Playground</a></div>\
    </html>"
        .into()
}

pub async fn custom_subscriptions(
    Extension(schema): Extension<Arc<Schema>>,
    Extension(database): Extension<Database>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.protocols(["graphql-transport-ws", "graphql-ws"])
        .max_frame_size(1024)
        .max_message_size(1024)
        .max_write_buffer_size(100)
        .on_upgrade(move |socket| {
            subscriptions::serve_ws(
                socket,
                schema,
                ConnectionConfig::new(database).with_max_in_flight_operations(10),
            )
        })
}

async fn custom_graphql(
    Extension(schema): Extension<Arc<Schema>>,
    Extension(database): Extension<Database>,
    JuniperRequest(request): JuniperRequest,
) -> JuniperResponse {
    JuniperResponse(request.execute(&*schema, &database).await)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let schema = Schema::new(Query, EmptyMutation::new(), Subscription);
    let database = Database::new();

    let app = Router::new()
        .route(
            "/graphql",
            on(MethodFilter::GET.or(MethodFilter::POST), custom_graphql),
        )
        .route("/subscriptions", get(custom_subscriptions))
        .route("/graphiql", get(graphiql("/graphql", "/subscriptions")))
        .route("/playground", get(playground("/graphql", "/subscriptions")))
        .route("/", get(homepage))
        .layer(Extension(Arc::new(schema)))
        .layer(Extension(database));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| panic!("failed to listen on {addr}: {e}"));
    tracing::info!("listening on {addr}");
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| panic!("failed to run `axum::serve`: {e}"));
}
