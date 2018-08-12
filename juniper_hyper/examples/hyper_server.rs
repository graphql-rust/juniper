extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate juniper;
extern crate juniper_hyper;
extern crate pretty_env_logger;

use futures::future;
use futures_cpupool::CpuPool;
use hyper::rt::{self, Future};
use hyper::service::{service_fn, Service};
use hyper::{Body, Request, Response, Server, StatusCode};
use juniper::tests::model::Database;
use juniper::EmptyMutation;
use juniper_hyper::GraphQLHandler;

use hyper::Method;
use std::error::Error as StdError;
use std::sync::Arc;

fn context_factory(_: &mut Request<Body>) -> Result<Database, Box<StdError + Send + Sync>> {
    Ok(Database::new())
}

static PHRASE: &'static [u8] = b"Hello World!";

fn main() {
    pretty_env_logger::init();

    let addr = ([127, 0, 0, 1], 3000).into();

    let pool = CpuPool::new(4);
    let db = Arc::new(Database::new());

    //    let new_service = move || {
    //        let pool = pool.clone();
    //        let db = db.clone();
    //        service_fn(move |req| match (req.method(), req.uri().path()) {
    //            (&Method::GET, "/hello") => {
    //                let mut response = Response::new(Body::empty());
    //                *response.body_mut() = Body::from("Try POSTing data to /graphql");
    //                Box::new(future::ok(response))
    //            }
    //            (&Method::POST, "/graphql") => {
    //                let mut service = GraphQLHandler::new(
    //                    pool.clone(),
    //                    context_factory,
    //                    db.clone(),
    //                    EmptyMutation::<Database>::new(),
    //                );
    //                service.call(req)
    //            }
    //            _ => {
    //                let mut response = Response::new(Body::empty());
    //                *response.status_mut() = StatusCode::NOT_FOUND;
    //                Box::new(future::ok(response))
    //            }
    //        })
    //    };
    let new_service =
        move || Ok(MyService { pool: pool.clone() }) as Result<MyService, Box<hyper::Error>>;
    let server = Server::bind(&addr)
        .serve(new_service)
        .map_err(|e| eprintln!("server error: {}", e));
    println!("Listening on http://{}", addr);

    rt::run(server);
}
