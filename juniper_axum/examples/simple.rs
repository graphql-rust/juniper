//! This example demonstrates simple default integration with [`axum`].

use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    response::Html,
    routing::{get, on, MethodFilter},
    Extension, Router,
};
use futures::stream::{BoxStream, StreamExt as _};
use juniper::{graphql_object, graphql_subscription, EmptyMutation, FieldError, RootNode};
use juniper_axum::{graphiql, graphql, playground, ws};
use juniper_graphql_ws::ConnectionConfig;
use tokio::{net::TcpListener, time::interval};
use tokio_stream::wrappers::IntervalStream;

#[derive(Clone, Copy, Debug)]
pub struct Query;

#[graphql_object]
impl Query {
    /// Adds two `a` and `b` numbers.
    fn add(a: i32, b: i32) -> i32 {
        a + b
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Subscription;

type NumberStream = BoxStream<'static, Result<i32, FieldError>>;

#[graphql_subscription]
impl Subscription {
    /// Counts seconds.
    async fn count() -> NumberStream {
        let mut value = 0;
        let stream = IntervalStream::new(interval(Duration::from_secs(1))).map(move |_| {
            value += 1;
            Ok(value)
        });
        Box::pin(stream)
    }
}

type Schema = RootNode<Query, EmptyMutation, Subscription>;

async fn homepage() -> Html<&'static str> {
    "<html><h1>juniper_axum/simple example</h1>\
           <div>visit <a href=\"/graphiql\">GraphiQL</a></div>\
           <div>visit <a href=\"/playground\">GraphQL Playground</a></div>\
    </html>"
        .into()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let schema = Schema::new(Query, EmptyMutation::new(), Subscription);

    let app = Router::new()
        .route(
            "/graphql",
            on(
                MethodFilter::GET.or(MethodFilter::POST),
                graphql::<Arc<Schema>>,
            ),
        )
        .route(
            "/subscriptions",
            get(ws::<Arc<Schema>>(ConnectionConfig::new(()))),
        )
        .route("/graphiql", get(graphiql("/graphql", "/subscriptions")))
        .route("/playground", get(playground("/graphql", "/subscriptions")))
        .route("/", get(homepage))
        .layer(Extension(Arc::new(schema)));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| panic!("failed to listen on {addr}: {e}"));
    tracing::info!("listening on {addr}");
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| panic!("failed to run `axum::serve`: {e}"));
}
