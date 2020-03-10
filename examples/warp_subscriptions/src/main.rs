//! This example demonstrates asynchronous subscriptions with warp and tokio 0.2

use std::{pin::Pin, sync::Arc, time::Duration};

use futures::{Future, FutureExt as _, Stream};
use warp::{http::Response, Filter};

use juniper::{DefaultScalarValue, EmptyMutation, FieldError, RootNode};
use juniper_warp::playground_filter;
use juniper_subscriptions::Coordinator;
use warp_subscriptions::*;

mod warp_subscriptions;

#[derive(Clone)]
struct Context {}

impl juniper::Context for Context {}

/// Kind of a User
#[derive(juniper::GraphQLEnum, Clone, Copy)]
enum UserKind {
    Admin,
    User,
    Guest,
}

/// User representation
struct User {
    id: i32,
    kind: UserKind,
    name: String,
}

#[juniper::graphql_object(Context = Context)]
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
            return vec![
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
            ];
        } else if self.id == 2 {
            return vec![User {
                id: 21,
                kind: UserKind::User,
                name: "user21".into(),
            }];
        } else if self.id == 3 {
            return vec![
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
            ];
        } else {
            return vec![];
        }
    }
}

struct Query;

#[juniper::graphql_object(Context = Context)]
impl Query {
    async fn users(id: i32) -> Vec<User> {
        vec![User {
            id,
            kind: UserKind::Admin,
            name: "user1".into(),
        }]
    }
}

type TypeAlias = Pin<Box<dyn Stream<Item = Result<User, FieldError>> + Send>>;

struct Subscription;

#[juniper::graphql_subscription(Context = Context)]
impl Subscription {
    async fn users() -> TypeAlias {
        let mut counter = 0;
        let stream = tokio::time::interval(Duration::from_secs(5)).map(move |_| {
            counter += 1;
            if counter == 2 {
                Err(FieldError::new(
                    "some field error from handler",
                    Value::Scalar(DefaultScalarValue::String(
                        "some additional string".to_string(),
                    )),
                ))
            } else {
                Ok(User {
                    id: counter,
                    kind: UserKind::Admin,
                    name: "stream user".to_string(),
                })
            }
        });
        Box::pin(stream)
    }
}

type Schema = RootNode<'static, Query, EmptyMutation<Context>, Subscription>;

fn schema() -> Schema {
    Schema::new(Query, EmptyMutation::new(), Subscription)
}

#[tokio::main]
async fn main() {
    ::std::env::set_var("RUST_LOG", "warp_subscriptions");
    env_logger::init();

    let log = warp::log("warp_server");

    let homepage = warp::path::end().map(|| {
        Response::builder()
            .header("content-type", "text/html")
            .body(format!(
                "<html><h1>juniper_subscriptions demo</h1><div>visit <a href=\"/playground\">graphql playground</a></html>"
            ))
    });

    let state = warp::any().map(move || Context {});
    let qm_schema = schema();

    let state2 = warp::any().map(move || Context {});
    let qm_graphql_filter = juniper_warp::make_graphql_filter_async(qm_schema, state.boxed());

    let coordinator = Arc::new(juniper_subscriptions::Coordinator::new(schema()));

    log::info!("Listening on 127.0.0.1:8080");

    let routes =
        (warp::path("subscriptions")
        .and(warp::ws())
        .and(state2.clone())
        .and(warp::any().map(move || Arc::clone(&coordinator)))
        .map(|ws: warp::ws::Ws, ctx: Context, coordinator: Arc<Coordinator<'static, _, _, _, _, _>>| {
            ws.on_upgrade(|websocket| -> Pin<Box<dyn Future<Output = ()> + Send>> {
                log::info!("ws connected");
                graphql_subscriptions_async(
                    websocket,
                    coordinator,
                    ctx
                ).boxed()
            })
        }))
    .or(warp::post()
        .and(warp::path("graphql"))
        .and(qm_graphql_filter))
    .or(warp::get()
        .and(warp::path("playground"))
        .and(playground_filter("/graphql", Some("/subscriptions"))))
    .or(homepage)
    .with(log);

    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}
