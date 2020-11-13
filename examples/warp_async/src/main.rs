//! This example demonstrates async/await usage with warp.

use juniper::{
    graphql_object, EmptyMutation, EmptySubscription, FieldError, GraphQLEnum, RootNode,
};
use warp::{http::Response, Filter};

#[derive(Clone, Copy, Debug)]
struct Context;
impl juniper::Context for Context {}

#[derive(Clone, Copy, Debug, GraphQLEnum)]
enum UserKind {
    Admin,
    User,
    Guest,
}

#[derive(Clone, Debug)]
struct User {
    id: i32,
    kind: UserKind,
    name: String,
}

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
        vec![]
    }
}

#[derive(Clone, Copy, Debug)]
struct Query;

#[graphql_object(context = Context)]
impl Query {
    async fn users() -> Vec<User> {
        vec![User {
            id: 1,
            kind: UserKind::Admin,
            name: "user1".into(),
        }]
    }

    /// Fetch a URL and return the response body text.
    async fn request(url: String) -> Result<String, FieldError> {
        Ok(reqwest::get(&url).await?.text().await?)
    }
}

type Schema = RootNode<'static, Query, EmptyMutation<Context>, EmptySubscription<Context>>;

fn schema() -> Schema {
    Schema::new(
        Query,
        EmptyMutation::<Context>::new(),
        EmptySubscription::<Context>::new(),
    )
}

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "warp_async");
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

    let state = warp::any().map(|| Context);
    let graphql_filter = juniper_warp::make_graphql_filter(schema(), state.boxed());

    warp::serve(
        warp::get()
            .and(warp::path("graphiql"))
            .and(juniper_warp::graphiql_filter("/graphql", None))
            .or(homepage)
            .or(warp::path("graphql").and(graphql_filter))
            .with(log),
    )
    .run(([127, 0, 0, 1], 8080))
    .await
}
