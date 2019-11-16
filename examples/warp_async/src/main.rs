//!
//! This example demonstrates async/await usage with warp.
//! NOTE: this uses tokio 0.1 , not the alpha tokio 0.2.

use juniper::{EmptyMutation, RootNode, FieldError};
use warp::{http::Response, Filter};

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
        vec![]
    }
}

struct Query;

#[juniper::graphql_object(Context = Context)]
impl Query {
    async fn users() -> Vec<User> {
        vec![
            User{
                id: 1,
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

type Schema = RootNode<'static, Query, EmptyMutation<Context>>;

fn schema() -> Schema {
    Schema::new(Query, EmptyMutation::<Context>::new())
}

fn main() {
    ::std::env::set_var("RUST_LOG", "warp_async");
    env_logger::init();

    let log = warp::log("warp_server");

    let homepage = warp::path::end().map(|| {
        Response::builder()
            .header("content-type", "text/html")
            .body(format!(
                "<html><h1>juniper_warp</h1><div>visit <a href=\"/graphiql\">/graphiql</a></html>"
            ))
    });

    log::info!("Listening on 127.0.0.1:8080");

    let state = warp::any().map(move || Context{} );
    let graphql_filter = juniper_warp::make_graphql_filter_async(schema(), state.boxed());

    warp::serve(
        warp::get2()
            .and(warp::path("graphiql"))
            .and(juniper_warp::graphiql_filter("/graphql"))
            .or(homepage)
            .or(warp::path("graphql").and(graphql_filter))
            .with(log),
    )
    .run(([127, 0, 0, 1], 8080));
}
