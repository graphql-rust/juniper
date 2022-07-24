use std::{net::SocketAddr, pin::Pin, sync::Arc, time::Duration};

use axum::{
    extract::WebSocketUpgrade,
    response::Response,
    routing::{get, post},
    Extension, Router,
};
use futures::{Stream, StreamExt};

use juniper::{graphql_object, graphql_subscription, EmptyMutation, FieldError, RootNode};
use juniper_axum::{
    extract::JuniperRequest, playground, response::JuniperResponse,
    subscriptions::handle_graphql_socket,
};

#[derive(Clone)]
pub struct Context;
impl juniper::Context for Context {}

#[derive(Clone, Copy, Debug)]
pub struct Query;

#[graphql_object(context = Context)]
impl Query {
    /// Add two numbers a and b
    fn add(a: i32, b: i32) -> i32 {
        a + b
    }
}

pub struct Subscription;
type NumberStream = Pin<Box<dyn Stream<Item = Result<i32, FieldError>> + Send>>;
type AppSchema = RootNode<'static, Query, EmptyMutation<Context>, Subscription>;

#[graphql_subscription(context = Context)]
impl Subscription {
    /// Count seconds
    async fn count() -> NumberStream {
        let mut value = 0;
        let stream = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
            Duration::from_secs(1),
        ))
        .map(move |_| {
            value += 1;
            Ok(value)
        });
        Box::pin(stream)
    }
}

#[tokio::main]
async fn main() {
    let schema = Arc::new(AppSchema::new(Query, EmptyMutation::new(), Subscription));

    let context = Context;

    let app = Router::new()
        .route("/", get(|| playground("/graphql", Some("/subscriptions"))))
        .route("/graphql", post(graphql))
        .route("/subscriptions", get(juniper_subscriptions))
        .layer(Extension(schema))
        .layer(Extension(context));

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

pub async fn juniper_subscriptions(
    Extension(schema): Extension<Arc<AppSchema>>,
    Extension(context): Extension<Context>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.protocols(["graphql-ws"])
        .max_frame_size(1024)
        .max_message_size(1024)
        .max_send_queue(100)
        .on_upgrade(|socket| handle_graphql_socket(socket, schema, context))
}

async fn graphql(
    JuniperRequest(request): JuniperRequest,
    Extension(schema): Extension<Arc<AppSchema>>,
    Extension(context): Extension<Context>,
) -> JuniperResponse {
    JuniperResponse(request.execute(&schema, &context).await)
}
