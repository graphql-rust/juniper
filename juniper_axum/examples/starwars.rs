use std::net::SocketAddr;

use axum::{
    extract::WebSocketUpgrade,
    response::Response,
    routing::{get, post},
    Extension, Router,
};
use juniper::{
    tests::fixtures::starwars::schema::{Database, Query, Subscription},
    EmptyMutation, RootNode,
};
use juniper_axum::{
    extract::JuniperRequest, playground, response::JuniperResponse,
    subscriptions::handle_graphql_socket,
};

type Schema = RootNode<'static, Query, EmptyMutation<Database>, Subscription>;

#[tokio::main]
async fn main() {
    let schema = Schema::new(Query, EmptyMutation::new(), Subscription);

    let context = Database::new();

    let app = Router::new()
        .route("/", get(playground("/graphql", "/subscriptions")))
        .route("/graphql", post(graphql))
        .route("/subscriptions", get(juniper_subscriptions))
        .layer(Extension(schema))
        .layer(Extension(context));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

pub async fn juniper_subscriptions(
    Extension(schema): Extension<Schema>,
    Extension(context): Extension<Database>,
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
    Extension(schema): Extension<Schema>,
    Extension(context): Extension<Database>,
) -> JuniperResponse {
    JuniperResponse(request.execute(&schema, &context).await)
}
