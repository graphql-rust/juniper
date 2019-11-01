//!
//! This example demonstrates async/await usage with warp.
//! NOTE: this uses tokio 0.2.alpha
//!
//!
//! # Error variable is not set when trying to resolve subscription
//!
//! GraphQL Playground does not send variables with subscriptions for some reason
//! The following js code can be used to test variables with subscriptions:
//! ```js
//!  // Create WebSocket connection.
//!  let socket = new WebSocket('ws://localhost:8080/subscriptions');
//!
//!  // Send query once connection was opened
//!  socket.addEventListener('open', function (event) {
//!      let query = '{"id":"1","type":"start","payload":{"variables":{"withFriends": true},"extensions":{},"operationName":null,"query":"subscription {  users {    id    name    friends {      id      name    }  }}"}}';
//!      socket.send(query);
//!  });
//!
//!  // Print message that connection was closed
//!  socket.addEventListener('close', function (event) {
//!      console.log('================================================');
//!      console.log('============= CLOSED CONNECTION ================');
//!      console.log('================================================');
//!
//!  });
//!
//!  // Print every message from server
//!  socket.addEventListener('message', function (event) {
//!      console.log('Message from server ', event.data);
//!  });
//!
//!  // Paste this separatly to stop subscription execution
//!  socket.send('{"id":"1","type":"stop"}')
//! ```
//!

use std::{
    time::Duration,
    pin::Pin, sync::Arc,
};

use futures::{
    Future, FutureExt,
};
use tokio::timer::Interval;
use warp::{Filter, http::Response};

use juniper::{
    FieldError,
    RootNode,
    EmptyMutation
};
use juniper_warp::playground_filter;

#[derive(Clone)]
struct Context {}

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

//struct EmptyMutation {}

//#[juniper::object(Context = Context)]
//impl EmptyMutation {}

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

#[juniper::object(Context = Context)]
impl Query {
    async fn users(id: i32) -> Vec<User> {
        vec![User {
            id: id,
            kind: UserKind::Admin,
            name: "user1".into(),
        }]
    }

    /// Fetch a URL and return the response body text.
    async fn request(url: String) -> Result<String, FieldError> {
        use futures::{
            compat::{Future01CompatExt, Stream01CompatExt},
            stream::TryStreamExt,
        };

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

        let stream = Interval::new_interval(Duration::from_secs(8)).map(move |_| {
            counter += 1;
            User {
                id: counter,
                kind: UserKind::Admin,
                name: "stream user".to_string(),
            }
        });

        Ok(Box::pin(stream))

        //        Ok(stream)
    }
}

type Schema = RootNode<'static, Query, EmptyMutation<Context>, juniper::EmptySubscription<Context>>;

fn schema() -> Schema {
    Schema::new(Query, EmptyMutation::new(), juniper::EmptySubscription::new())
}

#[tokio::main]
async fn main() {
    ::std::env::set_var("RUST_LOG", "warp_async");
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
    let s_schema = Arc::new(schema());
    let qm_graphql_filter = juniper_warp::make_graphql_filter_async(qm_schema, state.boxed());

    println!("Listening on 127.0.0.1:8080");

    let routes = (warp::path("subscriptions")
        .and(warp::ws2())
        .and(state2.clone())
        .and(warp::any().map(move || Arc::clone(&s_schema)))
        .map(|ws: warp::ws::Ws2, ctx: Context, schema: Arc<Schema>| {
            ws.on_upgrade(|websocket| -> Pin<Box<dyn Future<Output = ()> + Send>> {
                println!("ws connected");
                juniper_warp::graphql_subscriptions_async(websocket, schema, ctx).boxed()
            })
        }))
    .or(warp::post2()
        .and(warp::path("graphql"))
        .and(qm_graphql_filter))
    .or(warp::get2()
        .and(warp::path("playground"))
        .and(playground_filter("/graphql", "/subscriptions")))
    .or(homepage)
    .with(log);

    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}
