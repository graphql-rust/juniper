extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate juniper;
extern crate juniper_hyper;
extern crate pretty_env_logger;

use futures::future;
use futures_cpupool::Builder as CpuPoolBuilder;
use hyper::rt::{self, Future};
use hyper::service::service_fn;
use hyper::Method;
use hyper::{Body, Response, Server, StatusCode};
use juniper::tests::model::Database;
use juniper::EmptyMutation;
use juniper::RootNode;
use std::sync::Arc;

fn main() {
    pretty_env_logger::init();

    let addr = ([127, 0, 0, 1], 3000).into();

    let pool = CpuPoolBuilder::new().create();
    let db = Arc::new(Database::new());
    let root_node = Arc::new(RootNode::new(db.clone(), EmptyMutation::<Database>::new()));

    let new_service = move || {
        let pool = pool.clone();
        let root_node = root_node.clone();
        let ctx = db.clone();
        service_fn(move |req| {
            let pool = pool.clone();
            let root_node = root_node.clone();
            let ctx = ctx.clone();
            match (req.method(), req.uri().path()) {
                (&Method::GET, "/") => juniper_hyper::graphiql("/graphql"),
                (&Method::GET, "/graphql") => juniper_hyper::graphql(pool, root_node, ctx, req),
                (&Method::POST, "/graphql") => juniper_hyper::graphql(pool, root_node, ctx, req),
                _ => {
                    let mut response = Response::new(Body::empty());
                    *response.status_mut() = StatusCode::NOT_FOUND;
                    Box::new(future::ok(response))
                }
            }
        })
    };
    let server = Server::bind(&addr)
        .serve(new_service)
        .map_err(|e| eprintln!("server error: {}", e));
    println!("Listening on http://{}", addr);

    rt::run(server);
}
