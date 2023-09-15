//! This example demonstrates asynchronous subscriptions with warp and tokio 0.2

use std::{env, pin::Pin, sync::Arc, time::Duration};

use futures::Stream;
use juniper::{
    graphql_object, graphql_subscription, graphql_value, EmptyMutation, FieldError, GraphQLEnum,
    RootNode,
};
use juniper_graphql_transport_ws::ConnectionConfig;
use juniper_graphql_ws::ConnectionConfig as LegacyConnectionConfig;
use juniper_warp::{
    graphiql_filter, playground_filter,
    subscriptions::{serve_graphql_transport_ws, serve_graphql_ws},
};
use warp::{http::Response, Filter};

#[derive(Clone)]
struct Context;

impl juniper::Context for Context {}

#[derive(Clone, Copy, GraphQLEnum)]
enum UserKind {
    Admin,
    User,
    Guest,
}

struct User {
    id: i32,
    kind: UserKind,
    name: String,
}

// Field resolvers implementation
#[graphql_object(context = Context)]
impl User {
    fn id(&self) -> i32 {
        self.id
    }

    fn kind(&self) -> UserKind {
        self.kind
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn friends(&self) -> Vec<User> {
        if self.id == 1 {
            vec![
                User {
                    id: 11,
                    kind: UserKind::User,
                    name: "user11".into(),
                },
                User {
                    id: 12,
                    kind: UserKind::Admin,
                    name: "user12".into(),
                },
                User {
                    id: 13,
                    kind: UserKind::Guest,
                    name: "user13".into(),
                },
            ]
        } else if self.id == 2 {
            vec![User {
                id: 21,
                kind: UserKind::User,
                name: "user21".into(),
            }]
        } else if self.id == 3 {
            vec![
                User {
                    id: 31,
                    kind: UserKind::User,
                    name: "user31".into(),
                },
                User {
                    id: 32,
                    kind: UserKind::Guest,
                    name: "user32".into(),
                },
            ]
        } else {
            vec![]
        }
    }
}

struct Query;

#[graphql_object(context = Context)]
impl Query {
    async fn users(id: i32) -> Vec<User> {
        vec![User {
            id,
            kind: UserKind::Admin,
            name: "User Name".into(),
        }]
    }
}

type UsersStream = Pin<Box<dyn Stream<Item = Result<User, FieldError>> + Send>>;

struct Subscription;

#[graphql_subscription(context = Context)]
impl Subscription {
    async fn users() -> UsersStream {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let stream = async_stream::stream! {
            let mut counter = 0;
            loop {
                counter += 1;
                interval.tick().await;
                if counter == 5 {
                    yield Err(FieldError::new(
                        "some field error from handler",
                        graphql_value!("some additional string"),
                    ))
                } else {
                    yield Ok(User {
                        id: counter,
                        kind: UserKind::Admin,
                        name: "stream user".into(),
                    })
                }
            }
        };

        Box::pin(stream)
    }
}

type Schema = RootNode<'static, Query, EmptyMutation<Context>, Subscription>;

fn schema() -> Schema {
    Schema::new(Query, EmptyMutation::new(), Subscription)
}

#[tokio::main]
async fn main() {
    env::set_var("RUST_LOG", "subscription");
    env_logger::init();

    let log = warp::log("subscription");

    let homepage = warp::path::end().map(|| {
        Response::builder()
            .header("content-type", "text/html")
            .body(
                "<html><h1>juniper_warp/subscription example</h1>\
                       <div>visit <a href=\"/graphiql\">GraphiQL</a></div>\
                       <div>visit <a href=\"/playground\">GraphQL Playground</a></div>\
                 </html>",
            )
    });

    let qm_schema = schema();
    let qm_state = warp::any().map(|| Context);
    let qm_graphql_filter = juniper_warp::make_graphql_filter(qm_schema, qm_state.boxed());

    let ws_schema = Arc::new(schema());
    let transport_ws_schema = ws_schema.clone();

    log::info!("Listening on 127.0.0.1:8080");

    let routes = warp::path("subscriptions")
        .and(warp::ws())
        .and(warp::filters::header::value("sec-websocket-protocol"))
        .map(move |ws: warp::ws::Ws, subproto| {
            let transport_ws_schema = transport_ws_schema.clone();
            ws.on_upgrade(move |websocket| async move {
                if subproto == "graphql-ws" {
                    serve_graphql_ws(
                        websocket,
                        transport_ws_schema,
                        LegacyConnectionConfig::new(Context),
                    )
                    .await
                } else {
                    serve_graphql_transport_ws(
                        websocket,
                        transport_ws_schema,
                        ConnectionConfig::new(Context),
                    )
                    .await
                }
                .unwrap_or_else(|e| {
                    log::error!("WebSocket error: {e}");
                })
            })
        })
        .or(warp::post()
            .and(warp::path("graphql"))
            .and(qm_graphql_filter))
        .or(warp::get()
            .and(warp::path("playground"))
            .and(playground_filter("/graphql", Some("/subscriptions"))))
        .or(warp::get()
            .and(warp::path("graphiql"))
            .and(graphiql_filter("/graphql", Some("/subscriptions"))))
        .or(homepage)
        .with(log);

    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}
