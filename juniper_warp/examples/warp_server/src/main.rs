#![deny(warnings)]

extern crate env_logger;
#[macro_use]
extern crate log as irrelevant_log;
extern crate juniper;
extern crate juniper_warp;
extern crate warp;

use juniper::tests::model::Database;
use juniper::{EmptyMutation, RootNode};
use warp::{http::Response, log, Filter};

type Schema = RootNode<'static, Database, EmptyMutation<Database>>;

fn schema() -> Schema {
    Schema::new(Database::new(), EmptyMutation::<Database>::new())
}

fn main() {
    ::std::env::set_var("RUST_LOG", "warp_server");
    env_logger::init();

    let log = log("warp_server");

    let homepage = warp::path::end().map(|| {
        Response::builder()
            .header("content-type", "text/html")
            .body(format!(
                "<html><h1>juniper_warp</h1><div>visit <a href=\"/graphiql\">/graphiql</a></html>"
            ))
    });

    info!("Listening on 127.0.0.1:8080");

    let state = warp::any().map(move || Database::new());
    let graphql_filter = juniper_warp::make_graphql_filter(schema(), state.boxed());

    warp::serve(
        warp::get2()
            .and(warp::path("graphiql"))
            .and(juniper_warp::graphiql_filter("/graphql"))
            .or(homepage)
            .or(warp::path("graphql").and(graphql_filter))
            .with(log),
    ).run(([127, 0, 0, 1], 8080));
}
