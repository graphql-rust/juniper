//!
//! This example demonstrates async/await usage with warp.
//! NOTE: this uses tokio 0.1 , not the alpha tokio 0.2.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::{future, FutureExt, Poll, StreamExt};
use futures::future::{Future, PollFn};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::timer::Interval;
use warp::{Filter, http::Response, Stream};

use juniper::{EmptyMutation, FieldError, RootNode};
use juniper::http::GraphQLRequest;
use juniper_warp::playground_filter;

#[derive(Clone)]
struct Context {
}

impl juniper::Context for Context {}

#[derive(juniper::GraphQLEnum, Clone, Copy)]
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

#[juniper::object(Context = Context)]
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
        }
        else if self.id == 2 {
            return vec![
                User {
                    id: 21,
                    kind: UserKind::User,
                    name: "user21".into(),
                },
            ];
        }
        else if self.id == 3 {
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
        }
        else {
            return vec![]
        }
    }
}

struct Query;

#[juniper::object(Context = Context)]
impl Query {
    async fn users(id: i32) -> Vec<User> {
        vec![
            User{
                id: id,
                kind: UserKind::Admin,
                name: "user1".into(),
            },
        ]
    }

    /// Fetch a URL and return the response body text.
    async fn request(url: String) -> Result<String, FieldError> {
        use futures::{ compat::{Stream01CompatExt, Future01CompatExt}, stream::TryStreamExt};

        let res = reqwest::r#async::Client::new()
            .get(&url)
            .send()
            .compat()
            .await?;

        let body_raw = res.into_body().compat().try_concat().await?;
        let body = std::str::from_utf8(&body_raw).unwrap_or("invalid utf8");
        Ok(body.to_string())
    }
}

struct MySubscription;

#[juniper::subscription(
    Context = Context
)]
impl MySubscription {
    async fn users() -> User {

        let mut counter = 0;

        let stream = Interval::new_interval(Duration::from_secs(1))
            .map(move|_| {
                counter += 1;
                User {
                    id: counter,
                    kind: UserKind::Admin,
                    name: "stream user".to_string()
                }
            });

        Ok(Box::pin(stream))

//        Ok(stream)
    }
}

type Schema = RootNode<'static, Query, EmptyMutation<Context>, MySubscription>;

fn schema() -> Schema {
    Schema::new(Query, EmptyMutation::<Context>::new(), MySubscription)
}

#[tokio::main]
async fn main() {
    ::std::env::set_var("RUST_LOG", "warp_async");
    env_logger::init();

    let log = warp::log("warp_server");



    // TODO: get original example back, move this example to separate folder
    // create new graphql schema1
    // init new playground
    //                              keep and manage ws connections
    // create new ws connection
    // send stuff over ws connection


    let homepage = warp::path::end().map(|| {
        Response::builder()
            .header("content-type", "text/html")
            .body(format!(
                "<html><h1>juniper_subscriptions demo</h1><div>visit <a href=\"/playground\">graphql playground</a></html>"
            ))
    });

    let state = warp::any().map(move || Context{} );
    let qm_schema = schema();

    let state2 = warp::any().map(move || Context{} );
    let s_schema = Arc::new(schema());
    let qm_graphql_filter = juniper_warp::make_graphql_filter_async(qm_schema, state.boxed());

    println!("Listening on 127.0.0.1:8080");

    let routes =
        (warp::path("subscriptions")
            .and(warp::ws2())
            .and(state2.clone())
            .and(warp::any().map(move || Arc::clone(&s_schema)))
            .map(|ws: warp::ws::Ws2, ctx: Context, schema: Arc<Schema>| {
                ws.on_upgrade(|websocket| -> Pin<Box<Future<Output = ()> + Send>> {
                    println!("ws connected");
                    juniper_warp::graphql_subscriptions_async(
                        websocket,
                        schema,
                        ctx
                    )
                        .boxed()
                })
            })
        )
        .or(warp::post2()
            .and(warp::path("graphql"))
            .and(qm_graphql_filter))
        .or(warp::get2()
            .and(warp::path("playground"))
            .and(playground_filter("/graphql", "/subscriptions")))
        .or(homepage);

    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}

